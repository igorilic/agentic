//! Tauri IPC for real ticket-driven pipeline runs.
//!
//! Mirrors `agentic-cli`'s `cmd_run_ticket` flow but uses the Tauri-managed
//! `EventBus` so envelopes stream into the cockpit (via `subscribe_events`)
//! instead of stdout. `execute_pipeline` itself lives in `agentic_cli::
//! ticket_run` — see the architectural-cleanup tech-debt note.
//!
//! The command:
//!   1. Validates the requested backend kind.
//!   2. Resolves the workspace root (process cwd) and a stable ws_id.
//!   3. Seeds workspace + run rows synchronously so the caller can pin
//!      `activeRunId` to the returned ULID immediately.
//!   4. Spawns the orchestrator (status projection from events) and
//!      `execute_pipeline` in detached tasks.
//!   5. Returns the run_id.
//!
//! Run failures (missing agent files, missing backend binary, failed token
//! exchange) surface as `RunComplete(failed)` envelopes on the bus — the
//! webview sees them via the existing event stream. The IPC itself only
//! errors on synchronous validation failures (bad backend kind, DB seed
//! failure).

use std::sync::Arc;

use agentic_cli::ticket_run::{PipelineRunContext, execute_pipeline, stable_workspace_id};

/// Shared shape for the `Backend` factory closure used by both prod and
/// tests. Aliasing keeps clippy::type_complexity quiet.
type BackendFactoryArc = Arc<dyn Fn(&PipelineStep) -> Box<dyn Backend> + Send + Sync>;
use agentic_core::db::workspaces::{Workspace, WorkspaceRepo};
use agentic_core::events::{EventBus, RunStatus};
use agentic_core::pipeline::PipelineConfig;
use agentic_core::{
    Backend, ClaudeCodeBackend, CopilotCliBackend, Db, ModelId, Paths, PipelineStep, Run, RunRepo,
};
use tauri::State;
use ulid::Ulid;

use super::events::EventBusState;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    ClaudeCode,
    CopilotCli,
}

impl BackendKind {
    fn id_str(&self) -> &'static str {
        match self {
            BackendKind::ClaudeCode => "claude-code",
            BackendKind::CopilotCli => "copilot-cli",
        }
    }

    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "claude-code" => Ok(BackendKind::ClaudeCode),
            "copilot-cli" => Ok(BackendKind::CopilotCli),
            other => Err(format!(
                "invalid backend: {other:?} (expected 'claude-code' or 'copilot-cli')"
            )),
        }
    }
}

/// Tauri command. Kicks off a real ticket-driven pipeline run against the
/// process working directory. Returns the run_id immediately; the pipeline
/// runs in the background and emits envelopes via the managed EventBus.
#[tauri::command]
pub async fn start_ticket_run(
    bus_state: State<'_, EventBusState>,
    db_state: State<'_, Db>,
    ticket: String,
    backend: String,
    model: Option<String>,
) -> Result<String, String> {
    let backend_kind = BackendKind::parse(&backend)?;
    let ticket = ticket.trim().to_string();
    if ticket.is_empty() {
        return Err("ticket text is empty".to_string());
    }

    // Workspace root resolution:
    //   1. AGENTIC_WORKSPACE_ROOT env var (override for `cargo tauri dev`,
    //      where cwd is the tauri crate dir, not the user's target repo).
    //   2. process cwd at IPC time.
    let ws_root = match std::env::var_os("AGENTIC_WORKSPACE_ROOT") {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().map_err(|e| format!("cwd: {e}"))?,
    };
    if !ws_root.is_dir() {
        return Err(format!(
            "workspace root is not a directory: {}",
            ws_root.display()
        ));
    }
    let ws_id = stable_workspace_id(&ws_root);
    let run_id = Ulid::new().to_string().to_lowercase();

    let db = (*db_state).clone();
    let runs_repo = RunRepo::new(&db);
    let ws_repo = WorkspaceRepo::new(&db);

    // Seed workspace + run rows synchronously so the IPC errors out cleanly
    // on FK / UNIQUE failures rather than vanishing in the spawn.
    ws_repo
        .insert_if_absent(Workspace {
            id: ws_id.clone(),
            name: "ticket-ws".to_string(),
            root_path: ws_root.to_string_lossy().to_string(),
            remote_url: None,
            profile: "custom".to_string(),
            created_at: 0,
            last_opened: 0,
        })
        .map_err(|e| format!("seed workspace: {e}"))?;

    runs_repo
        .insert(Run {
            id: run_id.clone(),
            workspace_id: ws_id.clone(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Pending,
            ticket_type: Some("free-text".to_string()),
            ticket_ref: None,
            ticket_title: None,
            ticket_body: Some(ticket.clone()),
            backend: backend_kind.id_str().to_string(),
            model: model.clone().unwrap_or_else(|| "default".to_string()),
            started_at: 0,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })
        .map_err(|e| format!("seed run: {e}"))?;

    // Use the Tauri-managed bus so subscribe_events forwards envelopes into
    // the webview live. We need an owned EventBus to pass into
    // execute_pipeline; clone the inner one out of the Arc.
    //
    // The PipelineOrchestrator that projects status back into runs+steps
    // is spawned ONCE at app startup (main.rs::setup). Spawning per-run
    // here previously caused two orchestrators to race on the same
    // RunStarted event after a second /plan invocation, producing
    // `invalid state transition from "running" to "running"`.
    let bus: EventBus = (*bus_state.bus).clone();
    let returned_run_id = run_id.clone();

    // Resolve Paths for execute_pipeline (it needs the data dir for various
    // sub-flows like file snapshots).
    let paths = Paths::from_os().map_err(|e| format!("resolve paths: {e}"))?;

    let pipeline_config = PipelineConfig::load(&ws_root)
        .map_err(|e| format!("load pipeline config from {}: {e}", ws_root.display()))?;
    let pipeline = pipeline_config.default_pipeline().clone();

    let model_id = model.map(ModelId);

    let factory: BackendFactoryArc = match backend_kind {
        BackendKind::ClaudeCode => Arc::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(ClaudeCodeBackend::from_env())
        }),
        BackendKind::CopilotCli => Arc::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(CopilotCliBackend::from_env())
        }),
    };

    // Detach the pipeline. Errors surface to the webview as RunComplete(failed).
    let run_id_owned = run_id.clone();
    let ticket_owned = ticket.clone();
    let ws_id_owned = ws_id.clone();
    let ws_root_owned = ws_root.clone();
    let db_owned: Db = db.clone();
    tokio::spawn(async move {
        let factory_fn: agentic_cli::ticket_run::BackendFactory<'_> =
            Box::new(move |step| factory(step));
        let result = execute_pipeline(
            PipelineRunContext {
                db: &db_owned,
                bus: &bus,
                run_id: &run_id_owned,
                ws_id: &ws_id_owned,
                ws_root: &ws_root_owned,
                ticket_text: &ticket_owned,
                model_override: model_id,
                paths: &paths,
            },
            &pipeline,
            factory_fn,
        )
        .await;
        if let Err(e) = result {
            tracing::warn!(
                error = %e,
                run_id = %run_id_owned,
                "start_ticket_run: pipeline execution failed; \
                 RunComplete(failed) was published to the bus",
            );
        }
    });

    Ok(returned_run_id)
}
