//! Shared workspace-root resolution for Tauri IPC commands.
//!
//! All commands that need the user's project directory use the same
//! two-step resolution:
//!   1. `AGENTIC_WORKSPACE_ROOT` env var — set in `cargo tauri dev`
//!      so `cwd` points at the right project, not the tauri crate dir.
//!   2. `std::env::current_dir()` — production fallback.
//!
//! Tilde expansion: if the env var starts with `~/` or is exactly `~`,
//! the `~` is replaced with the real home directory before path resolution.

use std::path::{Path, PathBuf};

use agentic_core::stable_workspace_id;

/// Resolve the workspace root for an IPC call.
///
/// Uses the real home directory from [`directories::BaseDirs`] for tilde
/// expansion. Returns `Err` if neither `AGENTIC_WORKSPACE_ROOT` nor
/// `current_dir()` succeeds, or if the resolved path is not a directory.
pub fn resolve_workspace_root() -> Result<PathBuf, String> {
    let base = directories::BaseDirs::new();
    let home = base
        .as_ref()
        .map(|b| b.home_dir())
        .ok_or_else(|| "cannot determine home directory".to_string())?;
    let env_val =
        std::env::var_os("AGENTIC_WORKSPACE_ROOT").map(|v| v.to_string_lossy().to_string());
    resolve_with_home(env_val.as_deref(), home)
}

/// Testable core of workspace-root resolution with an injected home dir.
///
/// - `env_val`: the raw value of `AGENTIC_WORKSPACE_ROOT` (or `None`).
/// - `home`: the home directory to use for tilde expansion.
pub fn resolve_with_home(env_val: Option<&str>, home: &Path) -> Result<PathBuf, String> {
    let ws_root = match env_val {
        Some(raw) => expand_tilde(raw, home),
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

/// Testable inner implementation for `get_workspace_id` IPC.
///
/// Returns the stable workspace id derived from `ws_root`.
/// Exposed `pub` so integration tests in `crates/agentic-tauri/tests/`
/// can call it directly without going through the full Tauri IPC stack.
pub fn get_workspace_id_inner(ws_root: &Path) -> Result<String, String> {
    Ok(stable_workspace_id(ws_root))
}

/// IPC command: returns the stable workspace id for the resolved workspace root.
///
/// The id is `ws-` followed by the first 16 hex characters of a blake3 hash
/// of the canonicalized workspace path.  Stable across restarts for the same
/// project directory.
#[tauri::command]
pub fn get_workspace_id() -> Result<String, String> {
    let ws_root = resolve_workspace_root()?;
    get_workspace_id_inner(&ws_root)
}

/// Replace a leading `~` with `home`. Handles `~/...` and bare `~`.
fn expand_tilde(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home.join(rest);
    }
    PathBuf::from(path)
}
