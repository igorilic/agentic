use std::path::PathBuf;
use std::time::{Duration, Instant};

use agentic_core::Db;
use agentic_core::db::findings::{FindingRow, FindingsRepo};
use agentic_core::db::runs::{Run, RunRepo};
use agentic_core::db::steps::{Step, StepRepo};
use agentic_core::events::{Event, EventEnvelope, RunStatus, Severity, StepStatus};
use tauri::{AppHandle, Manager, Runtime, State};
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

use super::events::EventBusState;

/// Default inter-event sleep — gives the UI enough wall-clock to render
/// streaming. Override via the `delay_ms` argument for tests / real use.
pub const DEFAULT_DELAY_MS: u64 = 50;

/// Workspace id seeded at app startup that scripted runs FK against. Mirrors
/// the seed in `main.rs::setup`.
const DEFAULT_WORKSPACE_ID: &str = "default";

#[derive(Debug, thiserror::Error)]
pub enum ScriptedRunError {
    #[error("read script: {0}")]
    Io(String),
    #[error("parse script JSON as Vec<Event>: {0}")]
    Parse(String),
    #[error("script path outside allowed scope: {0}")]
    PathOutsideScope(String),
    #[error("seed run row: {0}")]
    Seed(String),
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

// ─── persistence helpers (CP-9 wiring) ──────────────────────────────────────

fn unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Seed a `runs` row via [`RunRepo`] so findings emitted by the script can
/// FK against it. Called synchronously before `tokio::spawn` so DB errors
/// surface to the Tauri caller rather than vanishing in the background task.
fn seed_run_row(db: &Db, run_id: &str, started_at: i64) -> Result<(), ScriptedRunError> {
    RunRepo::new(db)
        .insert(Run {
            id: run_id.to_string(),
            workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
            pipeline_name: "scripted".to_string(),
            status: RunStatus::Running,
            ticket_type: None,
            ticket_ref: None,
            ticket_title: None,
            ticket_body: None,
            backend: "scripted".to_string(),
            model: "fake".to_string(),
            started_at,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })
        .map(|_| ())
        .map_err(|e| ScriptedRunError::Seed(e.to_string()))
}

/// Insert a `run_steps` row via [`StepRepo`]. Used on `Event::StepStarted`
/// and as an implicit "default step" when a script emits a `Finding` before
/// any `StepStarted`.
fn seed_step_row(
    db: &Db,
    run_id: &str,
    step_id: &str,
    seq: i64,
    agent: &str,
    started_at: i64,
) -> agentic_core::Result<()> {
    StepRepo::new(db)
        .insert(Step {
            id: step_id.to_string(),
            run_id: run_id.to_string(),
            seq,
            agent_name: agent.to_string(),
            status: StepStatus::Running,
            started_at: Some(started_at),
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            retry_count: 0,
        })
        .map(|_| ())
}

fn severity_str(s: &Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// Tauri command. Loads `script_path` (JSON `[Event, Event, ...]`) and
/// publishes one envelope per event to the EventBus, sleeping
/// `delay_ms` between events. Returns the synthetic run_id used for
/// the publish so the frontend can correlate.
///
/// CP-9: the run also seeds a `runs` row and projects `Event::Finding` into
/// the typed `findings` table so the cockpit's FindingsTable has data to
/// render. Step rows are created on `Event::StepStarted` and used as the
/// `findings.step_id` FK target.
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
    db_state: State<'_, Db>,
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

    // CP-9: seed the runs row up front so any Finding insert in the loop
    // satisfies the FK constraint.
    let db = (*db_state).clone();
    seed_run_row(&db, &run_id, unix_ms()).map_err(|e| e.to_string())?;

    // F7: create + register a cancellation token for this run.
    let cancel_token = CancellationToken::new();
    state
        .register_cancellation(run_id.clone(), cancel_token.clone())
        .await;

    // Spawn so the command returns immediately.
    tokio::spawn(async move {
        let findings_repo = FindingsRepo::new(&db);
        let started = Instant::now();
        let mut cancelled = false;
        let mut current_step_id: Option<String> = None;
        let mut step_seq: i64 = 0;

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

            // CP-9: project typed events into typed tables BEFORE publishing
            // so the frontend's `list_findings(runId)` (called on history
            // reattach) sees a consistent view.
            match &event {
                Event::StepStarted { agent, .. } => {
                    let step_id = Ulid::new().to_string().to_lowercase();
                    if let Err(e) =
                        seed_step_row(&db, &run_id, &step_id, step_seq, agent, unix_ms())
                    {
                        tracing::warn!(
                            error = %e,
                            "scripted_run: failed to insert run_steps row; continuing",
                        );
                    } else {
                        current_step_id = Some(step_id);
                        step_seq += 1;
                    }
                }
                Event::Finding {
                    finding_id,
                    severity,
                    file,
                    line,
                    message,
                    suggestion,
                } => {
                    // Ensure a step row exists. Scripts that emit a Finding
                    // before any StepStarted get a synthetic "default" step
                    // so the FK still resolves.
                    let step_id = match current_step_id.clone() {
                        Some(id) => id,
                        None => {
                            let id = Ulid::new().to_string().to_lowercase();
                            if let Err(e) =
                                seed_step_row(&db, &run_id, &id, step_seq, "scripted", unix_ms())
                            {
                                tracing::warn!(
                                    error = %e,
                                    "scripted_run: failed to insert default run_steps row",
                                );
                            } else {
                                step_seq += 1;
                                current_step_id = Some(id.clone());
                            }
                            id
                        }
                    };
                    let row = FindingRow {
                        id: finding_id.clone(),
                        run_id: run_id.clone(),
                        step_id,
                        severity: severity_str(severity).to_string(),
                        file_path: file.as_ref().map(|p| p.display().to_string()),
                        line: *line,
                        message: message.clone(),
                        suggestion: suggestion.clone(),
                        triage: None,
                        triaged_at: None,
                        created_at: unix_ms(),
                    };
                    if let Err(e) = findings_repo.insert(&row) {
                        tracing::warn!(
                            error = %e,
                            finding_id = %row.id,
                            "scripted_run: failed to persist finding; continuing",
                        );
                    }
                }
                _ => {}
            }

            let envelope = EventEnvelope::now(run_id.clone(), current_step_id.clone(), event);
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
