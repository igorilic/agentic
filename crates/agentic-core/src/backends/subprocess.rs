//! Shared subprocess spawning utilities used by both the Claude Code and
//! Copilot CLI backend runners.
//!
//! This module provides the common scaffolding — process group setup, stdin
//! writing, stderr draining, cancellation with SIGTERM → SIGKILL escalation —
//! so that individual runners only need to build their binary-specific argv.

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

/// Outcome returned by the wait handle of a streaming run.
#[derive(Debug)]
pub struct WaitOutcome {
    /// Process exit code, or `None` if the process was killed before exiting.
    pub exit_code: Option<i32>,
    /// `true` when the run was terminated via the [`CancellationToken`].
    pub was_cancelled: bool,
    /// Last ≤64KB of text written by the subprocess to stderr.
    /// Empty when no stderr was produced.
    pub stderr_tail: String,
}

/// A live subprocess with an exposed stdout reader.
pub struct StreamingRun {
    /// Live stdout from the subprocess — pipe open until the subprocess exits.
    pub stdout: tokio::process::ChildStdout,
    /// Background task that handles cancellation/signal escalation and waits
    /// for the subprocess to exit.
    pub wait_handle: tokio::task::JoinHandle<Result<WaitOutcome>>,
}

/// Result of a fully-buffered subprocess run.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// Lines collected from the subprocess's stdout (empty lines excluded).
    pub stdout_lines: Vec<String>,
    /// Exit code, or `None` if the process was killed before it could exit.
    pub exit_code: Option<i32>,
    /// `true` when the run was terminated via the [`CancellationToken`].
    pub was_cancelled: bool,
    /// Last ≤64KB of text written by the subprocess to stderr.
    /// Empty when no stderr was produced.
    pub stderr_tail: String,
}

// ---------------------------------------------------------------------------
// Internal: bounded stderr drain task
// ---------------------------------------------------------------------------

/// Spawn a task that drains `stderr`, logs each line via `tracing::warn!`,
/// and returns the captured bytes bounded at `MAX_BYTES` (64 KB).
fn spawn_stderr_drainer(stderr: tokio::process::ChildStderr) -> tokio::task::JoinHandle<String> {
    tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};

        const MAX_BYTES: usize = 64 * 1024;
        let mut buf: Vec<u8> = Vec::with_capacity(4 * 1024);
        let mut lines = BufReader::new(stderr).lines();

        while let Ok(Some(line)) = lines.next_line().await {
            tracing::warn!(stderr_line = %line, "subprocess stderr");
            if buf.len() < MAX_BYTES {
                buf.extend_from_slice(line.as_bytes());
                buf.push(b'\n');
            }
            // Else silently drop — bound the buffer.
        }

        String::from_utf8_lossy(&buf).into_owned()
    })
}

// ---------------------------------------------------------------------------
// Spawn helpers
// ---------------------------------------------------------------------------

/// Spawn a subprocess and return a live stdout reader plus a wait handle.
///
/// Sets up process group (Unix), writes `stdin_bytes` via a concurrent task,
/// drains stderr into a 64KB-bounded buffer (each line forwarded to
/// `tracing::warn!`), and starts a background task that handles cancellation
/// with SIGTERM → SIGKILL escalation.
///
/// # Errors
/// Returns `Err` if the subprocess cannot be spawned.
pub fn spawn_streaming(
    binary: &PathBuf,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: &PathBuf,
    stdin_bytes: Vec<u8>,
    cancel: CancellationToken,
    grace: Duration,
) -> Result<StreamingRun> {
    let mut cmd = Command::new(binary);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    for (k, v) in env {
        cmd.env(k, v);
    }

    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = cmd.spawn().map_err(|e| CoreError::Backend(e.to_string()))?;

    // Take stdout before spawning tasks (must happen synchronously).
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| CoreError::Backend("no stdout handle".to_string()))?;

    // Spawn task: write stdin bytes concurrently with stdout/stderr reads.
    // This prevents deadlock when stdin is large and stdout pipe buffer fills up.
    if let Some(mut stdin_pipe) = child.stdin.take() {
        tokio::spawn(async move {
            if !stdin_bytes.is_empty() && stdin_pipe.write_all(&stdin_bytes).await.is_err() {
                tracing::warn!("subprocess stdin write failed");
            }
            // Drop closes the pipe — subprocess sees EOF.
            drop(stdin_pipe);
        });
    }

    // Spawn task: drain stderr into a bounded buffer, log each line.
    let stderr_handle = child.stderr.take().map(spawn_stderr_drainer);

    // Spawn wait task: handles cancellation + signal escalation.
    let wait_handle = tokio::spawn(async move {
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
        let stderr_tail = match stderr_handle {
            Some(handle) => handle.await.unwrap_or_default(),
            None => String::new(),
        };
        Ok(WaitOutcome {
            exit_code,
            was_cancelled,
            stderr_tail,
        })
    });

    Ok(StreamingRun {
        stdout,
        wait_handle,
    })
}

/// Spawn a subprocess, pipe `stdin_bytes`, and buffer all stdout lines.
///
/// Stdin is written via a concurrent task so that large stdin + large stdout
/// cannot deadlock on the kernel pipe buffer.
///
/// Returns once the subprocess exits (or is cancelled).
pub async fn spawn_buffered(
    binary: &PathBuf,
    args: &[String],
    env: &HashMap<String, String>,
    cwd: &PathBuf,
    stdin_bytes: Vec<u8>,
    cancel: CancellationToken,
    grace: Duration,
) -> Result<RunOutcome> {
    let mut cmd = Command::new(binary);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    for (k, v) in env {
        cmd.env(k, v);
    }

    // On Unix, put the child in its own process group so that signals
    // sent to the group reach all descendants (e.g., sub-shells and `sleep`).
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = cmd.spawn().map_err(|e| CoreError::Backend(e.to_string()))?;

    // Spawn task: write stdin concurrently with stdout/stderr reads.
    // Prevents deadlock when stdin is large and stdout pipe buffer fills up.
    let stdin_handle = child.stdin.take().map(|mut stdin_pipe| {
        tokio::spawn(async move {
            if !stdin_bytes.is_empty() && stdin_pipe.write_all(&stdin_bytes).await.is_err() {
                tracing::warn!("subprocess stdin write failed");
            }
            // Drop closes the pipe — subprocess sees EOF.
            drop(stdin_pipe);
        })
    });

    // Collect stdout asynchronously in a background task.
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

    // Drain stderr into a bounded buffer concurrently.
    let stderr_handle = child.stderr.take().map(spawn_stderr_drainer);

    // Wait for either child exit or cancellation.
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

    // Collect whatever stdout and stderr were produced before termination.
    let stdout_lines = stdout_handle.await.unwrap_or_default();
    let stderr_tail = match stderr_handle {
        Some(handle) => handle.await.unwrap_or_default(),
        None => String::new(),
    };

    // Ensure stdin writer is done (it will be if stdin fit in the pipe, which
    // it already must since the subprocess has exited at this point).
    if let Some(handle) = stdin_handle {
        let _ = handle.await;
    }

    Ok(RunOutcome {
        stdout_lines,
        exit_code,
        was_cancelled,
        stderr_tail,
    })
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

    // Wait up to `grace` for voluntary exit after SIGTERM.
    let exited = tokio::time::timeout(grace, child.wait()).await;

    if exited.is_err() {
        // Grace period expired — escalate to SIGKILL on the whole group.
        if let Some(pgid) = pgid {
            let _ = killpg(pgid, Signal::SIGKILL);
        }
        let _ = child.start_kill(); // also send to direct child just in case
        // Give it 200ms to actually die after SIGKILL (kernel-level, very fast).
        let _ = tokio::time::timeout(Duration::from_millis(200), child.wait()).await;
    }
}

#[cfg(not(unix))]
async fn terminate_child(child: &mut tokio::process::Child, _grace: Duration) {
    let _ = child.start_kill();
    let _ = tokio::time::timeout(Duration::from_millis(500), child.wait()).await;
}
