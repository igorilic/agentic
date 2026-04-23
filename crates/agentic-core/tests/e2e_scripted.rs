#![cfg(feature = "testing")]

use std::collections::HashMap;
use std::path::PathBuf;

use agentic_core::{
    Backend, BackendId, Db, Event, EventBus, EventEnvelope, EventPersister, ExecuteRequest,
    ModelId, Paths, Pipeline, PipelineConfig, PipelineOrchestrator, ProfileId, Run, RunId, RunRepo,
    RunStatus, ScriptedBackend, Step, StepId, StepRepo, StepStatus, TicketKind, TicketRef,
    TokenUsage,
};
use rusqlite::params;
use tokio_util::sync::CancellationToken;

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        params![id],
    )
    .unwrap();
}

fn seed_run_pending(id: &str) -> Run {
    Run {
        id: id.to_string(),
        workspace_id: "ws1".to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Pending,
        ticket_type: None,
        ticket_ref: None,
        ticket_title: None,
        ticket_body: None,
        backend: "scripted".to_string(),
        model: "fake".to_string(),
        started_at: 100,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    }
}

fn seed_steps(steps: &StepRepo, run_id: &str, pipeline: &Pipeline) -> HashMap<String, String> {
    let mut map = HashMap::new();
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
        map.insert(ps.agent.clone(), step_id);
    }
    map
}

fn step_script(agent_name: &str) -> Vec<Event> {
    vec![
        Event::StepStarted {
            agent: agent_name.to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::TextDelta {
            content: format!("{agent_name}: first"),
        },
        Event::TextDelta {
            content: format!("{agent_name}: second"),
        },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: format!("{agent_name} done"),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 100,
        },
    ]
}

#[tokio::test]
async fn four_scripted_backends_complete_full_pipeline_and_persist_all_events() {
    // --- Infrastructure ---
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let runs = RunRepo::new(&db);
    let steps_repo = StepRepo::new(&db);
    let bus = EventBus::new();

    // --- Seed workspace, run, 4 steps ---
    seed_workspace(&db, "ws1");
    let config = PipelineConfig::builtin_default();
    let pipeline = config.default_pipeline().clone();
    runs.insert(seed_run_pending("run1")).unwrap();
    let agent_to_step_id = seed_steps(&steps_repo, "run1", &pipeline);

    // --- Workaround (GH #17): orchestrator doesn't handle RunStarted yet ---
    runs.transition("run1", RunStatus::Running).unwrap();

    // --- Spawn orchestrator + persister ---
    let orch_handle = PipelineOrchestrator::spawn(bus.clone(), runs.clone(), steps_repo.clone());
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // --- Publish RunStarted ---
    bus.publish(EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::GithubIssue,
                reference: "#1".to_string(),
                title: Some("test".to_string()),
            },
            profile: ProfileId("github".to_string()),
            backend: BackendId("scripted".to_string()),
            model: ModelId("fake".to_string()),
        },
    ));

    // --- Run each scripted backend in pipeline order ---
    for ps in pipeline.steps.iter() {
        let step_id = agent_to_step_id
            .get(&ps.agent)
            .expect("agent in map")
            .clone();
        let backend = ScriptedBackend::new(step_script(&ps.agent));
        let req = ExecuteRequest {
            workspace: agentic_core::WorkspaceRef {
                id: "ws1".to_string(),
                root_path: PathBuf::from("/tmp/ws1"),
            },
            run_id: RunId("run1".to_string()),
            step_id: StepId(step_id),
            agent_name: ps.agent.clone(),
            agent_prompt: String::new(),
            user_context: String::new(),
            model: None,
            tools: Vec::new(),
            cwd: PathBuf::from("/tmp/ws1"),
            timeout: None,
            cancel: CancellationToken::new(),
        };
        let outcome = backend.execute(req, bus.sender()).await.expect("execute");
        assert_eq!(outcome.status, StepStatus::Passed, "agent {}", ps.agent);
    }

    // --- Publish RunComplete ---
    bus.publish(EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: 400,
            summary: "pipeline done".to_string(),
        },
    ));

    // --- Drain ---
    drop(bus);
    orch_handle.await.expect("orchestrator joins");
    pers_handle.await.expect("persister joins");

    // --- Assert run transitioned ---
    let run = runs.get("run1").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed, "run should be Completed");
    assert!(run.completed_at.is_some(), "run.completed_at should be set");
    assert_eq!(run.duration_ms, Some(400));

    // --- Assert all 4 steps Passed ---
    for ps in pipeline.steps.iter() {
        let step_id = agent_to_step_id.get(&ps.agent).unwrap();
        let step = steps_repo.get(step_id).unwrap().unwrap();
        assert_eq!(step.status, StepStatus::Passed, "step {}", ps.agent);
        assert!(step.completed_at.is_some(), "{} completed_at", ps.agent);
        assert_eq!(step.duration_ms, Some(100), "{} duration_ms", ps.agent);
    }

    // --- Assert stream_events row count = 18 ---
    let conn = db.conn().unwrap();
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stream_events WHERE run_id = 'run1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        total, 18,
        "expected 18 events: 1 RunStarted + 4 × (StepStarted + 2 TextDelta + StepComplete) + 1 RunComplete"
    );

    // --- Spot-check event_type distribution ---
    let count_by_type = |event_type: &str| -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM stream_events WHERE run_id = 'run1' AND event_type = ?1",
            params![event_type],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(count_by_type("RunStarted"), 1);
    assert_eq!(count_by_type("StepStarted"), 4);
    assert_eq!(count_by_type("TextDelta"), 8);
    assert_eq!(count_by_type("StepComplete"), 4);
    assert_eq!(count_by_type("RunComplete"), 1);
}
