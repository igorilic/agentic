//! Per-step observer that drives [`FileSnapshotter`] from `Edit`/`Write` tool-use events.
//!
//! [`ToolUseObserver`] subscribes to the [`EventBus`], filters for
//! `Event::ToolUseStart` events whose `tool_name` is `"Edit"` or `"Write"`,
//! and captures the before-state of the referenced file via
//! [`FileSnapshotter::capture`]. After the step completes, call
//! [`ToolUseObserverHandle::finalize_into`] to compute diffs, emit
//! `Event::FileChange` events, and write `file_changes.diff`.

use std::path::Path;
use std::path::PathBuf;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::backends::file_snapshots::{FileSnapshotter, FinalizeReport};
use crate::backends::EventSink;
use crate::events::EventBus;

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
        todo!("ToolUseObserverHandle::finalize_into — implement in GREEN phase")
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
        todo!("ToolUseObserver::spawn — implement in GREEN phase")
    }
}
