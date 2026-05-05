//! Shared workspace-root resolution for Tauri IPC commands.
//!
//! All commands that need the user's project directory use the same
//! two-step resolution:
//!   1. `AGENTIC_WORKSPACE_ROOT` env var — set in `cargo tauri dev`
//!      so `cwd` points at the right project, not the tauri crate dir.
//!   2. `std::env::current_dir()` — production fallback.

use std::path::PathBuf;

/// Resolve the workspace root for an IPC call.
///
/// Returns `Err` if neither `AGENTIC_WORKSPACE_ROOT` nor `current_dir()`
/// succeeds, or if the resolved path is not a directory.
pub fn resolve_workspace_root() -> Result<PathBuf, String> {
    let ws_root = match std::env::var_os("AGENTIC_WORKSPACE_ROOT") {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().map_err(|e| format!("cwd: {e}"))?,
    };
    if !ws_root.is_dir() {
        return Err(format!(
            "workspace root is not a directory: {}",
            ws_root.display()
        ));
    }
    Ok(ws_root)
}
