#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope};
use agentic_tauri::commands::events::{EventBusState, get_event_history};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

fn make_envelope(run_id: &str, event_id: &str) -> EventEnvelope {
    EventEnvelope {
        schema_version: 1,
        event_id: event_id.to_string(),
        run_id: run_id.to_string(),
        step_id: None,
        timestamp_ms: 0,
        event: Event::TextDelta {
            content: "test".to_string(),
        },
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn get_event_history_returns_buffered_envelopes() {
    let bus = Arc::new(EventBus::new());
    let bus_for_publish = bus.clone();

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![get_event_history])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    // Publish 3 envelopes for "run1" BEFORE invoking get_event_history.
    bus_for_publish.publish(make_envelope("run1", "e1"));
    bus_for_publish.publish(make_envelope("run1", "e2"));
    bus_for_publish.publish(make_envelope("run1", "e3"));

    // Give the buffer's subscriber time to record.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let state = app.state::<EventBusState>();
    let history = get_event_history(state, "run1".to_string())
        .await
        .unwrap();

    assert_eq!(history.len(), 3);
    assert_eq!(history[0].event_id, "e1");
    assert_eq!(history[1].event_id, "e2");
    assert_eq!(history[2].event_id, "e3");
}

#[tokio::test(flavor = "multi_thread")]
async fn get_event_history_returns_empty_for_unknown_run() {
    let bus = Arc::new(EventBus::new());

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![get_event_history])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let state = app.state::<EventBusState>();
    let history = get_event_history(state, "never-existed".to_string())
        .await
        .unwrap();

    assert!(history.is_empty());
}
