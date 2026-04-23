#![deny(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "agentic-cli", about = "Smoke-test binary for the Agentic pipeline core")]
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
    use agentic_core::Paths;

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

async fn cmd_run(_paths: &agentic_core::Paths, _script_path: &std::path::Path) -> Result<()> {
    unimplemented!("cmd_run: Step 5.1 GREEN will fill this in")
}

async fn cmd_doctor() -> Result<()> {
    println!("doctor: Step 5.2 will implement environment probes");
    Ok(())
}

async fn cmd_migrate(_paths: &agentic_core::Paths) -> Result<()> {
    unimplemented!("cmd_migrate: Step 5.1 GREEN will fill this in")
}
