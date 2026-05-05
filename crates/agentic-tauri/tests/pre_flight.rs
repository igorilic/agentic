//! Tests for pre-flight check error messages.
//!
//! These tests cover the contract from Step G.3: the agent-not-found error
//! must list every searched path for the active backend so users know exactly
//! where to drop agent files.
#![cfg(test)]

use agentic_core::BackendKind;
use agentic_tauri::commands::ticket::pre_flight_check_with_home;
use tempfile::TempDir;

/// Create a temp home directory (no agent files inside).
fn empty_home() -> TempDir {
    tempfile::tempdir().unwrap()
}

/// Create a temp workspace directory (no agent files inside) and a fake
/// binary at `<tmp>/bin/<name>` that is executable.
fn workspace_with_binary(bin_name: &str) -> (TempDir, std::path::PathBuf) {
    let ws = tempfile::tempdir().unwrap();
    let bin_dir = ws.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let bin_path = bin_dir.join(bin_name);
    std::fs::write(&bin_path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    (ws, bin_path)
}

// ---------------------------------------------------------------------------
// error message for missing claude-code agent lists the three claude paths
// ---------------------------------------------------------------------------

#[test]
fn error_message_for_missing_claude_code_agent_lists_three_claude_paths() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    // Override CLAUDE_CODE_BIN so the binary check passes.
    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let result =
        pre_flight_check_with_home(ws.path(), &BackendKind::ClaudeCode, Some(home.path()));

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("should fail: no agent files present");

    // Must start with "pre-flight:"
    assert!(
        err.starts_with("pre-flight:"),
        "error must start with 'pre-flight:'; got: {err}"
    );

    // Must contain the three ClaudeCode-specific paths for architect.
    let ws_str = ws.path().to_string_lossy();
    let home_str = home.path().to_string_lossy();

    assert!(
        err.contains(&format!("{ws_str}/.agentic/agents/architect.md")),
        "error should list .agentic/agents path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{ws_str}/.claude/agents/architect.md")),
        "error should list .claude/agents path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.claude/agents/architect.md")),
        "error should list $HOME/.claude/agents path; got:\n{err}"
    );

    // Must NOT mention copilot paths.
    assert!(
        !err.contains(".github/agents/"),
        "claude-code error must not list .github/agents/; got:\n{err}"
    );
    assert!(
        !err.contains(".copilot/"),
        "claude-code error must not list .copilot/; got:\n{err}"
    );

    // Must suggest `agentic-cli init` (without --copilot flag).
    assert!(
        err.contains("agentic-cli init"),
        "error should suggest agentic-cli init; got:\n{err}"
    );
    assert!(
        !err.contains("--copilot"),
        "claude-code error must not suggest --copilot flag; got:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// error message for missing copilot-cli agent lists the three copilot paths
// ---------------------------------------------------------------------------

#[test]
fn error_message_for_missing_copilot_cli_agent_lists_three_copilot_paths() {
    let (ws, bin_path) = workspace_with_binary("copilot");
    let home = empty_home();

    // Override COPILOT_CLI_BIN so the binary check passes.
    unsafe {
        std::env::set_var("COPILOT_CLI_BIN", &bin_path);
    }

    let result =
        pre_flight_check_with_home(ws.path(), &BackendKind::CopilotCli, Some(home.path()));

    unsafe {
        std::env::remove_var("COPILOT_CLI_BIN");
    }

    let err = result.expect_err("should fail: no agent files present");

    // Must start with "pre-flight:"
    assert!(
        err.starts_with("pre-flight:"),
        "error must start with 'pre-flight:'; got: {err}"
    );

    // Must contain the three CopilotCli-specific paths for architect.
    let ws_str = ws.path().to_string_lossy();
    let home_str = home.path().to_string_lossy();

    assert!(
        err.contains(&format!("{ws_str}/.agentic/agents/architect.md")),
        "error should list .agentic/agents path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{ws_str}/.github/agents/architect.md")),
        "error should list .github/agents path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.copilot/agents/architect.md")),
        "error should list $HOME/.copilot/agents path; got:\n{err}"
    );

    // Must NOT mention claude paths.
    assert!(
        !err.contains(".claude/agents/"),
        "copilot-cli error must not list .claude/agents/; got:\n{err}"
    );

    // Must suggest `agentic-cli init --copilot`.
    assert!(
        err.contains("agentic-cli init --copilot"),
        "error should suggest agentic-cli init --copilot; got:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// install hint regression: binary-on-PATH check still works
// ---------------------------------------------------------------------------

#[test]
fn error_message_includes_install_hint_when_binary_missing() {
    let ws = tempfile::tempdir().unwrap();
    let home = empty_home();

    // Point CLAUDE_CODE_BIN at a path that definitely does not exist.
    unsafe {
        std::env::set_var(
            "CLAUDE_CODE_BIN",
            "/nonexistent/path/to/claude-definitely-not-here",
        );
    }

    let result =
        pre_flight_check_with_home(ws.path(), &BackendKind::ClaudeCode, Some(home.path()));

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("should fail: binary not on PATH");

    assert!(
        err.starts_with("pre-flight:"),
        "error must start with 'pre-flight:'; got: {err}"
    );
    assert!(
        err.to_lowercase().contains("install claude code")
            || err.contains("https://docs.claude.com"),
        "error should include Claude Code install hint; got: {err}"
    );
}
