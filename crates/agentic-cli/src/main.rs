#![deny(unsafe_code)]

use agentic_cli::doctor::{SystemWhichProbe, run_doctor};
use std::path::PathBuf;
use std::process::ExitCode;

use agentic_core::{
    Backend, BackendId, Db, Event, EventBus, EventEnvelope, EventPersister, ExecuteRequest,
    ModelId, Paths, PipelineOrchestrator, ProfileId, Run, RunId, RunRepo, RunStatus,
    ScriptedBackend, Step, StepId, StepRepo, StepStatus, TicketKind, TicketRef, WorkspaceRef,
};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;

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
    /// Run a scripted pipeline: load a JSON array of Events from `path` and
    /// replay them through the orchestrator + persister, printing each
    /// envelope as JSON-per-line to stdout.
    Run {
        /// Path to a JSON file containing a `[Event, Event, ...]` array.
        #[arg(long)]
        scripted: PathBuf,
    },
    /// Probe the environment for required tools. (Stub at Step 5.1;
    /// implemented in Step 5.2.)
    Doctor,
    /// Ensure the database is initialized (runs pending migrations).
    /// Useful for first-time setup on a fresh install.
    Migrate,
}

#[tokio::main]
async fn main() -> ExitCode {
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
        Some(dir) => Paths::for_tests(&dir),
        None => Paths::from_os().context("resolve OS data directory")?,
    };
    paths
        .ensure_dirs()
        .context("ensure data directory exists")?;

    match cli.command {
        Command::Run { scripted } => cmd_run(&paths, &scripted).await,
        Command::Doctor => cmd_doctor().await,
        Command::Migrate => cmd_migrate(&paths).await,
    }
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

    // Spawn orchestrator + persister.
    let orch_handle =
        PipelineOrchestrator::spawn(bus.clone(), runs_repo.clone(), steps_repo.clone());
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // JSON-stdout printer: subscribe + print each envelope as a single line.
    let mut printer_rx = bus.subscribe();
    let printer_handle = tokio::spawn(async move {
        while let Ok(envelope) = printer_rx.recv().await {
            match serde_json::to_string(&envelope) {
                Ok(line) => println!("{line}"),
                Err(e) => eprintln!("printer: serialize failed: {e}"),
            }
        }
    });

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
    printer_handle.abort();
    Ok(())
}

async fn cmd_doctor() -> Result<()> {
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
    use rusqlite::params;
    // Workspace row (stream_events has no FK to workspaces, but runs.workspace_id does).
    let conn = db.conn().context("get conn for seeding workspace")?;
    conn.execute(
        "INSERT OR IGNORE INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES ('smoke-ws', 'smoke', '/tmp/smoke', 'custom', 0, 0)",
        params![],
    )?;
    // Run (ignore if already present from prior smoke run reusing the same DB path).
    let run_exists = runs.get("smoke-run")?.is_some();
    if !run_exists {
        runs.insert(Run {
            id: "smoke-run".to_string(),
            workspace_id: "smoke-ws".to_string(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Running,
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
