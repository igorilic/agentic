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
    let run_id = start_scripted_run(
        app_handle2,
        state2,
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
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::scripted::start_scripted_run
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    // Path doesn't exist — canonicalize will fail.
    let result = start_scripted_run(
        app_handle,
        state,
        "/nonexistent/script.json".to_string(),
        Some(0),
    )
    .await;

    assert!(result.is_err(), "expected Err for missing path");
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_returns_parse_error_on_bad_json() {
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::scripted::start_scripted_run
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

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
    let result = start_scripted_run(
        app_handle,
        state,
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
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::scripted::start_scripted_run
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    // /etc/passwd exists on macOS/Linux and is outside cwd and app_data_dir.
    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    let result = start_scripted_run(app_handle, state, "/etc/passwd".to_string(), Some(0)).await;

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
        script.path().to_string_lossy().into_owned(),
        Some(0),
    )
    .await
    .expect("start_scripted_run");

    // Wait for the publisher loop + finding insert to flush.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let state = app.state::<FindingsState>();
    let list = state.repo.list_by_run(&run_id).expect("list_by_run");
    assert_eq!(list.len(), 1, "expected one persisted finding, got {}", list.len());
    assert_eq!(list[0].id, "f1");
    assert_eq!(list[0].run_id, run_id);
    assert_eq!(list[0].severity, "warning");
    assert_eq!(list[0].message, "missing-error-handling");
    assert_eq!(list[0].file_path.as_deref(), Some("src/main.rs"));
    assert_eq!(list[0].line, Some(42));
    assert!(list[0].triage.is_none(), "fresh findings start untriaged");
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
