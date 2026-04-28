//! N-API bindings for `agentic-core`.
//!
//! Exposes `start_run`, `subscribe_events`, `triage_finding` to Node.js
//! via napi-rs. The async-iterator surface for `subscribe_events` is
//! modeled as an `EventStream` class with an async `next()` method;
//! the JS wrapper attaches a `[Symbol.asyncIterator]` that drives
//! `next()` until it yields `null`.
//!
//! State strategy: a single process-wide `EventBus` is lazily created
//! on first use, so events emitted by `start_run` flow to all
//! concurrent `subscribe_events` calls in the same Node process. The
//! DB is opened per-call from the supplied `data_dir` — cheap because
//! `Db` uses an r2d2 pool internally.

#![deny(unsafe_code)]

use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

use agentic_core::db::findings::{FindingRow, FindingsRepo};
use agentic_core::events::{
    DEFAULT_CAPACITY, Event, EventBus, EventEnvelope, RunStatus, Severity, StepStatus,
};
use agentic_core::{Db, Paths, Run, RunRepo, Step, StepRepo, Workspace, WorkspaceRepo};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use ulid::Ulid;

const DEFAULT_WORKSPACE_ID: &str = "default";

// ─── shared state ────────────────────────────────────────────────────────────

/// Process-wide bus. All `start_run` invocations publish here; all
/// `subscribe_events` calls subscribe from here.
static BUS: LazyLock<EventBus> = LazyLock::new(|| EventBus::with_capacity(DEFAULT_CAPACITY));

/// Active cancel tokens keyed by run_id. Future `cancelRun` API will
/// look up + trigger from this map.
static CANCELS: LazyLock<Mutex<std::collections::HashMap<String, CancellationToken>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn open_db(data_dir: &str) -> Result<Db> {
    let root = Path::new(data_dir);
    let paths = Paths::for_tests(root);
    paths
        .ensure_dirs()
        .map_err(|e| Error::from_reason(format!("ensure data_dir: {e}")))?;
    Db::open(&paths).map_err(|e| Error::from_reason(format!("open db: {e}")))
}

fn ensure_default_workspace(db: &Db) -> Result<()> {
    let now = now_ms();
    WorkspaceRepo::new(db)
        .insert_if_absent(Workspace {
            id: DEFAULT_WORKSPACE_ID.to_string(),
            name: "default".to_string(),
            root_path: ".".to_string(),
            remote_url: None,
            profile: "default".to_string(),
            created_at: now,
            last_opened: now,
        })
        .map(|_| ())
        .map_err(|e| Error::from_reason(format!("seed workspace: {e}")))
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

// ─── triageFinding ───────────────────────────────────────────────────────────

#[napi(object)]
pub struct TriageOptions {
    pub data_dir: String,
    pub run_id: String,
    pub finding_id: String,
    pub triage: String,
}

/// Set the triage state on a finding. `triage` must be one of
/// `"fix" | "tech-debt" | "ignore"`. Mirrors Tauri's `triage_finding`
/// command.
#[napi]
pub async fn triage_finding(opts: TriageOptions) -> Result<()> {
    let TriageOptions {
        data_dir,
        run_id,
        finding_id,
        triage,
    } = opts;
    let db = open_db(&data_dir)?;
    let repo = FindingsRepo::new(&db);
    let updated = repo
        .update_triage(&run_id, &finding_id, &triage, now_ms())
        .map_err(|e| Error::from_reason(e.to_string()))?;
    if !updated {
        return Err(Error::from_reason(format!(
            "finding not found: run={run_id} finding={finding_id}"
        )));
    }
    Ok(())
}

// ─── subscribeEvents ─────────────────────────────────────────────────────────

/// Async-iterator handle. Wraps a tokio broadcast receiver in a mutex
/// (napi-rs's async methods take `&self`, so the receiver must live
/// behind a sync boundary) and filters to a single `run_id`. `next()`
/// returns the next envelope for that run as JSON, or `null` once
/// the stream ends. The JS layer wraps this into a real
/// `[Symbol.asyncIterator]`.
#[napi]
pub struct EventStream {
    rx: tokio::sync::Mutex<broadcast::Receiver<EventEnvelope>>,
    filter_run_id: String,
}

#[napi]
impl EventStream {
    /// Yield the next envelope for this stream's run as a JSON string.
    /// Returns `null` when the stream ends (all senders dropped).
    /// Lagged events (subscriber fell behind capacity) are skipped
    /// rather than returned as errors — UI subscribers don't benefit
    /// from re-receiving every missed delta.
    #[napi]
    pub async fn next(&self) -> Result<Option<String>> {
        let mut rx = self.rx.lock().await;
        loop {
            match rx.recv().await {
                Ok(env) => {
                    if env.run_id == self.filter_run_id {
                        let s = serde_json::to_string(&env)
                            .map_err(|e| Error::from_reason(format!("serialize envelope: {e}")))?;
                        return Ok(Some(s));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return Ok(None),
            }
        }
    }
}

/// Subscribe to bus envelopes filtered by `run_id`.
#[napi]
pub fn subscribe_events(run_id: String) -> EventStream {
    EventStream {
        rx: tokio::sync::Mutex::new(BUS.subscribe()),
        filter_run_id: run_id,
    }
}

// ─── startRun ───────────────────────────────────────────────────────────────

#[napi(object)]
pub struct StartRunOptions {
    pub data_dir: String,
    /// Path to a JSON file containing `Vec<Event>` — the scripted-run
    /// fixture format used elsewhere in the project.
    pub script_path: String,
    /// Per-event delay in milliseconds. Default 0 (no throttling).
    pub delay_ms: Option<u32>,
}

#[napi(object)]
pub struct StartRunResult {
    pub run_id: String,
}

/// Start a scripted run. Returns immediately with the new `run_id`;
/// events flow through the global BUS so `subscribe_events(run_id)`
/// will pick them up. Real claude-code / copilot-cli backends are
/// out of scope for this step — they need agent discovery and
/// workspace setup that has its own design pass.
#[napi]
pub async fn start_run(opts: StartRunOptions) -> Result<StartRunResult> {
    let StartRunOptions {
        data_dir,
        script_path,
        delay_ms,
    } = opts;

    let db = open_db(&data_dir)?;
    ensure_default_workspace(&db)?;

    let raw = tokio::fs::read_to_string(&script_path)
        .await
        .map_err(|e| Error::from_reason(format!("read script {script_path}: {e}")))?;
    let events: Vec<Event> =
        serde_json::from_str(&raw).map_err(|e| Error::from_reason(format!("parse script: {e}")))?;

    let run_id = Ulid::new().to_string().to_lowercase();
    seed_run_row(&db, &run_id)?;

    let cancel = CancellationToken::new();
    {
        let mut map = CANCELS.lock().expect("CANCELS poisoned");
        map.insert(run_id.clone(), cancel.clone());
    }

    let bus_sender = BUS.sender();
    let returned = run_id.clone();
    let delay = delay_ms.unwrap_or(0) as u64;
    let db_for_task = db.clone();

    tokio::spawn(async move {
        run_scripted_loop(db_for_task, run_id, events, cancel, bus_sender, delay).await;
    });

    Ok(StartRunResult { run_id: returned })
}

/// Iterate the scripted events, projecting them to typed tables and
/// publishing to the bus. Mirrors the Tauri scripted command's loop
/// closely (CP-9 contract): every Finding gets a `findings` row with
/// a valid `step_id` FK; every StepStarted gets a `run_steps` row.
/// A synthetic `RunComplete` is published at the end.
async fn run_scripted_loop(
    db: Db,
    run_id: String,
    events: Vec<Event>,
    cancel: CancellationToken,
    bus_sender: broadcast::Sender<EventEnvelope>,
    delay_ms: u64,
) {
    let findings_repo = FindingsRepo::new(&db);
    let mut current_step_id: Option<String> = None;
    let mut step_seq: i64 = 0;
    let mut cancelled = false;

    for event in events {
        if cancel.is_cancelled() {
            cancelled = true;
            break;
        }

        // Project to typed tables BEFORE publishing so any
        // re-entry / list-by-run query sees a consistent view.
        match &event {
            Event::StepStarted { agent, .. } => {
                let step_id = Ulid::new().to_string().to_lowercase();
                if let Err(e) = StepRepo::new(&db).insert(Step {
                    id: step_id.clone(),
                    run_id: run_id.clone(),
                    seq: step_seq,
                    agent_name: agent.to_string(),
                    status: StepStatus::Running,
                    started_at: Some(now_ms()),
                    completed_at: None,
                    duration_ms: None,
                    token_usage: None,
                    cost_usd: None,
                    summary: None,
                    retry_count: 0,
                }) {
                    tracing::warn!(error = %e, "scripted_run: insert step failed");
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
                // Need a step_id for the FK. If we haven't seen
                // StepStarted yet, synthesize a default one.
                let step_id = match current_step_id.clone() {
                    Some(id) => id,
                    None => {
                        let id = Ulid::new().to_string().to_lowercase();
                        if let Err(e) = StepRepo::new(&db).insert(Step {
                            id: id.clone(),
                            run_id: run_id.clone(),
                            seq: step_seq,
                            agent_name: "scripted".to_string(),
                            status: StepStatus::Running,
                            started_at: Some(now_ms()),
                            completed_at: None,
                            duration_ms: None,
                            token_usage: None,
                            cost_usd: None,
                            summary: None,
                            retry_count: 0,
                        }) {
                            tracing::warn!(error = %e, "scripted_run: synthesize step failed");
                        } else {
                            current_step_id = Some(id.clone());
                            step_seq += 1;
                        }
                        id
                    }
                };
                if let Err(e) = findings_repo.insert(&FindingRow {
                    id: finding_id.clone(),
                    run_id: run_id.clone(),
                    step_id,
                    severity: severity_str(*severity).to_string(),
                    file_path: file.as_ref().map(|p| p.display().to_string()),
                    line: *line,
                    message: message.clone(),
                    suggestion: suggestion.clone(),
                    triage: None,
                    triaged_at: None,
                    created_at: now_ms(),
                }) {
                    tracing::warn!(error = %e, "scripted_run: insert finding failed");
                }
            }
            _ => {}
        }

        let envelope = EventEnvelope::now(run_id.clone(), current_step_id.clone(), event);
        let _ = bus_sender.send(envelope);

        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
    }

    let summary = if cancelled {
        "cancelled"
    } else {
        "scripted complete"
    };
    let status = if cancelled {
        RunStatus::Cancelled
    } else {
        RunStatus::Completed
    };
    let envelope = EventEnvelope::now(
        run_id.clone(),
        None,
        Event::RunComplete {
            status,
            duration_ms: 0,
            summary: summary.to_string(),
        },
    );
    let _ = bus_sender.send(envelope);

    let mut map = CANCELS.lock().expect("CANCELS poisoned");
    map.remove(&run_id);
}

fn seed_run_row(db: &Db, run_id: &str) -> Result<()> {
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
            started_at: now_ms(),
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })
        .map(|_| ())
        .map_err(|e| Error::from_reason(format!("seed run: {e}")))
}
