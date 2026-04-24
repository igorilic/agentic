#![cfg(feature = "testing")]

use std::path::PathBuf;
use std::time::Duration;

use agentic_core::{
    Backend, Db, Event, EventBus, EventEnvelope, EventPersister, ExecuteRequest, ModelId, Paths,
    PipelineOrchestrator, Run, RunId, RunRepo, RunStatus, ScriptedBackend, Step, StepId, StepRepo,
    StepStatus, TokenUsage, WorkspaceRef,
};
use rusqlite::params;
use tokio_util::sync::CancellationToken;

fn init_tracing() {
    // Best-effort — don't panic if another test already set a subscriber.
    let _ = tracing_subscriber::fmt()
        .with_env_filter("warn")
        .with_test_writer()
        .try_init();
}

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'stress', '/tmp/stress', 'test', 100, 100)",
        params![id],
    )
    .unwrap();
}

fn seed_run_running(id: &str, ws_id: &str) -> Run {
    Run {
        id: id.to_string(),
        workspace_id: ws_id.to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Running,
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

/// Build a script that emits many TextDelta envelopes, sandwiched between
/// StepStarted and StepComplete, so the orchestrator also has work to do.
fn stress_script(agent: &str, n_deltas: usize) -> Vec<Event> {
    let mut events = Vec::with_capacity(n_deltas + 2);
    events.push(Event::StepStarted {
        agent: agent.to_string(),
        model: ModelId("fake".to_string()),
    });
    for i in 0..n_deltas {
        events.push(Event::TextDelta {
            content: format!("chunk {i}"),
        });
    }
    events.push(Event::StepComplete {
        status: StepStatus::Passed,
        summary: format!("{agent} done"),
        token_usage: TokenUsage::default(),
        cost_usd: None,
        duration_ms: 100,
    });
    events
}

#[tokio::test]
async fn persister_writes_all_events_under_heavy_volume() {
    init_tracing();

    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let bus = EventBus::new();

    // Seed workspace + run so orchestrator has rows to mutate.
    seed_workspace(&db, "ws-stress");
    let runs = RunRepo::new(&db);
    let steps_repo = StepRepo::new(&db);
    runs.insert(seed_run_running("run-stress", "ws-stress"))
        .unwrap();

    // Spawn orchestrator + persister.
    let orch_handle = PipelineOrchestrator::spawn(bus.clone(), runs.clone(), steps_repo.clone());
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // Seed a step row so orchestrator has something to mark Running -> Passed.
    let step_id = "step-stress-0".to_string();
    steps_repo
        .insert(Step {
            id: step_id.clone(),
            run_id: "run-stress".to_string(),
            seq: 0,
            agent_name: "stress-agent".to_string(),
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

    // Publish N events via a ScriptedBackend to simulate a heavy step.
    const N_DELTAS: usize = 5000;
    let script = stress_script("stress-agent", N_DELTAS);
    let expected_events = script.len(); // StepStarted + N_DELTAS + StepComplete

    let backend = ScriptedBackend::new(script);
    let req = ExecuteRequest {
        workspace: WorkspaceRef {
            id: "ws-stress".to_string(),
            root_path: PathBuf::from("/tmp/stress"),
        },
        run_id: RunId("run-stress".to_string()),
        step_id: StepId(step_id.clone()),
        agent_name: "stress-agent".to_string(),
        agent_prompt: String::new(),
        user_context: String::new(),
        model: None,
        tools: Vec::new(),
        cwd: PathBuf::from("/tmp/stress"),
        timeout: None,
        cancel: CancellationToken::new(),
    };
    backend.execute(req, bus.sender()).await.expect("execute");

    // Publish RunComplete (orchestrator updates runs.status).
    bus.publish(EventEnvelope::now(
        "run-stress".to_string(),
        None,
        Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: 100,
            summary: "stress done".to_string(),
        },
    ));

    // Drain.
    drop(bus);
    // Allow tasks time to drain.
    let _ = tokio::time::timeout(Duration::from_secs(60), orch_handle)
        .await
        .expect("orch join");
    let _ = tokio::time::timeout(Duration::from_secs(60), pers_handle)
        .await
        .expect("pers join");

    // Assert: stream_events row count = expected + 1 (RunComplete).
    let conn = db.conn().unwrap();
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stream_events WHERE run_id = 'run-stress'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let expected_total = expected_events as i64 + 1; // +1 for RunComplete
    assert_eq!(
        total, expected_total,
        "persister dropped events: got {total} in DB, expected {expected_total}"
    );

    // Sanity: run row updated by orchestrator.
    let run = runs.get("run-stress").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    // Sanity: step row updated.
    let step = steps_repo.get(&step_id).unwrap().unwrap();
    assert_eq!(step.status, StepStatus::Passed);
}
