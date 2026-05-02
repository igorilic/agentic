use agentic_core::{
    Db, Event, EventBus, EventEnvelope, EventPersister, Paths, PermissionDecision, PermissionRisk,
    PermissionSource, Severity,
};
use rusqlite::params;

fn setup() -> (tempfile::TempDir, Db, EventBus) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let bus = EventBus::new();
    (tmp, db, bus)
}

fn make_event(run_id: &str, attempt: u32) -> EventEnvelope {
    EventEnvelope::now(
        run_id.to_string(),
        None,
        Event::RetryStarted {
            attempt,
            reason: format!("attempt {attempt}"),
        },
    )
}

#[tokio::test]
async fn publishing_100_events_produces_100_rows_with_seq_0_to_99() {
    let (_tmp, db, bus) = setup();
    let subscriber = bus.subscribe();
    let handle = EventPersister::spawn(subscriber, db.clone());

    for i in 0..100u32 {
        bus.publish(make_event("run1", i));
    }

    // Drop the bus so the persister sees Closed and exits cleanly.
    drop(bus);
    handle.await.expect("persister task joins");

    let conn = db.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stream_events WHERE run_id = 'run1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 100, "expected 100 rows, got {count}");

    let seqs: Vec<i64> = conn
        .prepare("SELECT seq FROM stream_events WHERE run_id = 'run1' ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(
        seqs,
        (0..100i64).collect::<Vec<_>>(),
        "seq must be 0..=99 contiguous"
    );
}

#[tokio::test]
async fn seq_counters_are_independent_per_run_id() {
    let (_tmp, db, bus) = setup();
    let subscriber = bus.subscribe();
    let handle = EventPersister::spawn(subscriber, db.clone());

    // Interleave publishes so a shared global counter impl would fail.
    // 5 paired rounds of (run-A, run-B), then 5 more run-A publishes.
    // Expected final seqs: run-A = 0..=9, run-B = 0..=4.
    for i in 0..5u32 {
        bus.publish(make_event("run-A", i));
        bus.publish(make_event("run-B", i));
    }
    for i in 5..10u32 {
        bus.publish(make_event("run-A", i));
    }

    drop(bus);
    handle.await.expect("persister task joins");

    let conn = db.conn().unwrap();
    let total: i64 = conn
        .query_row("SELECT COUNT(*) FROM stream_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(total, 15);

    let a_seqs: Vec<i64> = conn
        .prepare("SELECT seq FROM stream_events WHERE run_id = 'run-A' ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    let b_seqs: Vec<i64> = conn
        .prepare("SELECT seq FROM stream_events WHERE run_id = 'run-B' ORDER BY seq")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<std::result::Result<_, _>>()
        .unwrap();
    assert_eq!(a_seqs, (0..10i64).collect::<Vec<_>>());
    assert_eq!(b_seqs, (0..5i64).collect::<Vec<_>>());
}

#[tokio::test]
async fn persisted_payload_roundtrips_through_rmp_serde() {
    let (_tmp, db, bus) = setup();
    let subscriber = bus.subscribe();
    let handle = EventPersister::spawn(subscriber, db.clone());

    let original = Event::Finding {
        finding_id: "f1".to_string(),
        severity: Severity::Warning,
        file: Some("src/lib.rs".into()),
        line: Some(42),
        message: "watch out".to_string(),
        suggestion: Some("use ?".to_string()),
    };
    let envelope = EventEnvelope::now(
        "run-X".to_string(),
        Some("step-Y".to_string()),
        original.clone(),
    );
    bus.publish(envelope.clone());

    drop(bus);
    handle.await.expect("persister task joins");

    let conn = db.conn().unwrap();
    let (event_type, payload, step_id, timestamp_ms): (String, Vec<u8>, Option<String>, i64) = conn
        .query_row(
            "SELECT event_type, payload, step_id, timestamp_ms FROM stream_events WHERE run_id = ?1",
            params![envelope.run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();

    assert_eq!(event_type, "Finding");
    assert_eq!(step_id.as_deref(), Some("step-Y"));
    assert_eq!(timestamp_ms, envelope.timestamp_ms);

    let decoded: Event = rmp_serde::from_slice(&payload).expect("rmp-serde decodes payload");
    assert_eq!(decoded, original, "roundtripped event must equal original");
}

// --- P.1.1: event_type_tag coverage for new permission variants ---

#[tokio::test]
async fn permission_request_event_type_tag_is_correct() {
    let (_tmp, db, bus) = setup();
    let subscriber = bus.subscribe();
    let handle = EventPersister::spawn(subscriber, db.clone());

    let event = Event::PermissionRequest {
        request_id: "req-01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
        agent: "developer".to_string(),
        tool: "Bash".to_string(),
        arg: "rm -rf node_modules".to_string(),
        scope: "shell.destructive".to_string(),
        risk: PermissionRisk::High,
        reason: "destructive shell".to_string(),
    };
    let envelope = EventEnvelope::now("run-perm-req".to_string(), None, event);
    bus.publish(envelope);

    drop(bus);
    handle.await.expect("persister task joins");

    let conn = db.conn().unwrap();
    let event_type: String = conn
        .query_row(
            "SELECT event_type FROM stream_events WHERE run_id = 'run-perm-req'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(event_type, "PermissionRequest");
}

#[tokio::test]
async fn permission_resolved_event_type_tag_is_correct() {
    let (_tmp, db, bus) = setup();
    let subscriber = bus.subscribe();
    let handle = EventPersister::spawn(subscriber, db.clone());

    let event = Event::PermissionResolved {
        request_id: "req-01JZZZZZZZZZZZZZZZZZZZZZZZ".to_string(),
        decision: PermissionDecision::AllowOnce,
        source: PermissionSource::User,
    };
    let envelope = EventEnvelope::now("run-perm-res".to_string(), None, event);
    bus.publish(envelope);

    drop(bus);
    handle.await.expect("persister task joins");

    let conn = db.conn().unwrap();
    let event_type: String = conn
        .query_row(
            "SELECT event_type FROM stream_events WHERE run_id = 'run-perm-res'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(event_type, "PermissionResolved");
}
