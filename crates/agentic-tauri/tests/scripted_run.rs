#![cfg(test)]

use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope, RunStatus, Severity, StepStatus};
use agentic_core::{Db, ModelId};
use agentic_tauri::commands::events::{EVENT_CHANNEL, EventBusState, subscribe_events};
use agentic_tauri::commands::findings::FindingsState;
use agentic_tauri::commands::scripted::{cancel_run, start_scripted_run};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager, WebviewWindowBuilder};

/// Write a script JSON file under cwd so path validation passes.
fn write_script_under_cwd(events: &[Event]) -> tempfile::NamedTempFile {
    let cwd = std::env::current_dir().unwrap();
    let mut file = tempfile::Builder::new()
        .prefix("scripted-run-test-")
        .suffix(".json")
        .tempfile_in(&cwd)
        .expect("tempfile under cwd");
    let json = serde_json::to_string(events).unwrap();
    file.write_all(json.as_bytes()).unwrap();
    file
}

// ─── helper: build a standard mock app with both commands registered ─────────
fn build_app() -> tauri::App<tauri::test::MockRuntime> {
    build_app_with_db().0
}

/// Build the same mock app but also expose the Db so tests can read the
/// findings table directly. Workspace `default` is seeded so finding inserts
/// can FK against it via the run row scripted_run creates at start.
fn build_app_with_db() -> (tauri::App<tauri::test::MockRuntime>, Db) {
    let bus = Arc::new(EventBus::new());
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    {
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
             VALUES ('default', 'test', '/tmp/test', 'github', 100, 100)",
            [],
        )
        .unwrap();
    }
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::events::subscribe_events,
            agentic_tauri::commands::scripted::start_scripted_run,
            agentic_tauri::commands::scripted::cancel_run,
        ])
        .manage(EventBusState::new(bus))
        .manage(FindingsState::new(&db))
        .manage(db.clone())
        .build(mock_context(noop_assets()))
        .expect("build mock app");
    (app, db)
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_publishes_events_in_order() {
    let app = build_app();
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    // Capture envelopes forwarded by subscribe_events.
    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(EVENT_CHANNEL, move |event| {
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(event.payload()) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    // Subscribe BEFORE start.
    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    subscribe_events(app_handle.clone(), state)
        .await
        .expect("subscribe");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Build a 3-event script — under cwd so path validator accepts it.
    let events = vec![
        Event::StepStarted {
            agent: "scripted".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::TextDelta {
            content: "hello".to_string(),
        },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: agentic_core::TokenUsage::default(),
            cost_usd: None,
            duration_ms: 10,
        },
    ];
    let script = write_script_under_cwd(&events);

    let app_handle2 = app.handle().clone();
    let state2 = app.state::<EventBusState>();
    let db_state2 = app.state::<Db>();
    let run_id = start_scripted_run(
        app_handle2,
        state2,
        db_state2,
        script.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await
    .expect("start_scripted_run");

    assert!(!run_id.is_empty(), "run_id should be non-empty");

    // Wait for forwarding to drain — 3 script events + 1 RunComplete.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let received = captured.lock().unwrap();
    // We now expect 4 envelopes: 3 script events + 1 synthetic RunComplete.
    assert!(
        received.len() >= 3,
        "expected at least 3 envelopes, got {}",
        received.len()
    );

    // Order check for the first 3.
    assert!(matches!(received[0].event, Event::StepStarted { .. }));
    assert!(matches!(received[1].event, Event::TextDelta { .. }));
    assert!(matches!(received[2].event, Event::StepComplete { .. }));

    // run_id propagated correctly.
    assert_eq!(received[0].run_id, run_id);
    assert_eq!(received[1].run_id, run_id);
    assert_eq!(received[2].run_id, run_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_returns_io_error_on_missing_file() {
    let (app, _db) = build_app_with_db();

    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    let db_state = app.state::<Db>();
    // Path doesn't exist — canonicalize will fail.
    let result = start_scripted_run(
        app_handle,
        state,
        db_state,
        "/nonexistent/script.json".to_string(),
        Some(0),
    )
    .await;

    assert!(result.is_err(), "expected Err for missing path");
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_returns_parse_error_on_bad_json() {
    let (app, _db) = build_app_with_db();

    // Write bad JSON under cwd so path validator accepts it.
    let cwd = std::env::current_dir().unwrap();
    let mut file = tempfile::Builder::new()
        .prefix("bad-json-")
        .suffix(".json")
        .tempfile_in(&cwd)
        .unwrap();
    file.write_all(b"this is not json").unwrap();

    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    let db_state = app.state::<Db>();
    let result = start_scripted_run(
        app_handle,
        state,
        db_state,
        file.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("parse") || err.to_lowercase().contains("json"),
        "error should mention parse: {err}"
    );
}

// ─── NEW TEST 1 — F1: path outside scope is rejected ─────────────────────────
#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_rejects_path_outside_scope() {
    let (app, _db) = build_app_with_db();

    // /etc/passwd exists on macOS/Linux and is outside cwd and app_data_dir.
    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    let db_state = app.state::<Db>();
    let result = start_scripted_run(
        app_handle,
        state,
        db_state,
        "/etc/passwd".to_string(),
        Some(0),
    )
    .await;

    assert!(result.is_err(), "expected Err for path outside scope");
    let err = result.unwrap_err();
    assert!(
        err.contains("outside allowed scope")
            || err.contains("not under any allowed root")
            || err.contains("PathOutsideScope"),
        "error should mention scope rejection: {err}"
    );
}

// ─── NEW TEST 2 — F2: RunComplete envelope is published after all events ──────
#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_publishes_run_complete_after_events() {
    let app = build_app();
    let window = WebviewWindowBuilder::new(&app, "main2", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(EVENT_CHANNEL, move |event| {
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(event.payload()) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    let state = app.state::<EventBusState>();
    subscribe_events(app.handle().clone(), state).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let events = vec![
        Event::TextDelta {
            content: "a".to_string(),
        },
        Event::TextDelta {
            content: "b".to_string(),
        },
    ];
    let script = write_script_under_cwd(&events);

    let run_id = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        script.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await
    .expect("start_scripted_run");

    tokio::time::sleep(Duration::from_millis(300)).await;

    let received = captured.lock().unwrap();
    // Should have 2 TextDelta + 1 RunComplete.
    assert!(
        received.len() >= 3,
        "expected ≥3 envelopes (2 script + RunComplete), got {}",
        received.len()
    );

    let last = &received[received.len() - 1];
    assert_eq!(last.run_id, run_id);
    assert!(
        matches!(
            &last.event,
            Event::RunComplete {
                status: RunStatus::Completed,
                ..
            }
        ),
        "last event should be RunComplete(Completed), got {:?}",
        last.event
    );
}

// ─── NEW TEST 3 — F7: cancel_run aborts an in-flight run ─────────────────────
#[tokio::test(flavor = "multi_thread")]
async fn cancel_run_aborts_in_flight_run() {
    let app = build_app();
    let window = WebviewWindowBuilder::new(&app, "main3", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(EVENT_CHANNEL, move |event| {
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(event.payload()) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    let state = app.state::<EventBusState>();
    subscribe_events(app.handle().clone(), state).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 5 events with 200ms delay each — cancel after ~100ms to interrupt early.
    let events: Vec<Event> = (0..5)
        .map(|i| Event::TextDelta {
            content: format!("chunk-{i}"),
        })
        .collect();
    let script = write_script_under_cwd(&events);

    let run_id = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        script.path().to_string_lossy().into_owned(),
        Some(200), // 200ms per event
    )
    .await
    .expect("start_scripted_run");

    // Let the first event land, then cancel.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let cancel_result = cancel_run(app.state::<EventBusState>(), run_id.clone())
        .await
        .expect("cancel_run");
    assert!(
        cancel_result,
        "cancel_run should return true for known run_id"
    );

    // Wait for the RunComplete(Failed) envelope.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let received = captured.lock().unwrap();

    // (a) Fewer than 5 TextDelta arrived.
    let delta_count = received
        .iter()
        .filter(|e| matches!(e.event, Event::TextDelta { .. }))
        .count();
    assert!(
        delta_count < 5,
        "expected fewer than 5 TextDelta after cancel, got {delta_count}"
    );

    // (c) A RunComplete(Failed) with "cancelled" in summary was published.
    let run_complete = received.iter().find(|e| {
        matches!(
            &e.event,
            Event::RunComplete {
                status: RunStatus::Failed,
                ..
            }
        )
    });
    assert!(
        run_complete.is_some(),
        "expected RunComplete(Failed) after cancel"
    );
    if let Some(env) = run_complete
        && let Event::RunComplete { summary, .. } = &env.event
    {
        assert!(
            summary.contains("cancel"),
            "summary should mention cancellation: {summary}"
        );
    }
}

// ─── NEW TEST — CP-9: findings emitted in a script land in the findings table ─
#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_persists_findings_to_db() {
    let (app, _db) = build_app_with_db();

    let events = vec![
        Event::StepStarted {
            agent: "reviewer".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::Finding {
            finding_id: "f1".to_string(),
            severity: Severity::Warning,
            file: Some(std::path::PathBuf::from("src/main.rs")),
            line: Some(42),
            message: "missing-error-handling".to_string(),
            suggestion: None,
        },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: agentic_core::TokenUsage::default(),
            cost_usd: None,
            duration_ms: 10,
        },
    ];
    let script = write_script_under_cwd(&events);

    let run_id = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        script.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await
    .expect("start_scripted_run");

    // Wait for the publisher loop + finding insert to flush.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let state = app.state::<FindingsState>();
    let list = state.repo.list_by_run(&run_id).expect("list_by_run");
    assert_eq!(
        list.len(),
        1,
        "expected one persisted finding, got {}",
        list.len()
    );
    // The DB row id is scoped with the run_id (`<run_id>:<finding_id>`)
    // so the same script can be replayed without PK collisions. The
    // envelope on the bus still carries the original finding_id.
    assert_eq!(list[0].id, format!("{run_id}:f1"));
    assert_eq!(list[0].run_id, run_id);
    assert_eq!(list[0].severity, "warning");
    assert_eq!(list[0].message, "missing-error-handling");
    assert_eq!(list[0].file_path.as_deref(), Some("src/main.rs"));
    assert_eq!(list[0].line, Some(42));
    assert!(list[0].triage.is_none(), "fresh findings start untriaged");
}

// ─── REGRESSION (#70) — RunComplete must flip runs.status off "running" ──────
#[tokio::test(flavor = "multi_thread")]
async fn run_complete_projects_back_into_runs_status() {
    use agentic_core::db::runs::RunRepo;
    use agentic_core::events::RunStatus as CoreRunStatus;

    let (app, db) = build_app_with_db();

    let events = vec![Event::TextDelta {
        content: "tiny".to_string(),
    }];
    let script = write_script_under_cwd(&events);

    let run_id = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        script.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await
    .expect("start");

    // Wait for the publish loop + RunComplete + project-back to flush.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let runs = RunRepo::new(&db);
    let row = runs
        .get(&run_id)
        .expect("get run row")
        .expect("run row should exist after seeding");
    assert_eq!(
        row.status,
        CoreRunStatus::Completed,
        "happy-path RunComplete must flip runs.status from Running to Completed",
    );
    assert!(
        row.completed_at.is_some(),
        "completed_at should be set when status flips off Running"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_run_projects_failed_status_back_into_runs_table() {
    use agentic_core::db::runs::RunRepo;
    use agentic_core::events::RunStatus as CoreRunStatus;

    let (app, db) = build_app_with_db();

    // 5 events, 200ms apart — slow enough that we can cancel after the first.
    let events: Vec<Event> = (0..5)
        .map(|i| Event::TextDelta {
            content: format!("chunk-{i}"),
        })
        .collect();
    let script = write_script_under_cwd(&events);

    let run_id = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        script.path().to_string_lossy().into_owned(),
        Some(200),
    )
    .await
    .expect("start");

    tokio::time::sleep(Duration::from_millis(100)).await;
    cancel_run(app.state::<EventBusState>(), run_id.clone())
        .await
        .expect("cancel");

    // Wait for the cancel cascade + RunComplete(Failed) + project-back.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let row = RunRepo::new(&db)
        .get(&run_id)
        .expect("get")
        .expect("run row");
    assert_eq!(
        row.status,
        CoreRunStatus::Failed,
        "cancelled run must flip runs.status from Running to Failed",
    );
}

// ─── REGRESSION — running the same script twice persists findings both times ─
#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_persists_findings_across_reruns_with_same_finding_id() {
    // Real bug: the demo script uses literal finding_id "f1"/"f2". Since
    // findings.id is a TEXT PRIMARY KEY and the original implementation
    // stored finding_id as the row id, the second run silently lost its
    // findings to a UNIQUE-constraint failure (logged as a tracing::warn,
    // but tracing wasn't initialised in the Tauri binary so the warning
    // vanished). Three of the user's demo runs ended up with no findings.
    let (app, _db) = build_app_with_db();

    let events = vec![
        Event::StepStarted {
            agent: "reviewer".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::Finding {
            finding_id: "f1".to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "msg-1".to_string(),
            suggestion: None,
        },
        Event::Finding {
            finding_id: "f2".to_string(),
            severity: Severity::Error,
            file: None,
            line: None,
            message: "msg-2".to_string(),
            suggestion: None,
        },
    ];
    let script = write_script_under_cwd(&events);
    let path = script.path().to_string_lossy().into_owned();

    // First run.
    let run_a = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        path.clone(),
        Some(0),
    )
    .await
    .expect("first run");

    // Second run — same script, same literal finding_ids.
    let run_b = start_scripted_run(
        app.handle().clone(),
        app.state::<EventBusState>(),
        app.state::<Db>(),
        path,
        Some(0),
    )
    .await
    .expect("second run");

    assert_ne!(run_a, run_b, "different runs must get different ULIDs");

    // Wait for both spawn loops to drain.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let state = app.state::<FindingsState>();
    let a_findings = state.repo.list_by_run(&run_a).expect("list run_a");
    let b_findings = state.repo.list_by_run(&run_b).expect("list run_b");

    assert_eq!(a_findings.len(), 2, "first run should persist 2 findings");
    assert_eq!(b_findings.len(), 2, "second run should persist 2 findings");
}

// ─── NEW TEST 4 — F7: cancel_run returns false for unknown run_id ─────────────
#[tokio::test(flavor = "multi_thread")]
async fn cancel_run_returns_false_for_unknown_run_id() {
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::scripted::cancel_run
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let result = cancel_run(app.state::<EventBusState>(), "no-such-run-id".to_string())
        .await
        .expect("cancel_run Ok");
    assert!(!result, "cancel_run should return false for unknown run_id");
}
