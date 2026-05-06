mod common;

use agentic_core::{
    BackendId, CURRENT_SCHEMA_VERSION, Db, Event, EventBus, EventEnvelope, ModelId, Paths,
    PipelineOrchestrator, ProfileId, Run, RunRepo, RunStatus, Step, StepRepo, StepStatus,
    TicketKind, TicketRef, TokenUsage,
};
use rusqlite::params;

use common::passthrough_gate;

fn setup() -> (tempfile::TempDir, Db, RunRepo, StepRepo, EventBus) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let runs = RunRepo::new(&db);
    let steps = StepRepo::new(&db);
    let bus = EventBus::new();
    seed_workspace(&db, "ws1");
    (tmp, db, runs, steps, bus)
}

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        params![id],
    )
    .unwrap();
}

fn seed_run_running(id: &str, started_at: i64) -> Run {
    Run {
        id: id.to_string(),
        workspace_id: "ws1".to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Running,
        ticket_type: None,
        ticket_ref: None,
        ticket_title: None,
        ticket_body: None,
        backend: "claude-code".to_string(),
        model: "claude-opus-4-7".to_string(),
        started_at,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    }
}

fn seed_run_pending(id: &str, started_at: i64) -> Run {
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
        started_at,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    }
}

fn seed_step(id: &str, run_id: &str, seq: i64, status: StepStatus) -> Step {
    Step {
        id: id.to_string(),
        run_id: run_id.to_string(),
        seq,
        agent_name: "architect".to_string(),
        status,
        started_at: None,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        retry_count: 0,
    }
}

#[tokio::test]
async fn step_started_event_transitions_step_row_to_running() {
    let (_tmp, _db, runs, steps, bus) = setup();
    runs.insert(seed_run_running("run1", 100)).unwrap();
    steps
        .insert(seed_step("step1", "run1", 0, StepStatus::Pending))
        .unwrap();

    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );

    bus.publish(EventEnvelope::now(
        "run1".to_string(),
        Some("step1".to_string()),
        Event::StepStarted {
            agent: "architect".to_string(),
            model: ModelId("claude-opus-4-7".into()),
        },
    ));

    drop(bus);
    handle.await.expect("orchestrator joins");

    let step = steps.get("step1").unwrap().unwrap();
    assert_eq!(step.status, StepStatus::Running);
}

#[tokio::test]
async fn step_complete_sets_status_completed_at_and_duration_ms() {
    let (_tmp, _db, runs, steps, bus) = setup();
    runs.insert(seed_run_running("run1", 100)).unwrap();
    steps
        .insert(seed_step("step1", "run1", 0, StepStatus::Running))
        .unwrap();

    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );

    bus.publish(EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("step1".to_string()),
        timestamp_ms: 500,
        event: Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: Some(0.012),
            duration_ms: 400,
        },
    });

    drop(bus);
    handle.await.expect("orchestrator joins");

    let step = steps.get("step1").unwrap().unwrap();
    assert_eq!(step.status, StepStatus::Passed);
    assert_eq!(step.completed_at, Some(500));
    assert_eq!(step.duration_ms, Some(400));
}

#[tokio::test]
async fn run_complete_transitions_run_row_and_delivers_to_subscribers() {
    let (_tmp, _db, runs, steps, bus) = setup();
    runs.insert(seed_run_running("run1", 100)).unwrap();

    // Second subscriber simulates the UI; must see the event via broadcast semantics.
    let mut ui_rx = bus.subscribe();
    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );

    bus.publish(EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "01J8RZYX1K3PQXGT1WJYR8AZ7Q".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 900,
        event: Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: 800,
            summary: "done".to_string(),
        },
    });

    drop(bus);
    handle.await.expect("orchestrator joins");

    let run = runs.get("run1").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);
    assert_eq!(run.completed_at, Some(900));
    assert_eq!(run.duration_ms, Some(800));

    let received = ui_rx.recv().await.expect("ui subscriber receives");
    assert!(matches!(
        received.event,
        Event::RunComplete {
            status: RunStatus::Completed,
            ..
        }
    ));
}

#[tokio::test]
async fn run_started_event_transitions_run_row_to_running() {
    let (_tmp, _db, runs, steps, bus) = setup();

    // Seed run as Pending (the natural default).
    runs.insert(seed_run_pending("run-rs", 100)).unwrap();

    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );
    bus.publish(EventEnvelope::now(
        "run-rs".to_string(),
        None,
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::GithubIssue,
                reference: "#1".into(),
                title: None,
            },
            profile: ProfileId("github".into()),
            backend: BackendId("claude-code".into()),
            model: ModelId("claude-opus-4-7".into()),
            agents: vec![],
        },
    ));

    drop(bus);
    handle.await.expect("orchestrator joins");

    let run = runs.get("run-rs").unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Running);
}

#[tokio::test]
async fn run_started_for_unknown_run_logs_error_and_continues() {
    let (_tmp, _db, runs, steps, bus) = setup();

    // Do NOT insert any run row for "ghost-run".

    let handle = PipelineOrchestrator::spawn(
        bus.clone(),
        runs.clone(),
        steps.clone(),
        passthrough_gate(&bus),
    );

    // Publish RunStarted for a run_id that doesn't exist.
    bus.publish(EventEnvelope::now(
        "ghost-run".to_string(),
        None,
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::FreeText,
                reference: "test".into(),
                title: None,
            },
            profile: ProfileId("custom".into()),
            backend: BackendId("scripted".into()),
            model: ModelId("fake".into()),
            agents: vec![],
        },
    ));

    // Now publish a valid event for a different run that DOES exist, to verify
    // the orchestrator continues processing after the error.
    runs.insert(seed_run_pending("survivor-run", 100)).unwrap();
    bus.publish(EventEnvelope::now(
        "survivor-run".to_string(),
        None,
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::FreeText,
                reference: "test".into(),
                title: None,
            },
            profile: ProfileId("custom".into()),
            backend: BackendId("scripted".into()),
            model: ModelId("fake".into()),
            agents: vec![],
        },
    ));

    drop(bus);
    handle.await.expect("orchestrator joins");

    // Verify the survivor was transitioned despite the earlier ghost error.
    let survivor = runs.get("survivor-run").unwrap().unwrap();
    assert_eq!(survivor.status, RunStatus::Running);
}
