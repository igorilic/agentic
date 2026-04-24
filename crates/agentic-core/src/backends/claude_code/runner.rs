//! Subprocess runner for the Claude Code CLI.
//!
//! Spawns `claude -p --output-format stream-json ...` (or any binary injected
//! via `CLAUDE_CODE_BIN`) with configurable CWD, environment variables, and
//! stdin piping.  Cancellation is handled via a [`CancellationToken`]:
//!
//!   1. On cancel: send **SIGTERM** (Unix only) and wait up to `grace_duration`.
//!   2. If still alive after grace: send **SIGKILL** and wait 200ms more.
//!
//! The runner collects stdout as `Vec<String>` (one entry per non-empty line).
//! Parser integration (step 6.3) will wire this into the stream parser.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::error::{CoreError, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of a single `ClaudeRunner::run` call.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// Lines collected from the subprocess's stdout (empty lines excluded).
    pub stdout_lines: Vec<String>,
    /// Exit code, or `None` if the process was killed before it could exit.
    pub exit_code: Option<i32>,
    /// `true` when the run was terminated via the [`CancellationToken`].
    pub was_cancelled: bool,
}

/// Spawns and manages a Claude Code subprocess.
#[derive(Debug, Clone)]
pub struct ClaudeRunner {
    binary: PathBuf,
    /// How long to wait after SIGTERM before escalating to SIGKILL (Unix).
    grace_duration: Duration,
}

impl ClaudeRunner {
    /// Construct from `CLAUDE_CODE_BIN` env var, falling back to `"claude"`.
    pub fn from_env() -> Self {
        let binary = std::env::var("CLAUDE_CODE_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("claude"));
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
        let mut cmd = Command::new(&self.binary);
        cmd.args(&args)
            .current_dir(&cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        for (k, v) in &env {
            cmd.env(k, v);
        }

        // On Unix, put the child in its own process group so that signals
        // sent to the group reach all descendants (e.g., sub-shells and `sleep`).
        #[cfg(unix)]
        cmd.process_group(0);

        let mut child = cmd.spawn().map_err(|e| CoreError::Backend(e.to_string()))?;

        // Write stdin and close the pipe so the subprocess sees EOF.
        if let Some(mut stdin_pipe) = child.stdin.take() {
            stdin_pipe
                .write_all(&stdin_bytes)
                .await
                .map_err(|e| CoreError::Backend(e.to_string()))?;
            // Dropping closes the pipe → subprocess sees EOF
        }

        // Collect stdout asynchronously in a background task
        let stdout_handle = {
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| CoreError::Backend("no stdout handle".to_string()))?;
            tokio::spawn(async move {
                use tokio::io::{AsyncBufReadExt, BufReader};
                let mut lines = BufReader::new(stdout).lines();
                let mut collected = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.is_empty() {
                        collected.push(line);
                    }
                }
                collected
            })
        };

        // Wait for either child exit or cancellation
        let grace = self.grace_duration;

        let (exit_code, was_cancelled) = tokio::select! {
            status = child.wait() => {
                let code = status.ok().and_then(|s| s.code());
                (code, false)
            },
            _ = cancel.cancelled() => {
                terminate_child(&mut child, grace).await;
                (None, true)
            },
        };

        // Collect whatever stdout was produced before termination
        let stdout_lines = stdout_handle.await.unwrap_or_default();

        Ok(RunOutcome {
            stdout_lines,
            exit_code,
            was_cancelled,
        })
    }
}

// ---------------------------------------------------------------------------
// Signal helpers (Unix only)
// ---------------------------------------------------------------------------

/// Attempt a graceful SIGTERM → SIGKILL escalation.
///
/// On non-Unix platforms this is a no-op (the child is expected to have already
/// exited or will be killed by the OS).
#[cfg(unix)]
async fn terminate_child(child: &mut tokio::process::Child, grace: Duration) {
    use nix::sys::signal::{Signal, killpg};
    use nix::unistd::Pid;

    // Capture PGID once before any wait() call — child.id() may return None
    // after the child is reaped inside the grace-period timeout.
    let pgid = child.id().map(|p| Pid::from_raw(p as i32));

    if let Some(pgid) = pgid {
        // Kill the entire process group so sub-shells and their children
        // (e.g., `sleep`) also receive the signal.
        let _ = killpg(pgid, Signal::SIGTERM);
    } else {
        // Child already gone — nothing to signal.
        return;
    }

    // Wait up to `grace` for voluntary exit after SIGTERM
    let exited = tokio::time::timeout(grace, child.wait()).await;

    if exited.is_err() {
        // Grace period expired — escalate to SIGKILL on the whole group.
        // Re-use the PGID captured before the timeout (child.id() may be None now).
        if let Some(pgid) = pgid {
            let _ = killpg(pgid, Signal::SIGKILL);
        }
        let _ = child.start_kill(); // also send to direct child just in case
        // Give it 200ms to actually die after SIGKILL (kernel-level, very fast)
        let _ = tokio::time::timeout(Duration::from_millis(200), child.wait()).await;
    }
}

#[cfg(not(unix))]
async fn terminate_child(child: &mut tokio::process::Child, _grace: Duration) {
    // On non-Unix platforms, TerminateProcess (via start_kill) is the only option.
    let _ = child.start_kill();
    let _ = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
}
