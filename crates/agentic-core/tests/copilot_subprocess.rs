//! Integration tests for the Copilot CLI subprocess runner.
//!
//! All tests are gated behind `#[cfg(unix)]` because:
//!   - The fixtures are shell scripts (`.sh`).
//!   - Signal-based cancellation semantics (SIGTERM → SIGKILL) are Unix-only.

#[cfg(unix)]
mod unix_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use agentic_core::backends::copilot_cli::runner::{CopilotRunner, RunOutcome, StreamingRun};
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio_util::sync::CancellationToken;

    /// Return the absolute path to a named fixture in `tests/fixtures/bin/`.
    fn fixture_bin(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bin")
            .join(name)
    }

    // -----------------------------------------------------------------------
    // Happy path: runner captures JSONL stdout
    // -----------------------------------------------------------------------

    /// CopilotRunner::run with fake-copilot-pass.sh must capture 6 JSONL lines,
    /// exit_code = Some(0), was_cancelled = false.
    #[tokio::test]
    async fn run_with_fake_copilot_captures_jsonl_stdout() {
        let runner = CopilotRunner::with_binary(fixture_bin("fake-copilot-pass.sh"));
        let cwd = std::env::temp_dir();
        let cancel = CancellationToken::new();

        let outcome: RunOutcome = runner
            .run(vec![], HashMap::new(), cwd, vec![], cancel)
            .await
            .expect("runner must not error on happy path");

        assert!(!outcome.was_cancelled, "should not be cancelled");
        assert_eq!(outcome.exit_code, Some(0), "exit code should be 0");
        assert_eq!(
            outcome.stdout_lines.len(),
            6,
            "expected 6 JSONL lines, got: {:?}",
            outcome.stdout_lines
        );
    }

    // -----------------------------------------------------------------------
    // Streaming: run_streaming yields line BEFORE subprocess exits
    // -----------------------------------------------------------------------

    /// run_streaming must expose a live stdout reader so the first line is
    /// readable well before the subprocess exits (proving streaming not buffering).
    /// Uses fake-claude-slow-stream.sh which emits "line1", sleeps 2s, then "line2".
    #[tokio::test]
    async fn run_streaming_yields_line_before_subprocess_exits() {
        let runner = CopilotRunner::with_binary_and_grace(
            fixture_bin("fake-claude-slow-stream.sh"),
            Duration::from_millis(300),
        );
        let cancel = CancellationToken::new();
        let cwd = std::env::temp_dir();

        let start = Instant::now();
        let StreamingRun {
            stdout,
            wait_handle,
        } = runner
            .run_streaming(vec![], HashMap::new(), cwd, Vec::new(), cancel)
            .expect("run_streaming must not error on spawn");

        let mut reader = BufReader::new(stdout);
        let mut first_line = String::new();
        reader
            .read_line(&mut first_line)
            .await
            .expect("must be able to read first line");

        let elapsed = start.elapsed();

        // First line should arrive before the 2-second sleep completes.
        assert!(
            elapsed < Duration::from_secs(1),
            "first line should arrive before subprocess exits (streaming, not buffered); took {elapsed:?}"
        );
        assert!(
            first_line.contains("line1"),
            "first line should contain 'line1', got: {first_line:?}"
        );

        // Let the subprocess finish and verify clean exit.
        let wait_outcome = wait_handle
            .await
            .expect("wait_handle must not panic")
            .expect("wait_handle must return Ok");
        assert_eq!(wait_outcome.exit_code, Some(0), "subprocess should exit 0");
        assert!(!wait_outcome.was_cancelled, "should not be cancelled");
    }

    // -----------------------------------------------------------------------
    // SIGKILL escalation after grace period
    // -----------------------------------------------------------------------

    /// fake-copilot-trap.sh traps SIGTERM. Runner must escalate to SIGKILL.
    /// Assert subprocess ends within grace + cancel delay + buffer, was_cancelled = true.
    #[tokio::test]
    async fn run_escalates_sigterm_to_sigkill_on_cancel() {
        let grace = Duration::from_millis(300);
        let runner =
            CopilotRunner::with_binary_and_grace(fixture_bin("fake-copilot-trap.sh"), grace);
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

        // Must finish within cancel(200) + grace(300) + post-SIGKILL(200) + 800ms buffer.
        let max_expected = grace + Duration::from_millis(200 + 800);
        assert!(
            elapsed < max_expected,
            "should finish within {max_expected:?}, took {elapsed:?}"
        );
    }
}
