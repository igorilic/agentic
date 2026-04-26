#![cfg(test)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope};
use agentic_tauri::commands::events::{EVENT_CHANNEL, EventBusState, subscribe_events};
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::{Listener, Manager, WebviewWindowBuilder};

fn make_test_envelope() -> EventEnvelope {
    EventEnvelope::now(
        "test-run".to_string(),
        Some("test-step".to_string()),
        Event::TextDelta {
            content: "hello from bus".to_string(),
        },
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_events_forwards_envelope_to_webview() {
    let bus = Arc::new(EventBus::new());
    let bus_for_publish = bus.clone();

    // Build mock app with managed state.
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::events::subscribe_events
        ])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    // Create a webview window in the test runtime.
    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    // Capture emitted events.
    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(EVENT_CHANNEL, move |event| {
        let payload = event.payload();
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(payload) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    // Invoke subscribe_events (it spawns a background task internally).
    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    subscribe_events(app_handle.clone(), state)
        .await
        .expect("subscribe_events");

    // Give the spawned subscriber a moment to register.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Publish on the bus.
    bus_for_publish.publish(make_test_envelope());

    // Wait up to 1s for delivery.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    loop {
        if !captured.lock().unwrap().is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!("timeout: no envelope received within 1s");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let received = captured.lock().unwrap();
    assert_eq!(received.len(), 1);
    match &received[0].event {
        Event::TextDelta { content } => assert_eq!(content, "hello from bus"),
        other => panic!("expected TextDelta, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_events_handles_no_subscribers_gracefully() {
    let bus = Arc::new(EventBus::new());

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::events::subscribe_events
        ])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    // Don't create a window or listener — just spawn the subscriber and let it
    // sit. Publish an envelope; the emit will silently no-op (no listeners).
    let app_handle = app.handle().clone();
    let state = app.state::<EventBusState>();
    subscribe_events(app_handle, state)
        .await
        .expect("subscribe_events");

    tokio::time::sleep(Duration::from_millis(50)).await;
    bus.publish(make_test_envelope());

    // Just assert: no panic, command returned Ok.
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribe_events_re_invocation_replaces_forwarder() {
    let bus = Arc::new(EventBus::new());
    let bus_for_publish = bus.clone();

    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::events::subscribe_events
        ])
        .manage(EventBusState::new(bus.clone()))
        .build(mock_context(noop_assets()))
        .expect("build mock app");

    let window = WebviewWindowBuilder::new(&app, "main", tauri::WebviewUrl::default())
        .build()
        .expect("build window");

    // Capture emitted events.
    let captured: Arc<Mutex<Vec<EventEnvelope>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = captured.clone();
    window.listen(EVENT_CHANNEL, move |event| {
        let payload = event.payload();
        if let Ok(env) = serde_json::from_str::<EventEnvelope>(payload) {
            captured_clone.lock().unwrap().push(env);
        }
    });

    let app_handle = app.handle().clone();

    // First subscription.
    let state = app.state::<EventBusState>();
    subscribe_events(app_handle.clone(), state)
        .await
        .expect("first subscribe");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Second subscription — should abort the first and replace it.
    let state2 = app.state::<EventBusState>();
    subscribe_events(app_handle.clone(), state2)
        .await
        .expect("second subscribe");
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Publish ONE envelope.
    bus_for_publish.publish(make_test_envelope());

    // Wait for delivery.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Assert: exactly ONE envelope received, not two.
    let received = captured.lock().unwrap();
    assert_eq!(
        received.len(),
        1,
        "expected exactly 1 envelope after re-subscribe (de-dupe), got {}",
        received.len()
    );
}
