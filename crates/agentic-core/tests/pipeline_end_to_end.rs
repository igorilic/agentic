mod common;

use std::collections::HashMap;

use agentic_core::{
    BackendId, Db, Event, EventBus, EventEnvelope, ModelId, Paths, Pipeline, PipelineConfig,
    PipelineOrchestrator, PipelineSm, ProfileId, Run, RunRepo, RunStatus, SmInput, Step, StepRepo,
    StepStatus, TicketKind, TicketRef,
};
use rusqlite::params;

use common::passthrough_gate;

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        params![id],
    )
    .unwrap();
}

fn seed_run_and_steps(
    runs: &RunRepo,
    steps: &StepRepo,
    pipeline: &Pipeline,
    run_id: &str,
) -> HashMap<String, String> {
    runs.insert(Run {
        id: run_id.to_string(),
        workspace_id: "ws1".to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Pending,
        ticket_type: None,
        ticket_ref: None,
        ticket_title: None,
        ticket_body: None,
        backend: "claude-code".to_string(),
        model: "claude-opus-4-7".to_string(),
        started_at: 100,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    })
    .unwrap();

    let mut agent_to_step_id = HashMap::new();
    for (seq, ps) in pipeline.steps.iter().enumerate() {
        let step_id = format!("{run_id}-step-{seq}-{}", ps.agent);
        steps
            .insert(Step {
                id: step_id.clone(),
                run_id: run_id.to_string(),
                seq: seq as i64,
                agent_name: ps.agent.clone(),
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                duration_ms: None,
                token_usage: None,
                cost_usd: None,
                summary: None,
                retry_count: 0,
            })
            .unwrap();
        agent_to_step_id.insert(ps.agent.clone(), step_id);
    }
    agent_to_step_id
}

#[tokio::test]
async fn happy_path_run_drives_sm_events_through_orchestrator_to_completed_state() {
    // 1. Infrastructure
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let runs = RunRepo::new(&db);
    let steps = StepRepo::new(&db);
    let bus = EventBus::new();

    // 2. Seed workspace + run + 4 step rows
    seed_workspace(&db, "ws1");
    let config = PipelineConfig::builtin_default();
    let pipeline = config.default_pipeline().clone();
    let agent_to_step_id = seed_run_and_steps(&runs, &steps, &pipeline, "run1");

    // 3. Spawn orchestrator (subscribes before RunStarted is published).
    //    The SM's Start input emits RunStarted first; the orchestrator handles
    //    it and transitions the run Pending → Running automatically.
    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );

    // 5. Construct SM with the same pipeline
    let mut sm = PipelineSm::new("run1".to_string(), pipeline.clone());

    // 6. Drive: Start + 4 × StepPassed
    let inputs: Vec<SmInput> = std::iter::once(SmInput::Start {
        ticket: TicketRef {
            kind: TicketKind::GithubIssue,
            reference: "#1".to_string(),
            title: Some("test".to_string()),
        },
        profile: ProfileId("github".to_string()),
        backend: BackendId("claude-code".to_string()),
        model: ModelId("claude-opus-4-7".to_string()),
    })
    .chain(std::iter::repeat_n(SmInput::StepPassed, 4))
    .collect();

    for input in inputs {
        // Capture step-agent BEFORE handle for StepComplete's step_id resolution.
        let prior_agent = sm
            .current_step_index()
            .map(|i| pipeline.steps[i].agent.clone());
        let events = sm.handle(input).expect("sm.handle");
        for event in events {
            let step_id = match &event {
                Event::StepStarted { agent, .. } => agent_to_step_id.get(agent).cloned(),
                Event::StepComplete { .. } => prior_agent
                    .as_ref()
                    .and_then(|a| agent_to_step_id.get(a).cloned()),
                _ => None,
            };
            bus.publish(EventEnvelope::now("run1".to_string(), step_id, event));
        }
    }

    // 7. Close the bus and wait for orchestrator to drain.
    drop(bus);
    handle.await.expect("orchestrator joins");

    // 8. Assertions — final DB state.
    let run = runs.get("run1").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed, "run should be Completed");
    assert!(
        run.completed_at.is_some(),
        "run should have completed_at; got {:?}",
        run.completed_at
    );
    assert!(
        run.duration_ms.is_some(),
        "run should have duration_ms; got {:?}",
        run.duration_ms
    );

    for agent in ["architect", "tdd-developer", "qa", "reviewer"] {
        let step_id = agent_to_step_id.get(agent).expect("agent present in map");
        let step = steps.get(step_id).unwrap().unwrap();
        assert_eq!(
            step.status,
            StepStatus::Passed,
            "step {agent} should be Passed; got {:?}",
            step.status
        );
        assert!(
            step.completed_at.is_some(),
            "step {agent} should have completed_at; got {:?}",
            step.completed_at
        );
        assert!(
            step.duration_ms.is_some(),
            "step {agent} should have duration_ms; got {:?}",
            step.duration_ms
        );
    }
}
