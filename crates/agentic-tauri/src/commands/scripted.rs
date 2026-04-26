use std::path::PathBuf;
use std::time::{Duration, Instant};

use agentic_core::events::{Event, EventEnvelope, RunStatus};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio_util::sync::CancellationToken;
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
    #[error("script path outside allowed scope: {0}")]
    PathOutsideScope(String),
}

// ─── F1: path validation ─────────────────────────────────────────────────────

fn validate_script_path<R: Runtime>(
    raw: &str,
    app: &AppHandle<R>,
) -> Result<PathBuf, ScriptedRunError> {
    let requested = PathBuf::from(raw);
    // Canonicalize — fails if path doesn't exist.
    let canonical = requested
        .canonicalize()
        .map_err(|e| ScriptedRunError::Io(format!("canonicalize {}: {e}", requested.display())))?;

    // Build allowed roots: cwd and Tauri app_data_dir.
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.canonicalize().ok());

    let app_data = app
        .path()
        .app_data_dir()
        .ok()
        .and_then(|p| p.canonicalize().ok());

    let allowed_roots: Vec<PathBuf> = [cwd, app_data].into_iter().flatten().collect();

    if allowed_roots.is_empty() {
        return Err(ScriptedRunError::PathOutsideScope(
            "no allowed scope could be resolved".into(),
        ));
    }

    if allowed_roots.iter().any(|root| canonical.starts_with(root)) {
        Ok(canonical)
    } else {
        Err(ScriptedRunError::PathOutsideScope(format!(
            "{} not under any allowed root",
            canonical.display()
        )))
    }
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// Tauri command. Loads `script_path` (JSON `[Event, Event, ...]`) and
/// publishes one envelope per event to the EventBus, sleeping
/// `delay_ms` between events. Returns the synthetic run_id used for
/// the publish so the frontend can correlate.
///
/// F1: the script path is canonicalized and must be under cwd or app_data_dir.
/// F2: a synthetic RunComplete envelope is published after all events.
/// F3: I/O is async (tokio::fs).
/// F7: a CancellationToken is registered; the loop races against cancellation.
///
/// Errors surface as `String` (Tauri's command-error convention).
#[tauri::command]
pub async fn start_scripted_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EventBusState>,
    script_path: String,
    delay_ms: Option<u64>,
) -> Result<String, String> {
    // F1: validate path before any I/O.
    let path = validate_script_path(&script_path, &app).map_err(|e| e.to_string())?;

    // F3: async I/O.
    let raw = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| ScriptedRunError::Io(format!("{}: {e}", path.display())).to_string())?;

    let events: Vec<Event> = serde_json::from_str(&raw)
        .map_err(|e| ScriptedRunError::Parse(e.to_string()).to_string())?;

    let run_id = Ulid::new().to_string().to_lowercase();
    let bus = state.bus.clone();
    let delay = Duration::from_millis(delay_ms.unwrap_or(DEFAULT_DELAY_MS));
    let returned_run_id = run_id.clone();

    // F7: create + register a cancellation token for this run.
    let cancel_token = CancellationToken::new();
    state
        .register_cancellation(run_id.clone(), cancel_token.clone())
        .await;

    // Spawn so the command returns immediately.
    tokio::spawn(async move {
        let started = Instant::now();
        let mut cancelled = false;

        for event in events {
            // F7: race event publishing against cancellation.
            tokio::select! {
                biased;
                _ = cancel_token.cancelled() => {
                    cancelled = true;
                    break;
                }
                _ = async {} => {}
            }

            let envelope = EventEnvelope::now(run_id.clone(), None, event);
            bus.publish(envelope);

            if !delay.is_zero() {
                tokio::select! {
                    _ = cancel_token.cancelled() => {
                        cancelled = true;
                        break;
                    }
                    _ = tokio::time::sleep(delay) => {}
                }
            }
        }

        // F2: publish synthetic RunComplete.
        let duration_ms = started.elapsed().as_millis() as u64;
        let (status, summary) = if cancelled {
            (RunStatus::Failed, "cancelled".to_string())
        } else {
            (RunStatus::Completed, "scripted run done".to_string())
        };
        let complete_envelope = EventEnvelope::now(
            run_id.clone(),
            None,
            Event::RunComplete {
                status,
                duration_ms,
                summary,
            },
        );
        bus.publish(complete_envelope);
    });

    Ok(returned_run_id)
}

/// Tauri command. Cancels an in-flight scripted run by its run_id.
/// Returns `true` if the run was found and cancelled, `false` if unknown
/// (idempotent — calling cancel on an unknown run is not an error).
#[tauri::command]
pub async fn cancel_run(state: State<'_, EventBusState>, run_id: String) -> Result<bool, String> {
    Ok(state.cancel(&run_id).await)
}
