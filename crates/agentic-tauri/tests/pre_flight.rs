//! Tests for pre-flight check error messages.
//!
//! These tests cover the contract from Step G.3 (existing) and Step I.2 (new):
//! the agent-not-found error must list every searched path for the active
//! backend so users know exactly where to drop agent files.
//!
//! Step I.2 adds a user-supplied `agents` slice so the hardcoded 4-agent list
//! is gone.
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

/// Write a minimal valid agent stub file at `<dir>/<name>.md`.
fn write_agent_stub(dir: &std::path::Path, name: &str) {
    let content = format!(
        "+++\nname = \"{name}\"\ndescription = \"stub\"\npipeline_role = \"step\"\n+++\nbody"
    );
    std::fs::write(dir.join(format!("{name}.md")), content).unwrap();
}

fn canonical_agents() -> Vec<String> {
    ["architect", "tdd-developer", "qa", "reviewer"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// G.3 tests — updated to pass canonical 4-agent list as the new agents arg
// ---------------------------------------------------------------------------

#[test]
fn error_message_for_missing_claude_code_agent_lists_three_claude_paths() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    // Override CLAUDE_CODE_BIN so the binary check passes.
    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents = canonical_agents();
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("should fail: no agent files present");

    // Must start with "pre-flight:"
    assert!(
        err.starts_with("pre-flight:"),
        "error must start with 'pre-flight:'; got: {err}"
    );

    // Must contain both ClaudeCode-specific path variants for architect.
    let ws_str = ws.path().to_string_lossy();
    let home_str = home.path().to_string_lossy();

    assert!(
        err.contains(&format!("{ws_str}/.claude/agents/architect.md")),
        "error should list project .claude/agents .md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{ws_str}/.claude/agents/architect.agent.md")),
        "error should list project .claude/agents .agent.md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.claude/agents/architect.md")),
        "error should list $HOME/.claude/agents .md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.claude/agents/architect.agent.md")),
        "error should list $HOME/.claude/agents .agent.md path; got:\n{err}"
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

    // Must NOT suggest the retired `agentic-cli init` hint.
    assert!(
        !err.contains("agentic-cli init"),
        "error must not suggest retired agentic-cli init; got:\n{err}"
    );
    // Must suggest placing a file at one of the listed paths.
    assert!(
        err.contains("Place an agent file"),
        "error should say 'Place an agent file'; got:\n{err}"
    );
}

#[test]
fn error_message_for_missing_copilot_cli_agent_lists_three_copilot_paths() {
    let (ws, bin_path) = workspace_with_binary("copilot");
    let home = empty_home();

    // Override COPILOT_CLI_BIN so the binary check passes.
    unsafe {
        std::env::set_var("COPILOT_CLI_BIN", &bin_path);
    }

    let agents = canonical_agents();
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::CopilotCli,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("COPILOT_CLI_BIN");
    }

    let err = result.expect_err("should fail: no agent files present");

    // Must start with "pre-flight:"
    assert!(
        err.starts_with("pre-flight:"),
        "error must start with 'pre-flight:'; got: {err}"
    );

    // Must contain both CopilotCli-specific path variants for architect.
    let ws_str = ws.path().to_string_lossy();
    let home_str = home.path().to_string_lossy();

    assert!(
        err.contains(&format!("{ws_str}/.github/agents/architect.md")),
        "error should list project .github/agents .md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{ws_str}/.github/agents/architect.agent.md")),
        "error should list project .github/agents .agent.md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.copilot/agents/architect.md")),
        "error should list $HOME/.copilot/agents .md path; got:\n{err}"
    );
    assert!(
        err.contains(&format!("{home_str}/.copilot/agents/architect.agent.md")),
        "error should list $HOME/.copilot/agents .agent.md path; got:\n{err}"
    );

    // Must NOT mention claude paths.
    assert!(
        !err.contains(".claude/agents/"),
        "copilot-cli error must not list .claude/agents/; got:\n{err}"
    );

    // Must NOT suggest the retired `agentic-cli init --copilot` hint.
    assert!(
        !err.contains("agentic-cli init"),
        "error must not suggest retired agentic-cli init; got:\n{err}"
    );
    // Must suggest placing a file at one of the listed paths.
    assert!(
        err.contains("Place an agent file"),
        "error should say 'Place an agent file'; got:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// Smoke regression: mirrors user's actual setup (.agent.md extension)
// ---------------------------------------------------------------------------

/// Mirrors the user's actual workspace: `spec-writer.agent.md` (not
/// `spec-writer.md`) at `.github/agents/`. The pre-flight must resolve it.
#[test]
fn smoke_agent_dot_agent_md_resolves_for_copilot_cli() {
    let (ws, bin_path) = workspace_with_binary("copilot");
    let home = empty_home();

    let agents_dir = ws.path().join(".github").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    // Write the .agent.md variant only — no plain .md file.
    let content = "+++\nname = \"spec-writer\"\ndescription = \"stub\"\npipeline_role = \"step\"\n+++\nbody";
    std::fs::write(agents_dir.join("spec-writer.agent.md"), content).unwrap();

    unsafe {
        std::env::set_var("COPILOT_CLI_BIN", &bin_path);
    }

    let agents = vec!["spec-writer".to_string()];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::CopilotCli,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("COPILOT_CLI_BIN");
    }

    assert!(
        result.is_ok(),
        "spec-writer.agent.md in .github/agents/ must resolve; got: {:?}",
        result
    );
}

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

    let agents = canonical_agents();
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

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

// ---------------------------------------------------------------------------
// I.2 tests — new behaviour: user-supplied agents slice
// ---------------------------------------------------------------------------

/// Happy path: a single agent present in .claude/agents/ resolves Ok.
/// This proves the function works for any non-empty list, not just the
/// canonical four.
#[test]
fn pre_flight_succeeds_with_user_selected_agents() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    // Create only the agents the user selected.
    let agents_dir = ws.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    write_agent_stub(&agents_dir, "architect");
    write_agent_stub(&agents_dir, "qa");

    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents: Vec<String> = vec!["architect".to_string(), "qa".to_string()];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    assert!(
        result.is_ok(),
        "should succeed when all selected agents exist; got: {:?}",
        result
    );
}

/// Empty agents slice must return an error containing "agents list is empty".
#[test]
fn pre_flight_errors_on_empty_agent_list() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents: Vec<String> = vec![];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("empty agents list must be rejected");
    assert!(
        err.contains("agents list is empty"),
        "error must explain agents list is empty; got: {err}"
    );
}

/// First missing agent in the user's list is named in the error, not the
/// first of the canonical four.
#[test]
fn pre_flight_lists_first_missing_agent() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    // Create only architect; designer and qa are absent.
    let agents_dir = ws.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    write_agent_stub(&agents_dir, "architect");

    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents: Vec<String> = vec![
        "architect".to_string(),
        "designer".to_string(),
        "qa".to_string(),
    ];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("missing designer must fail pre-flight");
    assert!(
        err.contains("designer"),
        "error must name 'designer' (first missing agent); got: {err}"
    );
    // Must NOT name qa (second missing) — we stop at the first missing.
    assert!(
        !err.contains("'qa'"),
        "error must not mention 'qa' before reporting 'designer'; got: {err}"
    );
}

/// Totally non-canonical names still work when their files exist.
/// This is the load-bearing test that proves the hardcoded list is gone.
#[test]
fn pre_flight_does_not_require_canonical_4_agents() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    let agents_dir = ws.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    write_agent_stub(&agents_dir, "foo");
    write_agent_stub(&agents_dir, "bar");

    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents: Vec<String> = vec!["foo".to_string(), "bar".to_string()];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    assert!(
        result.is_ok(),
        "non-canonical agent names must succeed when files exist; got: {:?}",
        result
    );
}

/// Single-agent list resolves Ok when the one file exists.
#[test]
fn pre_flight_works_with_single_agent() {
    let (ws, bin_path) = workspace_with_binary("claude");
    let home = empty_home();

    let agents_dir = ws.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    write_agent_stub(&agents_dir, "just-one");

    unsafe {
        std::env::set_var("CLAUDE_CODE_BIN", &bin_path);
    }

    let agents: Vec<String> = vec!["just-one".to_string()];
    let result = pre_flight_check_with_home(
        ws.path(),
        &BackendKind::ClaudeCode,
        Some(home.path()),
        &agents,
    );

    unsafe {
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    assert!(
        result.is_ok(),
        "single-agent list must succeed when the file exists; got: {:?}",
        result
    );
}
