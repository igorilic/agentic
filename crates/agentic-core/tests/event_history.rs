use std::time::Duration;

use agentic_core::events::{
    DEFAULT_HISTORY_CAP, DEFAULT_RUNS_CAP, Event, EventBus, EventEnvelope, EventHistoryBuffer,
};

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

// ── LRU runs-cap tests (GH #64) ─────────────────────────────────────────────

/// DEFAULT_RUNS_CAP must be 32.
#[tokio::test]
async fn default_runs_cap_is_32() {
    assert_eq!(DEFAULT_RUNS_CAP, 32);
}

/// spawn_with_runs_cap exists and respects runs_cap.
/// With runs_cap=3 and 4 distinct run_ids, the oldest run (r1) must be evicted.
#[tokio::test]
async fn bounded_map_evicts_oldest_run() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn_with_runs_cap(&bus, 100, 3);

    bus.publish(make_envelope("r1", "e1", "a"));
    bus.publish(make_envelope("r2", "e2", "b"));
    bus.publish(make_envelope("r3", "e3", "c"));
    bus.publish(make_envelope("r4", "e4", "d")); // r1 is evicted when r4 pushes over runs_cap=3

    tokio::time::sleep(Duration::from_millis(50)).await;

    // r1 must be gone
    assert!(buffer.get("r1").await.is_empty(), "r1 should be evicted");
    // r2, r3, r4 must survive
    assert_eq!(buffer.get("r2").await.len(), 1, "r2 must survive");
    assert_eq!(buffer.get("r3").await.len(), 1, "r3 must survive");
    assert_eq!(buffer.get("r4").await.len(), 1, "r4 must survive");
}

/// Publishing a second envelope for r1 bumps it to most-recent.
/// With runs_cap=2: after r1, r2, r1-again, then r3 — r2 should be evicted (it's LRU).
#[tokio::test]
async fn lru_touch_on_new_envelope_prevents_eviction() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn_with_runs_cap(&bus, 100, 2);

    bus.publish(make_envelope("r1", "e1", "a")); // order: [r1]
    bus.publish(make_envelope("r2", "e2", "b")); // order: [r1, r2]
    bus.publish(make_envelope("r1", "e3", "c")); // r1 touched → order: [r2, r1]
    bus.publish(make_envelope("r3", "e4", "d")); // r2 evicted → order: [r1, r3]

    tokio::time::sleep(Duration::from_millis(50)).await;

    // r2 must be evicted
    assert!(
        buffer.get("r2").await.is_empty(),
        "r2 should be evicted (was LRU)"
    );
    // r1 has 2 envelopes (e1 + e3)
    assert_eq!(buffer.get("r1").await.len(), 2, "r1 must have 2 envelopes");
    // r3 has 1 envelope
    assert_eq!(buffer.get("r3").await.len(), 1, "r3 must survive");
}

/// `get` must NOT count as a touch — it's read-only.
/// With runs_cap=2: publish r1, r2 → call get("r1") → publish r3.
/// r1 (the LRU, since get didn't touch it) must be evicted, not r2.
#[tokio::test]
async fn get_does_not_touch_lru_order() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn_with_runs_cap(&bus, 100, 2);

    bus.publish(make_envelope("r1", "e1", "a")); // order: [r1]
    bus.publish(make_envelope("r2", "e2", "b")); // order: [r1, r2]

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Read r1 — must NOT affect LRU order
    let _ = buffer.get("r1").await;

    bus.publish(make_envelope("r3", "e3", "c")); // r1 evicted → order: [r2, r3]

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(
        buffer.get("r1").await.is_empty(),
        "r1 should be evicted (get must not touch)"
    );
    assert_eq!(buffer.get("r2").await.len(), 1, "r2 must survive");
    assert_eq!(buffer.get("r3").await.len(), 1, "r3 must survive");
}

/// Per-run cap still works independently of runs_cap.
#[tokio::test]
async fn per_run_cap_still_enforced_with_runs_cap() {
    let bus = EventBus::new();
    let buffer = EventHistoryBuffer::spawn_with_runs_cap(&bus, 5, 10);

    for i in 0..10 {
        bus.publish(make_envelope("r1", &format!("e{i}"), ""));
    }
    tokio::time::sleep(Duration::from_millis(50)).await;

    let r1 = buffer.get("r1").await;
    assert_eq!(r1.len(), 5, "per-run cap must still be enforced");
    assert_eq!(r1[0].event_id, "e5");
    assert_eq!(r1[4].event_id, "e9");
}
