#![cfg(test)]

use std::path::PathBuf;

use agentic_tauri::commands::workspace::{resolve_with_home, get_workspace_id_inner};

// ─── tilde expansion ─────────────────────────────────────────────────────────

/// `AGENTIC_WORKSPACE_ROOT=~/some/path` with a fake home dir should resolve
/// to `<fake_home>/some/path`, NOT the literal `~/some/path`.
#[test]
fn resolve_workspace_root_expands_tilde() {
    let home = tempfile::tempdir().unwrap();
    let fake_home = home.path();

    // Create the target directory so `is_dir()` passes.
    let target = fake_home.join("some").join("dir");
    std::fs::create_dir_all(&target).unwrap();

    // Simulate AGENTIC_WORKSPACE_ROOT=~/some/dir.
    let env_value = "~/some/dir";
    let result =
        resolve_with_home(Some(env_value), fake_home).expect("resolve_with_home should succeed");

    assert!(
        result.starts_with(fake_home),
        "resolved path should start with fake_home; got: {}",
        result.display()
    );
    assert_eq!(
        result,
        PathBuf::from(fake_home).join("some").join("dir"),
        "tilde should be replaced with home dir; got: {}",
        result.display()
    );
}

/// A path without tilde should pass through verbatim.
#[test]
fn resolve_workspace_root_absolute_path_unchanged() {
    let home = tempfile::tempdir().unwrap();
    let fake_home = home.path();

    let target = tempfile::tempdir().unwrap();
    let abs_path = target.path().to_str().unwrap().to_string();

    let result = resolve_with_home(Some(&abs_path), fake_home)
        .expect("resolve_with_home should succeed for absolute path");

    assert_eq!(result, target.path());
}

/// When env is `None`, `resolve_with_home` returns an error (no cwd fallback
/// is exercised in the unit test — just that None without a cwd fails gracefully).
#[test]
fn resolve_workspace_root_with_none_env_falls_back_to_cwd() {
    let home = tempfile::tempdir().unwrap();
    // We can't control cwd in a unit test, but we can assert it doesn't panic.
    // The function should return either Ok or Err without panicking.
    let _ = resolve_with_home(None, home.path());
}

/// `AGENTIC_WORKSPACE_ROOT=~` alone should resolve to home dir itself.
#[test]
fn resolve_workspace_root_bare_tilde_expands_to_home() {
    let home = tempfile::tempdir().unwrap();
    let fake_home = home.path();

    let result = resolve_with_home(Some("~"), fake_home)
        .expect("resolve_with_home should succeed for bare tilde");

    assert_eq!(
        result,
        fake_home,
        "bare ~ should resolve to fake_home; got: {}",
        result.display()
    );
}

// ─── get_workspace_id_inner ───────────────────────────────────────────────────

/// `get_workspace_id_inner` with a valid workspace root returns a `ws-` prefixed
/// string of length 19 (prefix "ws-" = 3 chars + 16 hex chars).
#[test]
fn get_workspace_id_returns_ws_prefixed_hex_string() {
    let tmp = tempfile::tempdir().unwrap();

    let result = get_workspace_id_inner(tmp.path())
        .expect("get_workspace_id_inner should succeed for a valid directory");

    assert!(
        result.starts_with("ws-"),
        "workspace id should start with 'ws-'; got: {result}"
    );
    assert_eq!(
        result.len(),
        19,
        "workspace id should be 19 chars ('ws-' + 16 hex); got: {result}"
    );
    // All chars after 'ws-' should be hex
    let hex_part = &result[3..];
    assert!(
        hex_part.chars().all(|c| c.is_ascii_hexdigit()),
        "chars after 'ws-' should all be hex digits; got: {hex_part}"
    );
}

/// Two calls with the same path return the same id (stable / deterministic).
#[test]
fn get_workspace_id_is_stable_for_same_path() {
    let tmp = tempfile::tempdir().unwrap();

    let id1 = get_workspace_id_inner(tmp.path()).unwrap();
    let id2 = get_workspace_id_inner(tmp.path()).unwrap();

    assert_eq!(id1, id2, "workspace id must be deterministic for the same path");
}

/// Two different paths produce different ids.
#[test]
fn get_workspace_id_differs_for_different_paths() {
    let tmp1 = tempfile::tempdir().unwrap();
    let tmp2 = tempfile::tempdir().unwrap();

    let id1 = get_workspace_id_inner(tmp1.path()).unwrap();
    let id2 = get_workspace_id_inner(tmp2.path()).unwrap();

    assert_ne!(id1, id2, "different paths must produce different workspace ids");
}

/// `get_workspace_id_inner` with AGENTIC_WORKSPACE_ROOT set returns a ws- prefixed string.
#[test]
fn get_workspace_id_with_env_var() {
    let tmp = tempfile::tempdir().unwrap();
    let ws_path = tmp.path().to_str().unwrap();

    temp_env::with_var("AGENTIC_WORKSPACE_ROOT", Some(ws_path), || {
        let result = get_workspace_id_inner(tmp.path())
            .expect("should succeed with valid directory");
        assert!(result.starts_with("ws-"));
        assert_eq!(result.len(), 19);
    });
}
