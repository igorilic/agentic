use std::sync::Arc;

use agentic_core::events::EventBus;
use tauri::{AppHandle, Emitter, Runtime, State};

/// Per-app shared state holding the EventBus. Created at app setup time and
/// stored in Tauri's managed state. The `subscribe_events` command pulls it
/// out via `State<Arc<EventBus>>`.
pub struct EventBusState(pub Arc<EventBus>);

/// The frontend channel name for forwarded envelopes. Frontend listens via
/// `window.listen("agentic://event", handler)`.
pub const EVENT_CHANNEL: &str = "agentic://event";

/// Tauri command. Subscribes to the EventBus and forwards every envelope as
/// a `tauri::Event` named `agentic://event`. Spawns a background tokio task
/// that lives for the lifetime of the AppHandle (or until the bus drops).
///
/// Returns immediately after spawning. The frontend MUST register a listener
/// before invoking this command, or events sent before listener registration
/// will be lost.
#[tauri::command]
pub async fn subscribe_events<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EventBusState>,
) -> Result<(), String> {
    let mut subscriber = state.0.subscribe();

    tokio::spawn(async move {
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

    Ok(())
}
