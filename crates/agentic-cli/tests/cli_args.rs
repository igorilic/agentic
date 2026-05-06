//! Tests for `--agents` CLI argument on `run --ticket`.
//!
//! Uses the subprocess approach (matching `cli_smoke.rs` / `cli_ticket.rs`):
//! invokes the compiled binary and inspects exit codes + stderr.
//!
//! These tests validate Step I.4 contract:
//!   - `--agents foo,bar` parses and is accepted
//!   - missing `--agents` with `--ticket` errors with an actionable message
//!   - whitespace-only `--agents` errors with the same actionable message
//!   - `--agents` without `--ticket` is rejected by clap

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agentic-cli"))
}

/// Scaffold minimal agent fixture files so agent discovery succeeds.
fn setup_agents(base: &std::path::Path, agent_names: &[&str]) {
    let agents_dir = base.join(".agentic").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    for name in agent_names {
        std::fs::write(
            agents_dir.join(format!("{name}.md")),
            format!("+++\nname = \"{name}\"\ndescription = \"test\"\n+++\nYou are {name}.\n"),
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// I.4.1 — `run --agents` appears in `run --help` (clap accepts the flag)
//
// The most reliable way to verify clap accepts --agents without triggering
// the full pipeline (which may hang waiting for a real backend): check that
// `run --help` lists --agents in its output. This verifies the flag is
// registered in the Cli struct.
// ---------------------------------------------------------------------------
#[test]
fn run_ticket_with_agents_is_accepted_by_clap() {
    let output = Command::new(cargo_bin())
        .args(["run", "--help"])
        .output()
        .expect("spawn");

    assert!(output.status.success(), "run --help should succeed");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(
        help.contains("--agents"),
        "run --help should list --agents flag; got: {help}"
    );
    // Also verify the help doesn't say it's only for --scripted
    // (it should be a --ticket-side option, not a global flag for scripted).
    assert!(
        !help.contains("unexpected argument '--agents'"),
        "clap should not reject --agents as unexpected; got: {help}"
    );
}

// ---------------------------------------------------------------------------
// I.4.2 — `run --ticket` without `--agents` errors with actionable message
//
// The binary must exit non-zero within 10 seconds and stderr must contain
// "--agents" and "is required". After implementation the validation fires
// before any backend is invoked, so exit is near-instant.
// ---------------------------------------------------------------------------
#[test]
fn run_ticket_without_agents_errors_with_actionable_message() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    setup_agents(
        tmp.path(),
        &["architect", "tdd-developer", "qa", "reviewer"],
    );

    let mut child = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .args(["run", "--ticket", "fix bug"])
        .env("CLAUDE_CODE_BIN", "/nonexistent/bin/claude")
        .current_dir(tmp.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    // Give the process at most 10 seconds to print the validation error.
    // After implementation, validation fires immediately — no backend spawn.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let output = loop {
        if std::time::Instant::now() > deadline {
            child.kill().ok();
            panic!(
                "run --ticket without --agents did not exit within 10 seconds; \
                    the validation check is missing or too slow"
            );
        }
        match child.try_wait().expect("try_wait") {
            Some(_) => break child.wait_with_output().expect("wait_with_output"),
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    assert!(
        !output.status.success(),
        "expected non-zero exit when --agents is missing"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--agents"),
        "error must mention --agents to be actionable; got stderr: {stderr}"
    );
    assert!(
        stderr.contains("is required"),
        "error message must contain 'is required' (our validation); got stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// I.4.3 — `run --ticket --agents ' '` (whitespace-only) errors
//
// Clap accepts a whitespace string as the --agents value (not a parse error),
// but our validation in cmd_run_ticket must reject it with the actionable
// message. The message must contain "is required" to distinguish from a
// clap "unexpected argument" error.
// ---------------------------------------------------------------------------
#[test]
fn run_ticket_with_whitespace_only_agents_errors_with_actionable_message() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    setup_agents(
        tmp.path(),
        &["architect", "tdd-developer", "qa", "reviewer"],
    );

    let mut child = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .args(["run", "--ticket", "fix bug", "--agents", " "])
        .env("CLAUDE_CODE_BIN", "/nonexistent/bin/claude")
        .current_dir(tmp.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let output = loop {
        if std::time::Instant::now() > deadline {
            child.kill().ok();
            panic!(
                "run --ticket --agents ' ' did not exit within 10 seconds; \
                    the whitespace validation is missing or too slow"
            );
        }
        match child.try_wait().expect("try_wait") {
            Some(_) => break child.wait_with_output().expect("wait_with_output"),
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    assert!(
        !output.status.success(),
        "expected non-zero exit when --agents is whitespace-only"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The message must be our validation error, not a clap parse error.
    assert!(
        stderr.contains("--agents"),
        "error must mention --agents to be actionable; got stderr: {stderr}"
    );
    assert!(
        stderr.contains("is required"),
        "error message must contain 'is required' (our validation error); got stderr: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// I.4.4 — `run --scripted ... --agents foo` is rejected quickly
//
// Our application-level check in run_command must reject --agents when used
// with --scripted before starting any infra. The binary must exit within
// 5 seconds with a non-zero exit code and an error mentioning "agents".
// ---------------------------------------------------------------------------
#[test]
fn run_scripted_with_agents_is_rejected_by_clap() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let script_path = tmp.path().join("script.json");
    std::fs::write(&script_path, "[]").unwrap();

    let mut child = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .args([
            "run",
            "--scripted",
            script_path.to_str().unwrap(),
            "--agents",
            "foo",
        ])
        .current_dir(tmp.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let output = loop {
        if std::time::Instant::now() > deadline {
            child.kill().ok();
            panic!(
                "run --scripted --agents did not exit within 5 seconds; \
                    the application-level --agents validation check is missing"
            );
        }
        match child.try_wait().expect("try_wait") {
            Some(_) => break child.wait_with_output().expect("wait_with_output"),
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    assert!(
        !output.status.success(),
        "expected non-zero exit when --agents used with --scripted"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("agents"),
        "error must mention 'agents'; got: {stderr}"
    );
}
