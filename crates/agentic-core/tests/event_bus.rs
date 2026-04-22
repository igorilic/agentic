use agentic_core::{Event, EventBus, EventEnvelope};
use tokio::sync::broadcast::error::RecvError;

fn sample(attempt: u32) -> EventEnvelope {
    EventEnvelope::now(
        "run1".to_string(),
        None,
        Event::RetryStarted {
            attempt,
            reason: format!("attempt {attempt}"),
        },
    )
}

#[tokio::test]
async fn two_subscribers_each_receive_every_published_event() {
    let bus = EventBus::new();
    let mut rx1 = bus.subscribe();
    let mut rx2 = bus.subscribe();

    for i in 1..=3 {
        bus.publish(sample(i));
    }

    for expected in 1..=3u32 {
        let a = rx1.recv().await.expect("rx1 recv");
        let b = rx2.recv().await.expect("rx2 recv");
        assert_eq!(
            a.event_id, b.event_id,
            "both subscribers see the same envelope"
        );
        match (&a.event, &b.event) {
            (Event::RetryStarted { attempt: x, .. }, Event::RetryStarted { attempt: y, .. }) => {
                assert_eq!(*x, expected);
                assert_eq!(*y, expected);
            }
            _ => panic!("expected RetryStarted"),
        }
    }
}

#[tokio::test]
async fn slow_subscriber_lagging_past_capacity_yields_lagged_and_fresh_subscriber_still_works() {
    let bus = EventBus::with_capacity(4);
    let mut slow = bus.subscribe();

    // Publish 10 without consuming. Slow lags by 10 - 4 = 6.
    for i in 1..=10 {
        bus.publish(sample(i));
    }

    match slow.recv().await {
        Err(RecvError::Lagged(n)) => {
            // Capacity 4, 10 publishes, 0 consumed → skipped exactly 6.
            // Tokio broadcast lag count is deterministic per-subscriber.
            assert_eq!(
                n, 6,
                "expected exactly 6 skipped events (10 publish - 4 capacity), got {n}"
            );
        }
        other => panic!("expected Lagged error, got {other:?}"),
    }

    // After a Lagged error, recv() resumes at the oldest still-buffered message.
    // Buffer holds events 7, 8, 9, 10 (the last `capacity` = 4 publishes).
    let recovered = slow
        .recv()
        .await
        .expect("slow subscriber recovers after Lagged");
    match recovered.event {
        Event::RetryStarted { attempt, .. } => {
            assert_eq!(
                attempt, 7,
                "expected oldest still-buffered event (7), got {attempt}"
            );
        }
        other => panic!("expected RetryStarted(7) on recovery, got {other:?}"),
    }

    // Subscribe fresh AFTER the lag. Publish a new event.
    let mut fresh = bus.subscribe();
    bus.publish(sample(100));

    let received = fresh
        .recv()
        .await
        .expect("fresh subscriber receives cleanly");
    match received.event {
        Event::RetryStarted { attempt, .. } => assert_eq!(attempt, 100),
        other => panic!("expected RetryStarted, got {other:?}"),
    }
}

#[tokio::test]
async fn publish_returns_number_of_active_receivers() {
    let bus = EventBus::new();

    // No subscribers: publish returns 0
    assert_eq!(bus.publish(sample(1)), 0, "no receivers means 0");

    let _rx1 = bus.subscribe();
    assert_eq!(bus.publish(sample(2)), 1, "one receiver");

    let _rx2 = bus.subscribe();
    assert_eq!(bus.publish(sample(3)), 2, "two receivers");

    // Drop one subscriber: count drops
    drop(_rx1);
    assert_eq!(bus.publish(sample(4)), 1, "one receiver after drop");
}
