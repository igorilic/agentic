use std::path::PathBuf;
use std::time::Duration;

use agentic_core::events::{Event, EventEnvelope};
use tauri::{AppHandle, Runtime, State};
use ulid::Ulid;

use super::events::EventBusState;

/// Default inter-event sleep — gives the UI enough wall-clock to render
/// streaming. Override via the `delay_ms` argument for tests / real use.
pub const DEFAULT_DELAY_MS: u64 = 50;

#[derive(Debug, thiserror::Error)]
pub enum ScriptedRunError {
    #[error("read script: {0}")]
    Io(String),
    #[error("parse script JSON as Vec<Event>: {0}")]
    Parse(String),
}

/// Tauri command. Loads `script_path` (JSON `[Event, Event, ...]`) and
/// publishes one envelope per event to the EventBus, sleeping
/// `delay_ms` between events. Returns the synthetic run_id used for
/// the publish so the frontend can correlate.
///
/// Errors surface as `String` (Tauri's command-error convention).
#[tauri::command]
pub async fn start_scripted_run<R: Runtime>(
    _app: AppHandle<R>,
    state: State<'_, EventBusState>,
    script_path: String,
    delay_ms: Option<u64>,
) -> Result<String, String> {
    let path = PathBuf::from(&script_path);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| ScriptedRunError::Io(format!("{}: {e}", path.display())).to_string())?;
    let events: Vec<Event> = serde_json::from_str(&raw)
        .map_err(|e| ScriptedRunError::Parse(e.to_string()).to_string())?;

    let run_id = Ulid::new().to_string().to_lowercase();
    let bus = state.bus.clone();
    let delay = Duration::from_millis(delay_ms.unwrap_or(DEFAULT_DELAY_MS));
    let returned_run_id = run_id.clone();

    // Spawn so the command returns immediately; the publishing happens
    // in the background. The frontend gets envelopes via the
    // `agentic://event` channel that subscribe_events already forwards.
    tokio::spawn(async move {
        for event in events {
            let envelope = EventEnvelope::now(run_id.clone(), None, event);
            bus.publish(envelope);
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }
        }
    });

    Ok(returned_run_id)
}
