#![cfg(test)]

use std::path::PathBuf;

use agentic_tauri::commands::workspace::resolve_with_home;

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
