use std::path::PathBuf;
use std::process::Command;

fn cargo_bin() -> PathBuf {
    // Cargo sets this env var for each binary target's integration tests.
    PathBuf::from(env!("CARGO_BIN_EXE_agentic-cli"))
}

#[test]
fn help_output_includes_run_subcommand() {
    let output = Command::new(cargo_bin())
        .arg("--help")
        .output()
        .expect("spawn");
    assert!(output.status.success(), "--help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("run"),
        "--help output should mention 'run' subcommand; got: {stdout}"
    );
}

#[test]
fn run_scripted_exits_0_and_emits_json_per_line() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let script_path = tmp.path().join("script.json");

    // A minimal script: 3 events.
    let script = r#"[
        {"type": "StepStarted", "data": {"agent": "smoke", "model": "fake"}},
        {"type": "TextDelta", "data": {"content": "hello"}},
        {"type": "StepComplete", "data": {
            "status": "passed",
            "summary": "ok",
            "token_usage": {
                "input_tokens": 0,
                "output_tokens": 0,
                "cache_read_input_tokens": 0,
                "cache_creation_input_tokens": 0
            },
            "cost_usd": null,
            "duration_ms": 0
        }}
    ]"#;
    std::fs::write(&script_path, script).unwrap();

    let output = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--scripted")
        .arg(&script_path)
        .output()
        .expect("spawn");
    assert!(
        output.status.success(),
        "run --scripted should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Each line should parse as JSON.
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        !lines.is_empty(),
        "expected at least one JSON line on stdout"
    );
    for line in &lines {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|_| panic!("line was not valid JSON: {line}"));
        assert!(parsed.is_object(), "line should be a JSON object: {line}");
    }
}

// #22 — malformed JSON script exits non-zero
#[test]
fn run_scripted_malformed_json_exits_nonzero() {
    let tmp = tempfile::tempdir().unwrap();
    let data_dir = tmp.path().join("data");
    let script_path = tmp.path().join("bad.json");
    std::fs::write(&script_path, b"{ this is not valid json !!!").unwrap();

    let output = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg(&data_dir)
        .arg("run")
        .arg("--scripted")
        .arg(&script_path)
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "run --scripted with malformed JSON should exit non-zero; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn invalid_data_dir_exits_with_code_2() {
    // Use a path that can't be used as a data dir — a read-only device
    // path that exists but can't be extended. /dev/null is a file, not a
    // dir; ensure_dirs on paths under it will error.
    let output = Command::new(cargo_bin())
        .arg("--data-dir")
        .arg("/dev/null/unreachable")
        .arg("migrate")
        .output()
        .expect("spawn");
    assert!(
        !output.status.success(),
        "expected nonzero exit for invalid data-dir"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2; got: {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
