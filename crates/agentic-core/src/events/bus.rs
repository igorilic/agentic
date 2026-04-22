use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::events::EventEnvelope;

/// Default broadcast channel capacity (events buffered per-subscriber).
pub const DEFAULT_CAPACITY: usize = 1024;

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
    /// New bus with [`DEFAULT_CAPACITY`] (1024).
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
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

    /// Publish an envelope. Returns the number of active receivers that the
    /// value was sent to (0 if no one is subscribed). Never errors: the "no
    /// receivers" case is normal for an event bus.
    pub fn publish(&self, envelope: EventEnvelope) -> usize {
        // broadcast::Sender::send returns Result<usize, SendError<T>>.
        // Err means no subscribers — return 0 (not an error for a bus).
        self.sender.send(envelope).unwrap_or(0)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
