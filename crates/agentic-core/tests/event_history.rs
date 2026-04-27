use std::time::Duration;

use agentic_core::events::{Event, EventBus, EventEnvelope, EventHistoryBuffer, DEFAULT_HISTORY_CAP};

fn make_envelope(run_id: &str, event_id: &str, content: &str) -> EventEnvelope {
    EventEnvelope {
        schema_version: 1,
        event_id: event_id.to_string(),
        run_id: run_id.to_string(),
        step_id: None,
        timestamp_ms: 0,
        event: Event::TextDelta {
            content: content.to_string(),
        },
    }
}

#[tokio::test]
async fn buffer_records_envelopes_per_run() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn(&bus, 100);

    bus.publish(make_envelope("run1", "e1", "hello"));
    bus.publish(make_envelope("run1", "e2", "world"));
    bus.publish(make_envelope("run2", "e3", "other"));

    // Give the subscriber a moment to drain.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let run1 = buffer.get("run1").await;
    assert_eq!(run1.len(), 2);
    assert_eq!(run1[0].event_id, "e1");
    assert_eq!(run1[1].event_id, "e2");

    let run2 = buffer.get("run2").await;
    assert_eq!(run2.len(), 1);
    assert_eq!(run2[0].event_id, "e3");
}

#[tokio::test]
async fn buffer_caps_per_run_at_specified_capacity() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn(&bus, 5);

    for i in 0..10 {
        bus.publish(make_envelope("run1", &format!("e{i}"), ""));
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let run1 = buffer.get("run1").await;
    assert_eq!(run1.len(), 5);
    // Most recent 5 retained.
    assert_eq!(run1[0].event_id, "e5");
    assert_eq!(run1[4].event_id, "e9");
}

#[tokio::test]
async fn buffer_returns_empty_for_unknown_run() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn(&bus, 100);
    assert!(buffer.get("never-existed").await.is_empty());
}

#[tokio::test]
async fn buffer_default_cap_is_1000() {
    assert_eq!(DEFAULT_HISTORY_CAP, 1000);
}
