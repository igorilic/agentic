//! Integration tests for the Claude subprocess runner.
//!
//! All tests are gated behind `#[cfg(unix)]` because:
//!   - The fixtures are shell scripts (`.sh`).
//!   - Signal-based cancellation semantics (SIGTERM → SIGKILL) are Unix-only.

#[cfg(unix)]
mod unix_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use agentic_core::backends::claude_code::runner::{ClaudeRunner, RunOutcome};
    use tokio_util::sync::CancellationToken;

    /// Return the absolute path to a named fixture in `tests/fixtures/bin/`.
    fn fixture_bin(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bin")
            .join(name)
    }

    // -----------------------------------------------------------------------
    // Happy path
    // -----------------------------------------------------------------------

    /// Runner invokes fake-claude.sh, pipes stdin, collects stdout lines.
    /// Asserts:
    ///   - The echo_stdin line reflects what we sent.
    ///   - The downstream JSONL lines arrive in order.
    ///   - `was_cancelled` is false.
    #[tokio::test]
    async fn happy_path_pipes_stdin_and_captures_stdout() {
        let runner = ClaudeRunner::with_binary(fixture_bin("fake-claude.sh"));
        let cwd = std::env::temp_dir();
        let cancel = CancellationToken::new();

        let prompt = "hello world";
        let stdin_bytes = prompt.as_bytes().to_vec();

        let outcome: RunOutcome = runner
            .run(
                vec![
                    "-p".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                ],
                HashMap::new(),
                cwd,
                stdin_bytes,
                cancel,
            )
            .await
            .expect("runner must not error on happy path");

        assert!(!outcome.was_cancelled, "should not be cancelled");
        assert_eq!(outcome.exit_code, Some(0), "exit code should be 0");

        // First line must be the echo_stdin roundtrip
        let first = outcome.stdout_lines.first().expect("at least one line");
        assert!(
            first.contains("echo_stdin"),
            "first line should be echo_stdin, got: {first}"
        );
        assert!(
            first.contains(prompt),
            "echo_stdin line should contain the prompt, got: {first}"
        );

        // The three synthetic Claude stream events must follow
        let types: Vec<String> = outcome
            .stdout_lines
            .iter()
            .filter_map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).ok()?;
                v.get("type")?.as_str().map(|s| s.to_owned())
            })
            .collect();

        assert!(
            types.iter().any(|t| t == "message_start"),
            "must contain message_start; got types: {types:?}"
        );
        assert!(
            types.iter().any(|t| t == "content_block_delta"),
            "must contain content_block_delta; got types: {types:?}"
        );
        assert!(
            types.iter().any(|t| t == "message_stop"),
            "must contain message_stop; got types: {types:?}"
        );
    }

    // -----------------------------------------------------------------------
    // SIGTERM on cancel within 1s
    // -----------------------------------------------------------------------

    /// Cancel token is triggered after 200ms. fake-claude-long.sh sleeps 30s.
    /// Assert the runner returns within 1000ms and `was_cancelled` is true.
    #[cfg(unix)]
    #[tokio::test]
    async fn sigterm_on_cancel_within_1s() {
        let runner = ClaudeRunner::with_binary_and_grace(
            fixture_bin("fake-claude-long.sh"),
            Duration::from_millis(500),
        );
        let cwd = std::env::temp_dir();
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        let start = Instant::now();
        let outcome = runner
            .run(vec![], HashMap::new(), cwd, vec![], cancel)
            .await
            .expect("runner must return Ok even when cancelled");

        let elapsed = start.elapsed();
        assert!(outcome.was_cancelled, "should be cancelled");
        assert!(
            elapsed < Duration::from_millis(1000),
            "should finish within 1000ms, took {elapsed:?}"
        );
    }

    // -----------------------------------------------------------------------
    // SIGKILL escalation after grace period
    // -----------------------------------------------------------------------

    /// fake-claude-trap.sh traps SIGTERM and ignores it.
    /// Runner must escalate to SIGKILL after grace period.
    /// Assert subprocess ends within grace + 500ms buffer.
    #[cfg(unix)]
    #[tokio::test]
    async fn sigkill_escalation_after_grace_period() {
        // Short grace (300ms) so the test completes quickly
        let grace = Duration::from_millis(300);
        let runner = ClaudeRunner::with_binary_and_grace(fixture_bin("fake-claude-trap.sh"), grace);
        let cwd = std::env::temp_dir();
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        let start = Instant::now();
        let outcome = runner
            .run(vec![], HashMap::new(), cwd, vec![], cancel)
            .await
            .expect("runner must return Ok even when killed");

        let elapsed = start.elapsed();
        assert!(outcome.was_cancelled, "should be cancelled");

        // Must finish within grace + cancel delay + 800ms buffer.
        // Worst-case path: cancel(200) + grace(300) + post-SIGKILL(200) = 700ms.
        // 800ms buffer gives a 600ms safety margin over that path.
        let max_expected = grace + Duration::from_millis(200 + 800);
        assert!(
            elapsed < max_expected,
            "should finish within {max_expected:?}, took {elapsed:?}"
        );
    }
}
