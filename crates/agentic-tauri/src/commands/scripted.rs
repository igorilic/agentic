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
    let _ = (state, script_path, delay_ms);
    todo!("start_scripted_run not yet implemented")
}
