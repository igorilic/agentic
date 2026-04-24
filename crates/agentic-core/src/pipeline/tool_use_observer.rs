//! Per-step observer that drives [`FileSnapshotter`] from `Edit`/`Write` tool-use events.
//!
//! [`ToolUseObserver`] subscribes to the [`EventBus`], filters for
//! `Event::ToolUseStart` events whose `tool_name` is `"Edit"` or `"Write"`,
//! and captures the before-state of the referenced file via
//! [`FileSnapshotter::capture`]. After the step completes, call
//! [`ToolUseObserverHandle::finalize_into`] to compute diffs, emit
//! `Event::FileChange` events, and write `file_changes.diff`.

use std::path::{Path, PathBuf};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::backends::EventSink;
use crate::backends::file_snapshots::{FileSnapshotter, FinalizeReport};
use crate::events::{Event, EventBus, EventEnvelope};

/// Handle returned by [`ToolUseObserver::spawn`]. Used to finalize the
/// observer after the step's backend has returned.
pub struct ToolUseObserverHandle {
    done_rx: tokio::sync::oneshot::Receiver<FileSnapshotter>,
    join: JoinHandle<()>,
}

impl ToolUseObserverHandle {
    /// Cancel the observer loop, drain the final snapshotter, run
    /// [`FileSnapshotter::finalize`], and return the [`FinalizeReport`].
    pub async fn finalize_into(
        self,
        diff_path: &Path,
        sink: &EventSink,
        run_id: &str,
        step_id: &str,
    ) -> std::io::Result<FinalizeReport> {
        // The stop CancellationToken was already cancelled by the caller before
        // calling finalize_into. Wait for the task to finish sending the snapshotter
        // via done_rx.
        let _ = self.join.await;

        // Receive the snapshotter from the task. If the sender was dropped (e.g.
        // task panicked), create an empty snapshotter as a safe fallback.
        let snapshotter = self.done_rx.await.unwrap_or_else(|_| {
            tracing::warn!(
                "ToolUseObserver: done_rx dropped before sending snapshotter; using empty fallback"
            );
            FileSnapshotter::new(PathBuf::new())
        });

        snapshotter.finalize(diff_path, sink, run_id, Some(step_id))
    }
}

/// Spawns a per-step observer task on the [`EventBus`].
pub struct ToolUseObserver;

impl ToolUseObserver {
    /// Spawn an observer for the given `(run_id, step_id)` pair.
    ///
    /// The returned handle MUST be finalized (via [`ToolUseObserverHandle::finalize_into`])
    /// after the step's backend has returned, or on any early-exit path.
    pub fn spawn(
        bus: &EventBus,
        _run_id: String,
        step_id: String,
        ws_root: PathBuf,
        stop: CancellationToken,
    ) -> ToolUseObserverHandle {
        let mut rx = bus.subscribe();
        let snapshotter = FileSnapshotter::new(ws_root.clone());
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();

        let join = tokio::spawn(async move {
            let mut snapshotter = snapshotter;
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    res = rx.recv() => {
                        match res {
                            Ok(envelope) => {
                                handle_envelope(&mut snapshotter, &envelope, &step_id, &ws_root);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                }
            }
            let _ = done_tx.send(snapshotter);
        });

        ToolUseObserverHandle { done_rx, join }
    }
}

/// Process one bus envelope: capture the before-state if this is an
/// `Edit`/`Write` ToolUseStart for the observed step.
fn handle_envelope(
    snapshotter: &mut FileSnapshotter,
    envelope: &EventEnvelope,
    step_id: &str,
    ws_root: &Path,
) {
    // Filter by step_id.
    if envelope.step_id.as_deref() != Some(step_id) {
        return;
    }

    // Only act on ToolUseStart for Edit or Write tools.
    let (tool_name, input) = match &envelope.event {
        Event::ToolUseStart {
            tool_name, input, ..
        } => (tool_name, input),
        _ => return,
    };

    if tool_name != "Edit" && tool_name != "Write" {
        return;
    }

    // Extract file_path from the tool input JSON.
    let file_path_str = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            tracing::warn!(
                step_id = %step_id,
                tool_name = %tool_name,
                "ToolUseObserver: tool input missing 'file_path' key — skipping capture"
            );
            return;
        }
    };

    // Resolve path: absolute paths are used as-is; relative paths are joined
    // onto ws_root.
    let path = Path::new(file_path_str);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        ws_root.join(path)
    };

    // Capture the before-state. On I/O error, warn and continue.
    if let Err(e) = snapshotter.capture(&resolved) {
        tracing::warn!(
            step_id = %step_id,
            path = %resolved.display(),
            error = %e,
            "ToolUseObserver: capture failed — skipping"
        );
    }
}
