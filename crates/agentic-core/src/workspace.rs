use std::path::Path;

/// Derive a stable workspace id from the canonical absolute path.
///
/// Uses the first 16 hex chars of a blake3 hash of the canonicalized path,
/// prefixed with `ws-`.  If `canonicalize` fails (e.g. the directory does not
/// exist yet), the raw path bytes are hashed instead — this is a safe fallback
/// for relative or not-yet-created paths.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use agentic_core::workspace::stable_workspace_id;
///
/// let id = stable_workspace_id(Path::new("/tmp"));
/// assert!(id.starts_with("ws-"));
/// assert_eq!(id.len(), 19);
/// ```
pub fn stable_workspace_id(ws_root: &Path) -> String {
    let canonical = ws_root
        .canonicalize()
        .unwrap_or_else(|_| ws_root.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes());
    let hex = hash.to_hex();
    format!("ws-{}", &hex.as_str()[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_workspace_id_starts_with_ws() {
        let tmp = tempfile::tempdir().unwrap();
        let id = stable_workspace_id(tmp.path());
        assert!(id.starts_with("ws-"), "got: {id}");
    }

    #[test]
    fn stable_workspace_id_has_length_19() {
        let tmp = tempfile::tempdir().unwrap();
        let id = stable_workspace_id(tmp.path());
        assert_eq!(id.len(), 19, "got: {id}");
    }

    #[test]
    fn stable_workspace_id_is_deterministic() {
        let tmp = tempfile::tempdir().unwrap();
        let id1 = stable_workspace_id(tmp.path());
        let id2 = stable_workspace_id(tmp.path());
        assert_eq!(id1, id2);
    }

    #[test]
    fn stable_workspace_id_differs_for_different_paths() {
        let tmp1 = tempfile::tempdir().unwrap();
        let tmp2 = tempfile::tempdir().unwrap();
        let id1 = stable_workspace_id(tmp1.path());
        let id2 = stable_workspace_id(tmp2.path());
        assert_ne!(id1, id2);
    }
}
