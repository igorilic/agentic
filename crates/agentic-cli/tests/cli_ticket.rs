use std::path::PathBuf;
use std::process::Command;

fn cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_agentic-cli"))
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
