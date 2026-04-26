use std::collections::HashMap;
use std::sync::Arc;

use agentic_core::events::EventBus;
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Per-app shared state holding the EventBus. Created at app setup time and
/// stored in Tauri's managed state.
///
/// The `forwarder` slot tracks the active background task spawned by
/// `subscribe_events`. Re-invoking the command aborts the previous handle and
/// installs a new one, so the webview receives exactly one stream of envelopes
/// regardless of how many times the frontend re-attaches (e.g., during Vite
/// HMR which re-invokes on every save).
pub struct EventBusState {
    pub bus: Arc<EventBus>,
    /// Handle to the active subscriber forwarder, if any. Re-invoking
    /// `subscribe_events` aborts the previous handle and replaces it,
    /// so the webview receives exactly one stream of envelopes regardless
    /// of how many times the frontend re-attaches (e.g., during Vite HMR).
    forwarder: Mutex<Option<JoinHandle<()>>>,
    /// Active run cancellation tokens, keyed by run_id.
    cancellations: Mutex<HashMap<String, CancellationToken>>,
}

impl EventBusState {
    pub fn new(bus: Arc<EventBus>) -> Self {
        Self {
            bus,
            forwarder: Mutex::new(None),
            cancellations: Mutex::new(HashMap::new()),
        }
    }

    /// Register a cancellation token for `run_id`.
    pub async fn register_cancellation(&self, run_id: String, token: CancellationToken) {
        self.cancellations.lock().await.insert(run_id, token);
    }

    /// Cancel the run identified by `run_id`. Returns `true` if a token was
    /// found and cancelled, `false` if no such run is registered (idempotent).
    pub async fn cancel(&self, run_id: &str) -> bool {
        let mut map = self.cancellations.lock().await;
        if let Some(token) = map.remove(run_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
}

/// The frontend channel name for forwarded envelopes. Frontend listens via
/// `window.listen("agentic.event", handler)`.
pub const EVENT_CHANNEL: &str = "agentic.event";

/// Tauri command. Subscribes to the EventBus and forwards every envelope as
/// a `tauri::Event` named `agentic.event`. Spawns a background tokio task.
///
/// Re-invoking this command aborts any previously spawned forwarder and
/// replaces it with a fresh one. This prevents Vite HMR from accumulating
/// N duplicate background tasks after N hot-reloads.
///
/// Returns immediately after spawning. The frontend MUST register a listener
/// before invoking this command, or events sent before listener registration
/// will be lost.
#[tauri::command]
pub async fn subscribe_events<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EventBusState>,
) -> Result<(), String> {
    let mut subscriber = state.bus.subscribe();

    // Spawn the new forwarder.
    let new_handle = tokio::spawn(async move {
        loop {
            match subscriber.recv().await {
                Ok(envelope) => {
                    if let Err(e) = app.emit(EVENT_CHANNEL, &envelope) {
                        tracing::warn!(error = %e, "subscribe_events: emit failed");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("subscribe_events: bus closed; forwarder exiting");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "subscribe_events: lagged behind bus");
                    // continue; broadcast catches up at next call
                }
            }
        }
    });

    // Atomically swap: take any old handle and abort it, install new one.
    let mut slot = state.forwarder.lock().await;
    if let Some(old) = slot.take() {
        old.abort();
        tracing::debug!("subscribe_events: aborted previous forwarder (re-invocation)");
    }
    *slot = Some(new_handle);

    Ok(())
}
