use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::events::EventEnvelope;

/// Default broadcast channel capacity (events buffered per-subscriber).
///
/// Raised to 16 384 (16× the original 1 024) to accommodate real Claude
/// runs that can emit hundreds of `TextDelta` events per step across a
/// multi-step pipeline. A slow subscriber (e.g. the persister doing
/// per-event SQLite inserts) can lag behind the producer without dropping
/// events at this headroom.
///
/// Override at runtime with the [`CAPACITY_ENV_VAR`] environment variable.
pub const DEFAULT_CAPACITY: usize = 16_384;

/// Env var name to override the default capacity at bus construction.
///
/// Useful for testing extreme cases or constraining memory in embedded
/// deployments. The value must parse as a positive `usize`; invalid or
/// zero values are silently ignored and [`DEFAULT_CAPACITY`] is used.
///
/// Example: `AGENTIC_BUS_CAPACITY=65536 agentic-cli run --ticket "…"`
pub const CAPACITY_ENV_VAR: &str = "AGENTIC_BUS_CAPACITY";

/// In-process event broadcast bus. Clone is cheap (internal Arc via the
/// `broadcast::Sender` wrapper). Subscribers receive every published event
/// in order, up to the channel capacity. If a subscriber lags past
/// capacity, subsequent `recv()` calls yield `RecvError::Lagged(n)` once
/// before recovering at the newest still-buffered event.
#[derive(Clone)]
pub struct EventBus {
    sender: Sender<EventEnvelope>,
}

impl EventBus {
    /// New bus with [`DEFAULT_CAPACITY`] (16 384) unless
    /// [`CAPACITY_ENV_VAR`] (`AGENTIC_BUS_CAPACITY`) is set to a valid
    /// positive integer.
    pub fn new() -> Self {
        let cap = std::env::var(CAPACITY_ENV_VAR)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_CAPACITY);
        Self::with_capacity(cap)
    }

    /// New bus with an explicit capacity. Must be > 0.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to the bus. Returns a `Receiver` that will see every event
    /// published AFTER this call (plus any still-buffered events within the
    /// channel capacity at subscribe time).
    pub fn subscribe(&self) -> Receiver<EventEnvelope> {
        self.sender.subscribe()
    }

    /// Clone the underlying broadcast Sender. Callers that need to publish
    /// via `EventSink` (e.g., `Backend::execute(req, event_sink)`) can obtain
    /// a sink this way rather than routing through `publish` — semantics are
    /// identical (both land in the same broadcast channel).
    pub fn sender(&self) -> crate::backends::EventSink {
        self.sender.clone()
    }

    /// Publish an envelope. Returns the number of active receivers that the
    /// value was sent to (0 if no one is subscribed).
    ///
    /// **Observability**: when no subscribers are active, the envelope is
    /// silently dropped and a `tracing::warn!` is emitted with `run_id`,
    /// `event_id`, and the dropping site. Callers that require delivery
    /// guarantees (e.g. the event persister in Step 2.3) must subscribe
    /// before the first publish.
    pub fn publish(&self, envelope: EventEnvelope) -> usize {
        match self.sender.send(envelope) {
            Ok(n) => n,
            Err(tokio::sync::broadcast::error::SendError(dropped)) => {
                tracing::warn!(
                    run_id = %dropped.run_id,
                    event_id = %dropped.event_id,
                    "EventBus::publish: no active subscribers, event dropped"
                );
                0
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
