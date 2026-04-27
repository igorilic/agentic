#![cfg(test)]

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope};
use agentic_tauri::commands::events::EventBusState;
use agentic_tauri::commands::mention::{mention_agent, MENTION_EVENT_CHANNEL};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager, WebviewWindowBuilder};

/// Channel name for mention events (dedicated channel, not the cockpit channel).
const MENTION_CHANNEL: &str = "agentic://mention-event";

fn build_app() -> tauri::App<tauri::test::MockRuntime> {
    let bus = Arc::new(EventBus::new());
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::mention::mention_agent,
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

#[tokio::test(flavor = "multi_thread")]
async fn mention_agent_returns_run_id_and_publishes_to_dedicated_channel() {
    let app = build_app();
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    // Listen on the DEDICATED mention channel (not the cockpit channel).
    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(MENTION_CHANNEL, move |event| {
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(event.payload()) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();

    let result = mention_agent(
        app_handle,
        state,
        "architect".to_string(),
        "design the system".to_string(),
    )
    .await
    .expect("mention_agent should succeed");

    assert!(!result.run_id.is_empty(), "run_id should be non-empty");
    assert_eq!(result.agent, "architect");

    // Wait for the stub events to be emitted.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let received = captured.lock().unwrap();
    assert!(
        received.len() >= 2,
        "expected at least 2 envelopes on mention channel, got {}",
        received.len()
    );

    // All events carry the same run_id as the result.
    for env in received.iter() {
        assert_eq!(
            env.run_id, result.run_id,
            "envelope run_id should match result"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn mention_agent_rejects_empty_agent() {
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::mention::mention_agent,
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let result = mention_agent(
        app.handle().clone(),
        app.state::<EventBusState>(),
        "".to_string(),
        "some body".to_string(),
    )
    .await;

    assert!(result.is_err(), "expected Err for empty agent");
    let err = result.unwrap_err();
    assert!(
        err.contains("agent"),
        "error should mention agent: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn mention_agent_rejects_empty_body() {
    let bus = Arc::new(EventBus::new());
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::mention::mention_agent,
        ])
        .manage(EventBusState::new(bus))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let result = mention_agent(
        app.handle().clone(),
        app.state::<EventBusState>(),
        "architect".to_string(),
        "".to_string(),
    )
    .await;

    assert!(result.is_err(), "expected Err for empty body");
    let err = result.unwrap_err();
    assert!(
        err.contains("body"),
        "error should mention body: {err}"
    );
}
