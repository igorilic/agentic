//! Subprocess runner for the Copilot CLI.
//!
//! Spawns `copilot -p <text> --output-format json --allow-all-tools ...` (or any
//! binary injected via `COPILOT_CLI_BIN`) with configurable CWD, environment
//! variables, and optional stdin.  Cancellation is handled via a
//! [`CancellationToken`]:
//!
//!   1. On cancel: send **SIGTERM** (Unix only) and wait up to `grace_duration`.
//!   2. If still alive after grace: send **SIGKILL** and wait 200ms more.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::backends::subprocess::{spawn_buffered, spawn_streaming};
use crate::error::Result;

/// Environment variable used to override the copilot binary path.
pub const COPILOT_BIN_ENV_VAR: &str = "COPILOT_CLI_BIN";

// ---------------------------------------------------------------------------
// Re-export shared types under canonical names.
// ---------------------------------------------------------------------------

pub use crate::backends::subprocess::{RunOutcome, StreamingRun, WaitOutcome};

/// Spawns and manages a Copilot CLI subprocess.
#[derive(Debug, Clone)]
pub struct CopilotRunner {
    binary: PathBuf,
    /// How long to wait after SIGTERM before escalating to SIGKILL (Unix).
    grace_duration: Duration,
}

impl CopilotRunner {
    /// Construct from `COPILOT_CLI_BIN` env var, falling back to `"copilot"`.
    pub fn from_env() -> Self {
        let binary = std::env::var(COPILOT_BIN_ENV_VAR)
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("copilot"));
        Self {
            binary,
            grace_duration: Duration::from_secs(5),
        }
    }

    /// Inject an explicit binary path — useful in tests.
    pub fn with_binary(binary: PathBuf) -> Self {
        Self {
            binary,
            grace_duration: Duration::from_secs(5),
        }
    }

    /// Inject binary path AND a custom SIGTERM grace period — useful in tests
    /// to keep signal escalation tests fast.
    pub fn with_binary_and_grace(binary: PathBuf, grace_duration: Duration) -> Self {
        Self {
            binary,
            grace_duration,
        }
    }

    /// Spawn the subprocess and return a live stdout reader plus a wait handle.
    ///
    /// The caller is responsible for draining `StreamingRun::stdout` (e.g. by
    /// passing it to the stream parser) and for awaiting
    /// `StreamingRun::wait_handle` to collect the exit outcome.
    ///
    /// Stderr is drained internally so it never blocks the subprocess.
    ///
    /// # Errors
    /// Returns `Err` if the subprocess cannot be spawned.
    pub fn run_streaming(
        &self,
        args: Vec<String>,
        env: HashMap<String, String>,
        cwd: PathBuf,
        stdin_bytes: Vec<u8>,
        cancel: CancellationToken,
    ) -> Result<StreamingRun> {
        spawn_streaming(
            &self.binary,
            &args,
            &env,
            &cwd,
            stdin_bytes,
            cancel,
            self.grace_duration,
        )
    }

    /// Run the subprocess, pipe `stdin_bytes` into its stdin, collect stdout.
    ///
    /// # Arguments
    /// - `args`        — command-line arguments appended after the binary name.
    /// - `env`         — additional environment variables (merged with process env).
    /// - `cwd`         — working directory for the subprocess.
    /// - `stdin_bytes` — bytes written to the subprocess's stdin before closing it.
    /// - `cancel`      — token that triggers graceful-then-forceful termination.
    pub async fn run(
        &self,
        args: Vec<String>,
        env: HashMap<String, String>,
        cwd: PathBuf,
        stdin_bytes: Vec<u8>,
        cancel: CancellationToken,
    ) -> Result<RunOutcome> {
        spawn_buffered(
            &self.binary,
            &args,
            &env,
            &cwd,
            stdin_bytes,
            cancel,
            self.grace_duration,
        )
        .await
    }
}
