//! Clap CLI types shared between `main.rs` and integration tests.
//!
//! Kept in `lib.rs` (re-exported here) so that `tests/cli_args.rs` can
//! import them without reaching into the binary's private namespace.

use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand, ValueEnum};

/// Thin clap wrapper around `agentic_core::BackendKind`.
///
/// Keeps `clap::ValueEnum` out of `agentic-core` (which has no clap
/// dependency). Conversion to the canonical core type is via `From`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum BackendKind {
    #[value(name = "claude-code")]
    ClaudeCode,
    #[value(name = "copilot-cli")]
    CopilotCli,
}

impl BackendKind {
    pub fn id_str(self) -> &'static str {
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
pub struct Cli {
    /// Override the data directory (default: OS config dir).
    /// Used primarily for test isolation.
    #[arg(long, global = true)]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
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

        /// Comma-separated agent names in pipeline order (requires --ticket).
        /// Example: --agents architect,tdd-developer,qa,reviewer
        #[arg(long, value_delimiter = ',', requires = "ticket")]
        agents: Vec<String>,
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
