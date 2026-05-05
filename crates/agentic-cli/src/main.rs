#![deny(unsafe_code)]

use agentic_cli::doctor::{SystemWhichProbe, run_doctor};
use agentic_cli::ticket_run::{
    BackendFactory, PipelineRunContext, execute_pipeline, stable_workspace_id,
};
use std::path::PathBuf;
use std::process::ExitCode;

use agentic_core::permissions::gate_async::AsyncGate;
use agentic_core::{
    Backend, BackendId, ClaudeCodeBackend, CopilotCliBackend, Db, Event, EventBus, EventEnvelope,
    EventPersister, ExecuteRequest, ModelId, Paths, PipelineConfig, PipelineOrchestrator,
    PipelineStep, ProfileId, Run, RunId, RunRepo, RunStatus, ScriptedBackend, Step, StepId,
    StepRepo, StepStatus, TicketKind, TicketRef, Workspace, WorkspaceRef, WorkspaceRepo,
};
use anyhow::{Context, Result};
use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use tokio_util::sync::CancellationToken;

/// Thin clap wrapper around `agentic_core::BackendKind`.
///
/// Keeps `clap::ValueEnum` out of `agentic-core` (which has no clap
/// dependency). Conversion to the canonical core type is via `From`.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum BackendKind {
    #[value(name = "claude-code")]
    ClaudeCode,
    #[value(name = "copilot-cli")]
    CopilotCli,
}

impl BackendKind {
    fn id_str(self) -> &'static str {
        agentic_core::BackendKind::from(self).id_str()
    }
}

impl From<BackendKind> for agentic_core::BackendKind {
    fn from(cli: BackendKind) -> Self {
        match cli {
            BackendKind::ClaudeCode => agentic_core::BackendKind::ClaudeCode,
            BackendKind::CopilotCli => agentic_core::BackendKind::CopilotCli,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "agentic-cli",
    about = "Smoke-test binary for the Agentic pipeline core"
)]
struct Cli {
    /// Override the data directory (default: OS config dir).
    /// Used primarily for test isolation.
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a pipeline. Use --scripted for a JSON event replay, or --ticket
    /// for a ticket-driven run using the claude-code backend.
    #[command(group(
        ArgGroup::new("source")
            .required(true)
            .multiple(false)
            .args(["scripted", "ticket"])
    ))]
    Run {
        /// Path to a JSON file containing a `[Event, Event, ...]` array.
        #[arg(long)]
        scripted: Option<PathBuf>,

        /// Free-text ticket description to drive the default pipeline.
        #[arg(long)]
        ticket: Option<String>,

        /// Override the model for all pipeline steps (requires --ticket).
        #[arg(long, requires = "ticket")]
        model: Option<String>,

        /// Which backend to invoke for ticket-driven runs. Default: claude-code.
        /// Only valid with --ticket.
        #[arg(long, value_enum, default_value_t = BackendKind::ClaudeCode, requires = "ticket")]
        backend: BackendKind,
    },
    /// Probe the environment for required tools. (Stub at Step 5.1;
    /// implemented in Step 5.2.)
    Doctor,
    /// Ensure the database is initialized (runs pending migrations).
    /// Useful for first-time setup on a fresh install.
    Migrate,
    /// Scaffold the four required agent files
    /// (architect, tdd-developer, qa, reviewer) into a directory the
    /// pipeline can discover. Default destination: `<cwd>/.claude/agents/`
    /// (reuses Claude Code's project-local convention so the same files
    /// drive both Agentic and Claude Code subagents). Use `--copilot` for
    /// `.github/agents/` instead, `--global` for the corresponding
    /// `$HOME/.{claude,copilot}/agents/` location, and `--agentic` for
    /// the explicit `.agentic/agents/` override.
    Init {
        /// Target repo root. Defaults to the current working directory.
        /// Ignored when `--global` is set.
        #[arg(long)]
        target: Option<PathBuf>,
        /// Use Copilot's convention: `.github/agents/` (or
        /// `$HOME/.copilot/agents/` with `--global`). Default is the
        /// Claude convention.
        #[arg(long)]
        copilot: bool,
        /// Write to `$HOME` instead of the repo. Combined with `--copilot`
        /// targets `$HOME/.copilot/agents/`; otherwise `$HOME/.claude/agents/`.
        #[arg(long)]
        global: bool,
        /// Use Agentic's explicit project override (`.agentic/agents/`)
        /// instead of the Claude/Copilot defaults. Useful when you want
        /// agents the underlying CLI tools don't see. Mutually exclusive
        /// with `--copilot` and `--global`.
        #[arg(long, conflicts_with_all = ["copilot", "global"])]
        agentic: bool,
        /// Overwrite agent files that already exist.
        #[arg(long)]
        force: bool,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match run_command(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(2)
        }
    }
}

async fn run_command(cli: Cli) -> Result<()> {
    let paths = match cli.data_dir {
        Some(dir) => Paths::with_root(&dir),
        None => Paths::from_os().context("resolve OS data directory")?,
    };
    paths
        .ensure_dirs()
        .context("ensure data directory exists")?;

    match cli.command {
        Command::Run {
            scripted: Some(path),
            ..
        } => cmd_run(&paths, &path).await,
        Command::Run {
            scripted: None,
            ticket: Some(ticket_text),
            model,
            backend,
        } => cmd_run_ticket(&paths, ticket_text, model, backend).await,
        Command::Run { .. } => {
            // clap ArgGroup ensures we can't get here; but the compiler needs exhaustiveness.
            anyhow::bail!("run requires exactly one of --scripted or --ticket")
        }
        Command::Doctor => cmd_doctor(),
        Command::Migrate => cmd_migrate(&paths).await,
        Command::Init {
            target,
            copilot,
            global,
            agentic,
            force,
        } => cmd_init(target.as_deref(), copilot, global, agentic, force),
    }
}

fn cmd_init(
    target: Option<&std::path::Path>,
    copilot: bool,
    global: bool,
    agentic: bool,
    force: bool,
) -> Result<()> {
    use agentic_cli::init::AgentDestination;

    let destination = match (agentic, copilot, global) {
        (true, _, _) => AgentDestination::AgenticRepo,
        (false, true, true) => AgentDestination::CopilotHome,
        (false, true, false) => AgentDestination::CopilotRepo,
        (false, false, true) => AgentDestination::ClaudeHome,
        (false, false, false) => AgentDestination::ClaudeRepo,
    };

    let repo_root = match target {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().context("resolve current working directory")?,
    };
    let home = directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf());
    let agents_dir = destination
        .resolve(&repo_root, home.as_deref())
        .context("resolve agents directory from --copilot/--global flags")?;

    let report = agentic_cli::init::write_agent_scaffolding(&agents_dir, force)
        .with_context(|| format!("scaffold agent files under {}", agents_dir.display()))?;
    println!(
        "Scaffolded {} agent files into {}:",
        report.created.len(),
        report.agents_dir.display()
    );
    for path in &report.created {
        println!("  {}", path.display());
    }
    println!("\nNext: edit each file to fit your project, then run");
    println!("  agentic-cli run --ticket \"<your ticket>\"");
    Ok(())
}

/// Spawns the three background tasks common to all run modes:
/// orchestrator (state machine), persister (event log), and stdout printer.
///
/// Returns `(orch_handle, pers_handle, printer_handle)`. Caller must:
///   1. Drop `bus` to signal shutdown.
///   2. `.await` all three handles for graceful drain — the printer exits
///      naturally when the broadcast channel closes (`RecvError::Closed`).
fn spawn_infra(
    bus: &EventBus,
    db: &Db,
    runs_repo: &RunRepo,
    steps_repo: &StepRepo,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::task::JoinHandle<()>,
    tokio::task::JoinHandle<()>,
) {
    // Load permissions config from the current working directory's permissions.toml,
    // or fall back to the built-in default. The gate is shared across all pipeline
    // steps for this run.
    let permissions_path = std::env::current_dir()
        .unwrap_or_default()
        .join("permissions.toml");
    let permissions_config = agentic_core::PermissionsConfig::load(&permissions_path)
        .unwrap_or_else(|_| agentic_core::PermissionsConfig::builtin_default());
    let gate = std::sync::Arc::new(AsyncGate::new(
        permissions_config,
        bus.clone(),
        std::time::Duration::from_secs(60),
        "agentic-cli".to_string(),
    ));
    let orch_handle =
        PipelineOrchestrator::spawn(bus.clone(), runs_repo.clone(), steps_repo.clone(), gate);
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());
    let mut printer_rx = bus.subscribe();
    let printer_handle = tokio::spawn(async move {
        while let Ok(envelope) = printer_rx.recv().await {
            match serde_json::to_string(&envelope) {
                Ok(line) => println!("{line}"),
                Err(e) => eprintln!("printer: serialize failed: {e}"),
            }
        }
    });
    (orch_handle, pers_handle, printer_handle)
}

async fn cmd_run(paths: &Paths, script_path: &std::path::Path) -> Result<()> {
    // Load the script from JSON file.
    let content = std::fs::read_to_string(script_path)
        .with_context(|| format!("read script file {}", script_path.display()))?;
    let events: Vec<Event> =
        serde_json::from_str(&content).context("parse script JSON as Vec<Event>")?;

    // Infrastructure setup.
    let db = Db::open(paths).context("open sqlite database")?;
    let runs_repo = RunRepo::new(&db);
    let steps_repo = StepRepo::new(&db);
    let bus = EventBus::new();

    // Seed a minimal workspace + run + single step so the orchestrator has
    // rows to mutate. Hardcoded IDs are fine for smoke.
    seed_minimal_run(&db, &runs_repo, &steps_repo)?;

    let (orch_handle, pers_handle, printer_handle) =
        spawn_infra(&bus, &db, &runs_repo, &steps_repo);

    // Publish RunStarted so the event log starts cleanly.
    bus.publish(EventEnvelope::now(
        "smoke-run".to_string(),
        None,
        Event::RunStarted {
            ticket: TicketRef {
                kind: TicketKind::FreeText,
                reference: "cli-smoke".to_string(),
                title: Some("agentic-cli smoke run".to_string()),
            },
            profile: ProfileId("custom".to_string()),
            backend: BackendId("scripted".to_string()),
            model: ModelId("fake".to_string()),
        },
    ));

    // Execute the scripted backend.
    let backend = ScriptedBackend::new(events);
    let req = ExecuteRequest {
        workspace: WorkspaceRef {
            id: "smoke-ws".to_string(),
            root_path: paths.data_dir().to_path_buf(),
        },
        run_id: RunId("smoke-run".to_string()),
        step_id: StepId("smoke-step".to_string()),
        agent_name: "scripted".to_string(),
        agent_prompt: String::new(),
        user_context: String::new(),
        model: None,
        tools: Vec::new(),
        cwd: paths.data_dir().to_path_buf(),
        timeout: None,
        cancel: CancellationToken::new(),
    };
    backend
        .execute(req, bus.sender())
        .await
        .context("scripted backend execute")?;

    // Final RunComplete.
    bus.publish(EventEnvelope::now(
        "smoke-run".to_string(),
        None,
        Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: 0,
            summary: "smoke complete".to_string(),
        },
    ));

    // Drain and shut down cleanly.
    drop(bus);
    orch_handle.await.context("orchestrator task")?;
    pers_handle.await.context("persister task")?;
    printer_handle.await.context("printer task")?;
    Ok(())
}

async fn cmd_run_ticket(
    paths: &Paths,
    ticket_text: String,
    model_override: Option<String>,
    backend_kind: BackendKind,
) -> Result<()> {
    let db = Db::open(paths).context("open sqlite database")?;
    let runs_repo = RunRepo::new(&db);
    let steps_repo = StepRepo::new(&db);
    let ws_repo = WorkspaceRepo::new(&db);
    let bus = EventBus::new();

    // Use the process working directory as the workspace root so agent
    // discovery can find `.agentic/agents/` relative to where the user
    // invoked the CLI.
    let ws_root = std::env::current_dir().context("determine working directory")?;
    // Derive a stable id from the canonical path so re-runs hit the same
    // workspace row via INSERT OR IGNORE instead of leaking orphan rows.
    let ws_id = stable_workspace_id(&ws_root);
    let run_id = ulid::Ulid::new().to_string().to_lowercase();

    // Seed workspace row (idempotent — INSERT OR IGNORE semantics).
    ws_repo.insert_if_absent(Workspace {
        id: ws_id.clone(),
        name: "ticket-ws".to_string(),
        root_path: ws_root.to_string_lossy().to_string(),
        remote_url: None,
        profile: "custom".to_string(),
        created_at: 0,
        last_opened: 0,
    })?;

    // Seed run row as Pending; the orchestrator transitions to Running when
    // execute_pipeline publishes RunStarted (closes GH #17).
    runs_repo.insert(Run {
        id: run_id.clone(),
        workspace_id: ws_id.clone(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Pending,
        ticket_type: Some("free-text".to_string()),
        ticket_ref: None,
        ticket_title: None,
        ticket_body: Some(ticket_text.clone()),
        backend: backend_kind.id_str().to_string(),
        model: model_override
            .clone()
            .unwrap_or_else(|| "default".to_string()),
        started_at: 0,
        completed_at: None,
        duration_ms: None,
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    })?;

    let (orch_handle, pers_handle, printer_handle) =
        spawn_infra(&bus, &db, &runs_repo, &steps_repo);

    // Load default pipeline config (from .agentic/pipeline.toml or built-in).
    let pipeline_config = PipelineConfig::load(&ws_root).context("load pipeline config")?;
    let pipeline = pipeline_config.default_pipeline().clone();

    let model_id = model_override.map(ModelId);

    // Build backend factory based on selected backend kind.
    let factory: BackendFactory<'_> = match backend_kind {
        BackendKind::ClaudeCode => Box::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(ClaudeCodeBackend::from_env())
        }),
        BackendKind::CopilotCli => Box::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(CopilotCliBackend::from_env())
        }),
    };

    let result = execute_pipeline(
        PipelineRunContext {
            db: &db,
            bus: &bus,
            run_id: &run_id,
            ws_id: &ws_id,
            ws_root: &ws_root,
            ticket_text: &ticket_text,
            model_override: model_id,
            paths,
            external_cancel: None,
        },
        &pipeline,
        factory,
    )
    .await;

    // Shut down cleanly.
    drop(bus);
    orch_handle.await.context("orchestrator task")?;
    pers_handle.await.context("persister task")?;
    printer_handle.await.context("printer task")?;

    result.context("pipeline execution failed")
}

fn cmd_doctor() -> Result<()> {
    run_doctor(&SystemWhichProbe, &mut std::io::stdout().lock()).context("doctor probe failed")?;
    Ok(())
}

async fn cmd_migrate(paths: &Paths) -> Result<()> {
    // Db::open runs migrations automatically; invoking it here guarantees a
    // migrate-then-close cycle. Useful for first-install sanity.
    let _db = Db::open(paths).context("run migrations")?;
    println!("migrate: database up to date");
    Ok(())
}

fn seed_minimal_run(db: &Db, runs: &RunRepo, steps: &StepRepo) -> Result<()> {
    // Workspace row (stream_events has no FK to workspaces, but runs.workspace_id does).
    let ws_repo = WorkspaceRepo::new(db);
    ws_repo.insert_if_absent(Workspace {
        id: "smoke-ws".to_string(),
        name: "smoke".to_string(),
        root_path: "/tmp/smoke".to_string(),
        remote_url: None,
        profile: "custom".to_string(),
        created_at: 0,
        last_opened: 0,
    })?;
    // Run (ignore if already present from prior smoke run reusing the same DB path).
    // Seed as Pending; RunStarted event (published after spawn_infra) drives
    // the Pending → Running transition via the orchestrator (closes GH #17).
    let run_exists = runs.get("smoke-run")?.is_some();
    if !run_exists {
        runs.insert(Run {
            id: "smoke-run".to_string(),
            workspace_id: "smoke-ws".to_string(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Pending,
            ticket_type: None,
            ticket_ref: None,
            ticket_title: None,
            ticket_body: None,
            backend: "scripted".to_string(),
            model: "fake".to_string(),
            started_at: 0,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })?;
    }
    // Step.
    let step_exists = steps.get("smoke-step")?.is_some();
    if !step_exists {
        steps.insert(Step {
            id: "smoke-step".to_string(),
            run_id: "smoke-run".to_string(),
            seq: 0,
            agent_name: "scripted".to_string(),
            status: StepStatus::Pending,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            retry_count: 0,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::BackendKind;

    #[test]
    fn backend_kind_id_str_matches_clap_value_name() {
        // id_str delegates to core::BackendKind via From, so this also
        // validates the From conversion is wired correctly.
        assert_eq!(BackendKind::ClaudeCode.id_str(), "claude-code");
        assert_eq!(BackendKind::CopilotCli.id_str(), "copilot-cli");
    }

    #[test]
    fn cli_backend_kind_converts_to_core_backend_kind() {
        use agentic_core::BackendKind as CoreKind;
        assert_eq!(
            CoreKind::from(BackendKind::ClaudeCode),
            CoreKind::ClaudeCode
        );
        assert_eq!(
            CoreKind::from(BackendKind::CopilotCli),
            CoreKind::CopilotCli
        );
    }
}
