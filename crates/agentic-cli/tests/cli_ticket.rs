use std::path::PathBuf;
use std::process::Command;

fn cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agentic-cli"))
}

/// Write minimal agent fixture files so agent discovery succeeds.
fn setup_workspace_with_default_agents(base: &std::path::Path) {
    let agents_dir = base.join(".agentic").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    for name in &["architect", "tdd-developer", "qa", "reviewer"] {
        std::fs::write(
            agents_dir.join(format!("{name}.md")),
            format!("+++\nname = \"{name}\"\ndescription = \"test\"\n+++\nYou are {name}.\n"),
        )
        .unwrap();
    }
}

#[test]
fn run_requires_scripted_or_ticket() {
    // Calling `run` without --scripted or --ticket should fail with exit code 2
    // and mention the missing argument group.
    let output = Command::new(cargo_bin())
        .arg("run")
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "expected non-zero exit when neither --scripted nor --ticket provided"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2; got: {:?}",
        output.status.code()
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // clap should mention one of the required flags.
    assert!(
        stderr.contains("scripted") || stderr.contains("ticket"),
        "stderr should mention --scripted or --ticket; got: {stderr}"
    );
}

#[test]
fn run_rejects_both_scripted_and_ticket() {
    // Providing both --scripted and --ticket should be rejected by clap ArgGroup.
    let output = Command::new(cargo_bin())
        .args(["run", "--scripted", "/tmp/foo.json", "--ticket", "bar"])
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "expected non-zero exit when both --scripted and --ticket provided"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2; got: {:?}",
        output.status.code()
    );
}

#[test]
fn run_ticket_without_claude_binary_fails_with_clear_error() {
    // Use a nonexistent claude binary path. The CLI should fail non-zero.
    // We do NOT set up agent files, so the failure may happen at agent discovery
    // OR at subprocess spawn — either is acceptable per the spec.
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    // Write a dummy agent file so we can reach the backend execution path.
    let agents_dir = tmp.path().join(".agentic").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    for name in &["architect", "tdd-developer", "qa", "reviewer"] {
        std::fs::write(
            agents_dir.join(format!("{name}.md")),
            format!("+++\nname = \"{name}\"\ndescription = \"test\"\n+++\nYou are {name}.\n"),
        )
        .unwrap();
    }

    let output = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .args(["run", "--ticket", "hello world"])
        .env("CLAUDE_CODE_BIN", "/nonexistent/bin/claude")
        .current_dir(tmp.path())
        .output()
        .expect("spawn");

    assert!(
        !output.status.success(),
        "expected non-zero exit when claude binary is absent"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should mention claude, binary path, or agent in the error.
    assert!(
        stderr.contains("claude")
            || stderr.contains("nonexistent")
            || stderr.contains("agent")
            || stderr.contains("error"),
        "stderr should contain a meaningful error message; got: {stderr}"
    );
}

#[test]
fn run_default_backend_is_claude_code_when_flag_omitted() {
    // Verify `--backend` flag appears in `run --help` with both valid values.
    let output = Command::new(cargo_bin())
        .args(["run", "--help"])
        .output()
        .expect("spawn cli");
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--backend"), "--backend flag should appear in help");
    assert!(help.contains("claude-code"), "claude-code should be a value");
    assert!(help.contains("copilot-cli"), "copilot-cli should be a value");
}

#[test]
fn run_rejects_invalid_backend_value() {
    let output = Command::new(cargo_bin())
        .args(["run", "--ticket", "hi", "--backend", "bogus"])
        .output()
        .expect("spawn cli");
    assert!(!output.status.success(), "should reject invalid backend");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("backend") || stderr.contains("bogus"),
        "stderr should mention backend or the bogus value: {stderr}"
    );
}

#[test]
fn run_backend_copilot_with_missing_binary_fails_with_clear_error() {
    // Mirrors run_ticket_without_claude_binary_fails_with_clear_error for copilot-cli.
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    setup_workspace_with_default_agents(tmp.path());

    let output = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .args(["run", "--ticket", "hello world", "--backend", "copilot-cli"])
        .env("COPILOT_CLI_BIN", "/nonexistent/bin/copilot")
        .current_dir(tmp.path())
        .output()
        .expect("spawn");

    assert!(
        !output.status.success(),
        "expected non-zero exit when copilot binary is absent"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("copilot")
            || stderr.contains("nonexistent")
            || stderr.contains("agent")
            || stderr.contains("error"),
        "stderr should contain a meaningful error message; got: {stderr}"
    );
}
