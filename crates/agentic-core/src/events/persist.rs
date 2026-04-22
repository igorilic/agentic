use rusqlite::params;
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::task::JoinHandle;

use crate::db::Db;
use crate::events::{Event, EventEnvelope};

/// Background event persister. Consumes envelopes from a bus subscriber and
/// writes each one as a row in `stream_events` with a monotonic per-run `seq`.
///
/// # Contract
/// - Spawn **before** the first publish on the bus. Events published before
///   the persister subscribes will be silently dropped by the bus
///   (`EventBus::publish` emits a `tracing::warn!` in that case).
/// - One persister per bus per DB. Multiple persisters would race on `seq`
///   allocation and produce UNIQUE violations on the `(run_id, seq)` PK.
/// - Payload is MessagePack-encoded `Event` (just the variant body, not the
///   envelope — envelope metadata lives in columns).
pub struct EventPersister;

impl EventPersister {
    /// Spawn a background task that persists every envelope received on
    /// `subscriber` to `stream_events`. Runs until the bus drops (subscriber
    /// sees `RecvError::Closed`). On per-event errors (encode failure, DB
    /// write failure), logs via `tracing::error!` and continues; the loop
    /// never panics and never breaks on recoverable errors.
    ///
    /// Returns a `JoinHandle<()>` the caller can await for graceful shutdown
    /// after dropping the bus.
    pub fn spawn(mut subscriber: Receiver<EventEnvelope>, db: Db) -> JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                match subscriber.recv().await {
                    Ok(envelope) => {
                        if let Err(e) = persist_envelope(&db, &envelope) {
                            tracing::error!(
                                run_id = %envelope.run_id,
                                event_id = %envelope.event_id,
                                error = %e,
                                "EventPersister: failed to persist envelope; continuing",
                            );
                        }
                    }
                    Err(RecvError::Closed) => {
                        tracing::info!("EventPersister: bus closed; shutting down");
                        break;
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(
                            skipped = n,
                            "EventPersister: lagged behind bus; skipped {n} envelopes",
                        );
                        // continue; next recv returns the oldest still-buffered envelope
                    }
                }
            }
        })
    }
}

fn persist_envelope(db: &Db, envelope: &EventEnvelope) -> crate::Result<()> {
    // Use to_vec_named (map format) rather than to_vec (array format):
    // Event's #[serde(tag = "type", content = "data")] strategy needs field
    // names to round-trip. Array format drops them, so `from_slice` can't
    // identify the variant. Cost: ~30% larger BLOBs than array MessagePack,
    // within spec §13.1's trade-off tolerance.
    let payload = rmp_serde::to_vec_named(&envelope.event)
        .map_err(|e| crate::CoreError::Db(format!("rmp-serde encode: {e}")))?;
    let event_type = event_type_tag(&envelope.event);

    let mut conn = db.conn()?;
    let tx = conn.transaction()?;
    let seq = next_seq(&tx, &envelope.run_id)?;
    tx.execute(
        "INSERT INTO stream_events (run_id, step_id, seq, event_type, payload, timestamp_ms) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            envelope.run_id,
            envelope.step_id,
            seq,
            event_type,
            payload,
            envelope.timestamp_ms,
        ],
    )?;
    tx.commit()?;
    Ok(())
}

/// Next `seq` for the given `run_id` (0 for the first event, else max+1).
/// Must be called inside the same transaction as the INSERT to avoid race
/// conditions with other persisters — but the single-persister contract
/// makes this mostly defensive.
fn next_seq(tx: &rusqlite::Transaction<'_>, run_id: &str) -> crate::Result<i64> {
    let n: i64 = tx.query_row(
        "SELECT COALESCE(MAX(seq), -1) + 1 FROM stream_events WHERE run_id = ?1",
        params![run_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

fn event_type_tag(event: &Event) -> &'static str {
    match event {
        Event::RunStarted { .. } => "RunStarted",
        Event::RunComplete { .. } => "RunComplete",
        Event::StepStarted { .. } => "StepStarted",
        Event::StepComplete { .. } => "StepComplete",
        Event::TextDelta { .. } => "TextDelta",
        Event::ThinkingDelta { .. } => "ThinkingDelta",
        Event::ToolUseStart { .. } => "ToolUseStart",
        Event::ToolUseDelta { .. } => "ToolUseDelta",
        Event::ToolUseEnd { .. } => "ToolUseEnd",
        Event::FileChange { .. } => "FileChange",
        Event::Finding { .. } => "Finding",
        Event::ClarifyingQuestion { .. } => "ClarifyingQuestion",
        Event::RetryStarted { .. } => "RetryStarted",
        Event::Error { .. } => "Error",
        Event::UserActionNeeded { .. } => "UserActionNeeded",
    }
}
