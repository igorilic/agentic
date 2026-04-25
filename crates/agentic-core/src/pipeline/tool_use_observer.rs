//! Per-step observer that drives [`FileSnapshotter`] from `Edit`/`Write`/`MultiEdit`
//! tool-use events.
//!
//! [`ToolUseObserver`] subscribes to the [`EventBus`], filters for
//! `Event::ToolUseStart` events whose `tool_name` is `"Edit"`, `"Write"`, or
//! `"MultiEdit"`, and captures the before-state of the referenced file via
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
            FileSnapshotter::new()
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
        run_id: String,
        step_id: String,
        ws_root: PathBuf,
        stop: CancellationToken,
    ) -> ToolUseObserverHandle {
        let mut rx = bus.subscribe();
        let snapshotter = FileSnapshotter::new();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();

        let join = tokio::spawn(async move {
            let mut snapshotter = snapshotter;
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    res = rx.recv() => {
                        match res {
                            Ok(envelope) => {
                                handle_envelope(&mut snapshotter, &envelope, &run_id, &step_id, &ws_root);
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

/// If `tool_name` corresponds to a known file-editing tool across our
/// supported backends, return the path it operates on. Used by the
/// pipeline observer to know which paths to snapshot.
///
/// Coverage:
/// - Claude Code: `Edit`, `Write`, `MultiEdit` with `input.file_path`.
/// - Copilot CLI: `create`, `str_replace` with `input.path`.
///
/// `bash` with shell redirects (e.g., `tee > file`) is intentionally NOT
/// covered; reliable shell-string parsing is too brittle. Files modified
/// only via `bash` will not appear in `file_changes` events. Tracked
/// separately if/when it matters.
pub(crate) fn edited_path_from_tool_use<'a>(
    tool_name: &str,
    input: &'a serde_json::Value,
) -> Option<&'a str> {
    match tool_name {
        "Edit" | "Write" | "MultiEdit" => input.get("file_path").and_then(|v| v.as_str()),
        "create" | "str_replace" => input.get("path").and_then(|v| v.as_str()),
        _ => None,
    }
}

/// Process one bus envelope: capture the before-state if this is a
/// known editing-tool ToolUseStart (Claude: Edit/Write/MultiEdit;
/// Copilot: create/str_replace) for the observed `(run_id, step_id)` pair.
fn handle_envelope(
    snapshotter: &mut FileSnapshotter,
    envelope: &EventEnvelope,
    run_id: &str,
    step_id: &str,
    ws_root: &Path,
) {
    // Filter by run_id (belt-and-suspenders: ULIDs make collisions unlikely
    // but the contract should be enforced).
    if envelope.run_id != run_id {
        return;
    }

    // Filter by step_id.
    if envelope.step_id.as_deref() != Some(step_id) {
        return;
    }

    // Only act on ToolUseStart for Edit, Write, or MultiEdit tools.
    let (tool_name, input) = match &envelope.event {
        Event::ToolUseStart {
            tool_name, input, ..
        } => (tool_name, input),
        _ => return,
    };

    // Extract path via the cross-backend helper. Returns None for non-editing
    // tools (read-only or shell), which we silently skip.
    let file_path_str = match edited_path_from_tool_use(tool_name, input) {
        Some(s) => s,
        None => return,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn edited_path_from_tool_use_recognizes_claude_tools() {
        let input = json!({"file_path": "/tmp/x"});
        assert_eq!(edited_path_from_tool_use("Edit", &input), Some("/tmp/x"));
        assert_eq!(edited_path_from_tool_use("Write", &input), Some("/tmp/x"));
        assert_eq!(
            edited_path_from_tool_use("MultiEdit", &input),
            Some("/tmp/x")
        );
    }

    #[test]
    fn edited_path_from_tool_use_recognizes_copilot_tools() {
        let input = json!({"path": "/tmp/y"});
        assert_eq!(edited_path_from_tool_use("create", &input), Some("/tmp/y"));
        assert_eq!(
            edited_path_from_tool_use("str_replace", &input),
            Some("/tmp/y")
        );
    }

    #[test]
    fn edited_path_from_tool_use_ignores_other_tools() {
        let input = json!({"path": "/tmp/z", "file_path": "/tmp/z"});
        assert!(edited_path_from_tool_use("view", &input).is_none());
        assert!(edited_path_from_tool_use("bash", &input).is_none());
        assert!(edited_path_from_tool_use("Read", &input).is_none());
        assert!(edited_path_from_tool_use("future_tool_xyz", &input).is_none());
    }

    #[test]
    fn edited_path_from_tool_use_returns_none_when_path_field_missing() {
        let input = json!({});
        assert!(edited_path_from_tool_use("Edit", &input).is_none());
        assert!(edited_path_from_tool_use("create", &input).is_none());
    }
}
