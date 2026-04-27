use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::events::{EventBus, EventEnvelope};

/// Default cap per run. Bigger than the webview's display buffer
/// (MAX_EVENTS = 500 in useTauriEvents) to ensure replay covers any
/// realistic mid-run attach window.
pub const DEFAULT_HISTORY_CAP: usize = 1000;

#[derive(Default)]
struct Inner {
    by_run: HashMap<String, Vec<EventEnvelope>>,
    cap: usize,
}

/// Per-run ring buffer of recent envelopes. Subscribes to the bus and
/// records each envelope under its run_id, capped at `cap` per run.
/// On replay (e.g., webview reattach), the buffer's `get(run_id)`
/// returns the recorded prefix in order.
pub struct EventHistoryBuffer {
    inner: Arc<Mutex<Inner>>,
    /// Spawned subscriber task. Kept for graceful shutdown if needed.
    _subscriber: JoinHandle<()>,
}

impl EventHistoryBuffer {
    pub fn spawn(bus: &EventBus, cap: usize) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            by_run: HashMap::new(),
            cap,
        }));
        let inner_clone = inner.clone();
        let mut rx = bus.subscribe();
        let subscriber = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(envelope) => {
                        let mut guard = inner_clone.lock().await;
                        let cap = guard.cap;
                        let entry = guard.by_run.entry(envelope.run_id.clone()).or_default();
                        entry.push(envelope);
                        // Sliding window: drop oldest if over cap.
                        if entry.len() > cap {
                            let drop_count = entry.len() - cap;
                            entry.drain(..drop_count);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "EventHistoryBuffer: lagged");
                    }
                }
            }
        });
        Self {
            inner,
            _subscriber: subscriber,
        }
    }

    pub fn spawn_default(bus: &EventBus) -> Self {
        Self::spawn(bus, DEFAULT_HISTORY_CAP)
    }

    /// Snapshot of buffered envelopes for the given run_id. Returns empty
    /// vec if no envelopes for that run.
    pub async fn get(&self, run_id: &str) -> Vec<EventEnvelope> {
        let guard = self.inner.lock().await;
        guard.by_run.get(run_id).cloned().unwrap_or_default()
    }
}
