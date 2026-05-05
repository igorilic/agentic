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
    Backend, BackendKind, ClaudeCodeBackend, CopilotCliBackend, Db, ModelId, Paths, PipelineStep,
    Run, RunRepo,
};
use tauri::State;
use ulid::Ulid;

use super::events::EventBusState;
use super::workspace::resolve_workspace_root;

// BackendKind is the canonical `agentic_core::BackendKind`. It serialises
// as kebab-case strings (e.g. "claude-code") which matches the Tauri IPC
// contract. The local definition has been removed.

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

    // Workspace root resolution: honours AGENTIC_WORKSPACE_ROOT env var
    // (for `cargo tauri dev`), then falls back to process cwd.
    let ws_root = resolve_workspace_root()?;

    // Pre-flight: fail fast with an actionable message if the backend
    // binary or any required agent file is missing. Without this, the
    // pipeline starts, the first claude/copilot subprocess crashes (or
    // discover_agent fails on the first step), and the user sees a
    // cryptic RunComplete(failed) buried in EventList. The chat IPC
    // surfaces a clear "install …" / "run agentic-cli init" message.
    //
    // TODO(I.5): replace this hardcoded list with agents from the IPC payload.
    let default_agents: Vec<String> = ["architect", "tdd-developer", "qa", "reviewer"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    pre_flight_check(&ws_root, &backend_kind, &default_agents)?;

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

    // Register a CancellationToken in the same registry the existing
    // cancel_run IPC reads from. Without this, the Cancel button in the
    // UI is a no-op for chat-driven ticket runs. The token is plumbed into
    // every per-step ExecuteRequest via execute_pipeline's external_cancel
    // field, so a cancel_run propagates all the way to the running claude
    // subprocess (which the backend SIGTERMs via req.cancel).
    let cancel_token = tokio_util::sync::CancellationToken::new();
    bus_state
        .register_cancellation(run_id.clone(), cancel_token.clone())
        .await;

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
                backend_kind,
                external_cancel: Some(cancel_token.clone()),
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

/// Pre-flight check called before seeding rows or spawning the pipeline.
///
/// Validates two things that have caused real "pipeline succeeded but
/// nothing happened" reports:
/// 1. The backend binary (`claude` for ClaudeCode, `copilot` for
///    CopilotCli) is on PATH. Honors the same env-var overrides the
///    backend runners use (`CLAUDE_CODE_BIN`, `COPILOT_CLI_BIN`) so
///    integration tests can point at a missing path.
/// 2. All agents in `agents` are discoverable from `ws_root` using the
///    same search order the pipeline itself uses (`.agentic/agents`,
///    `.claude/agents`, `.github/agents`, …).
///
/// Errors are short, actionable strings the chat surfaces verbatim.
fn pre_flight_check(
    ws_root: &std::path::Path,
    backend_kind: &BackendKind,
    agents: &[String],
) -> Result<(), String> {
    // Resolve the home directory once and delegate to the injectable variant.
    let base = directories::BaseDirs::new();
    let home = base.as_ref().map(|b| b.home_dir());
    pre_flight_check_with_home(ws_root, backend_kind, home, agents)
}

/// Same as [`pre_flight_check`] but accepts an injectable home directory so
/// tests can avoid seeing real agent files installed in `~/.claude/` or
/// `~/.copilot/`.
pub fn pre_flight_check_with_home(
    ws_root: &std::path::Path,
    backend_kind: &BackendKind,
    home: Option<&std::path::Path>,
    agents: &[String],
) -> Result<(), String> {
    // 0. Reject empty agent list immediately with an actionable error.
    if agents.is_empty() {
        return Err(
            "pre-flight: agents list is empty — pass at least one agent in \
             start_ticket_run.agents"
                .to_string(),
        );
    }

    // 1. Backend binary on PATH.
    let (binary_env, binary_default, install_hint) = match backend_kind {
        BackendKind::ClaudeCode => (
            "CLAUDE_CODE_BIN",
            "claude",
            "Install Claude Code from https://docs.claude.com/claude-code and run `claude /login`",
        ),
        BackendKind::CopilotCli => (
            "COPILOT_CLI_BIN",
            "copilot",
            "Install GitHub Copilot CLI: https://github.com/github/copilot-cli",
        ),
    };
    let binary = std::env::var(binary_env).unwrap_or_else(|_| binary_default.to_string());
    if !is_binary_resolvable(&binary) {
        return Err(format!(
            "pre-flight: `{binary}` not found on PATH. {install_hint}."
        ));
    }

    // 2. Agent files. Use the same discovery the pipeline does.
    for agent_name in agents {
        match agentic_core::discover_agent_with_home(*backend_kind, ws_root, home, agent_name) {
            Ok(_) => {}
            Err(agentic_core::CoreError::AgentNotFound { name, searched }) => {
                return Err(format_agent_not_found_error(
                    &name,
                    *backend_kind,
                    &searched,
                ));
            }
            Err(e) => {
                return Err(format!(
                    "pre-flight: agent '{agent_name}' in agents/ could not be parsed: {e}. \
                     Run `agentic-cli init` to re-scaffold the required agents."
                ));
            }
        }
    }

    Ok(())
}

/// Format a user-facing error string for a missing agent.
///
/// Lists each searched path as a bullet so the user knows exactly where to
/// place the file. Includes a backend-specific `agentic-cli init` hint.
fn format_agent_not_found_error(
    name: &str,
    backend: BackendKind,
    searched: &[std::path::PathBuf],
) -> String {
    let bullets: String = searched
        .iter()
        .map(|p| format!("  - {}\n", p.display()))
        .collect();
    let flag = match backend {
        BackendKind::CopilotCli => " --copilot",
        BackendKind::ClaudeCode => "",
    };
    let backend_label = match backend {
        BackendKind::ClaudeCode => "claude-code",
        BackendKind::CopilotCli => "copilot-cli",
    };
    format!(
        "pre-flight: agent '{name}' not found for backend '{backend_label}'. Searched:\n\
         {bullets}\
         Run `agentic-cli init{flag}` to scaffold the four required agents."
    )
}

/// Return `true` if `binary` is either an absolute/relative path that exists
/// or a bare name resolvable on the user's PATH.
fn is_binary_resolvable(binary: &str) -> bool {
    let p = std::path::Path::new(binary);
    if p.is_absolute() || binary.contains(std::path::MAIN_SEPARATOR) {
        return p.exists();
    }
    // Bare name — search PATH manually. Avoids pulling the `which` crate
    // for one call site.
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(binary).exists())
}
