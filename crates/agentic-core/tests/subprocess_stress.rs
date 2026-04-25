//! Tests for subprocess stderr capture (GH #36) and concurrent stdin/stdout
//! (GH #27) fixes.
//!
//! All tests are gated behind `#[cfg(unix)]` because:
//!   - The fixtures are shell scripts (`.sh`).
//!   - Signal-based cancellation semantics are Unix-only.

#[cfg(unix)]
mod unix_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    use agentic_core::backends::claude_code::runner::{ClaudeRunner, RunOutcome};
    use tokio_util::sync::CancellationToken;

    /// Return the absolute path to a named fixture in `tests/fixtures/bin/`.
    fn fixture_bin(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bin")
            .join(name)
    }

    // -----------------------------------------------------------------------
    // Issue #36: stderr is captured and surfaced in RunOutcome.stderr_tail
    // -----------------------------------------------------------------------

    /// fake-claude-stderr-error.sh writes two lines to stderr and exits 7.
    /// The runner must capture both lines in `RunOutcome.stderr_tail`.
    #[tokio::test]
    async fn runner_captures_stderr_on_nonzero_exit() {
        let runner = ClaudeRunner::with_binary(fixture_bin("fake-claude-stderr-error.sh"));
        let outcome: RunOutcome = tokio::time::timeout(
            Duration::from_secs(10),
            runner.run(
                vec![],
                HashMap::new(),
                std::env::temp_dir(),
                Vec::new(),
                CancellationToken::new(),
            ),
        )
        .await
        .expect("test timed out")
        .expect("runner must not error");

        assert_eq!(outcome.exit_code, Some(7), "expected exit code 7");
        assert!(
            outcome.stderr_tail.contains("fatal: something went wrong"),
            "stderr_tail must contain first error line; got: {:?}",
            outcome.stderr_tail
        );
        assert!(
            outcome.stderr_tail.contains("secondary error line"),
            "stderr_tail must contain second error line; got: {:?}",
            outcome.stderr_tail
        );
    }

    // -----------------------------------------------------------------------
    // Issue #27: 100KB stdin + 2000 lines stdout must not deadlock
    // -----------------------------------------------------------------------

    /// fake-claude-large-output.sh reads all stdin first, then emits 2000 lines.
    /// Piping 100KB of stdin while reading ~100KB of stdout would deadlock
    /// on the OLD synchronous-stdin code path. The concurrent stdin task fixes it.
    #[tokio::test]
    async fn runner_handles_large_stdin_with_concurrent_stdout() {
        let stdin_bytes: Vec<u8> = vec![b'x'; 100 * 1024];
        let runner = ClaudeRunner::with_binary(fixture_bin("fake-claude-large-output.sh"));

        let outcome: RunOutcome = tokio::time::timeout(
            Duration::from_secs(10),
            runner.run(
                vec![],
                HashMap::new(),
                std::env::temp_dir(),
                stdin_bytes,
                CancellationToken::new(),
            ),
        )
        .await
        .expect("test timed out — likely deadlocked on synchronous stdin write")
        .expect("runner must not error");

        assert_eq!(outcome.exit_code, Some(0), "expected exit code 0");
        assert_eq!(
            outcome.stdout_lines.len(),
            2000,
            "expected 2000 stdout lines, got {}",
            outcome.stdout_lines.len()
        );
    }

    // -----------------------------------------------------------------------
    // Issue #36: stderr buffer is capped at 64KB
    // -----------------------------------------------------------------------

    /// fake-claude-stderr-overflow.sh emits ~200KB of stderr.
    /// RunOutcome.stderr_tail must be <= 64KB + some newline slack.
    #[tokio::test]
    async fn runner_stderr_tail_capped_at_64k() {
        let runner = ClaudeRunner::with_binary(fixture_bin("fake-claude-stderr-overflow.sh"));

        let outcome: RunOutcome = tokio::time::timeout(
            Duration::from_secs(15),
            runner.run(
                vec![],
                HashMap::new(),
                std::env::temp_dir(),
                Vec::new(),
                CancellationToken::new(),
            ),
        )
        .await
        .expect("test timed out")
        .expect("runner must not error");

        // Buffer cap: 64KB + 1 extra line of slack (the last line may push it just over 64*1024)
        const CAP: usize = 64 * 1024 + 200;
        assert!(
            outcome.stderr_tail.len() <= CAP,
            "stderr_tail must be <= {CAP} bytes, got {} bytes",
            outcome.stderr_tail.len()
        );
        // Must have captured SOMETHING (not an empty string)
        assert!(
            !outcome.stderr_tail.is_empty(),
            "stderr_tail must not be empty when subprocess writes to stderr"
        );
    }
}
