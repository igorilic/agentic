use tokio::sync::broadcast::Receiver;
use tokio::task::JoinHandle;

use crate::db::Db;
use crate::events::EventEnvelope;

/// Background event persister. Consumes envelopes from a bus subscriber and
/// writes each one as a row in `stream_events` with a monotonic per-run `seq`.
pub struct EventPersister;

impl EventPersister {
    /// Spawn a background task that persists every envelope received on
    /// `subscriber` to `stream_events`. Runs until the bus drops.
    pub fn spawn(_subscriber: Receiver<EventEnvelope>, _db: Db) -> JoinHandle<()> {
        unimplemented!("EventPersister::spawn not yet implemented")
    }
}
