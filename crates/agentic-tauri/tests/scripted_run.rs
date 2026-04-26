#![cfg(test)]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope, StepStatus};
use agentic_core::ModelId;
use agentic_tauri::commands::events::{EVENT_CHANNEL, EventBusState, subscribe_events};
use agentic_tauri::commands::scripted::start_scripted_run;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager, WebviewWindowBuilder};

fn write_script_file(events: &[Event]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().expect("tempfile");
    let json = serde_json::to_string(events).expect("serialize");
    std::io::Write::write_all(&mut file, json.as_bytes()).expect("write");
    file
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_publishes_events_in_order() {
    let bus = Arc::new(EventBus::new());

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::events::subscribe_events,
            agentic_tauri::commands::scripted::start_scripted_run
        ])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

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
    subscribe_events(app_handle.clone(), state).await.expect("subscribe");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Build a 3-event script.
    let events = vec![
        Event::StepStarted {
            agent: "scripted".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::TextDelta { content: "hello".to_string() },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: agentic_core::TokenUsage::default(),
            cost_usd: None,
            duration_ms: 10,
        },
    ];
    let script = write_script_file(&events);

    // Start with delay_ms=0 to make the test fast.
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

    // Wait for forwarding to drain.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let received = captured.lock().unwrap();
    assert_eq!(received.len(), 3, "expected 3 envelopes, got {}", received.len());

    // Order check
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
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    let result = start_scripted_run(
        app_handle,
        state,
        "/nonexistent/script.json".to_string(),
        Some(0),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("/nonexistent/script.json"),
        "error should mention path: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_scripted_run_returns_parse_error_on_bad_json() {
    let bus = Arc::new(EventBus::new());

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::scripted::start_scripted_run
        ])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let mut file = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut file, b"this is not json").unwrap();

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
