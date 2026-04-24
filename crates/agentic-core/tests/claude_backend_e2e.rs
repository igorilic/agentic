//! End-to-end integration tests for `ClaudeCodeBackend::execute`.
//!
//! All tests are gated behind `#[cfg(unix)]` because:
//!   - Fixtures are shell scripts that require a Unix shell.
//!   - Cancellation uses SIGTERM/SIGKILL semantics.

#[cfg(unix)]
mod unix_tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use agentic_core::Event;
    use agentic_core::backends::claude_code::ClaudeCodeBackend;
    use agentic_core::{
        Backend, ExecuteOutcome, ExecuteRequest, ModelId, RunId, StepId, StepStatus, WorkspaceRef,
    };
    use tokio::sync::broadcast;
    use tokio_util::sync::CancellationToken;

    /// Return the absolute path to a named fixture in `tests/fixtures/bin/`.
    fn fixture_bin(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bin")
            .join(name)
    }

    fn make_request(
        binary: PathBuf,
        cancel: CancellationToken,
    ) -> (ClaudeCodeBackend, ExecuteRequest) {
        let backend = ClaudeCodeBackend::with_binary_and_grace(binary, Duration::from_millis(300));
        let req = ExecuteRequest {
            workspace: WorkspaceRef {
                id: "ws-test".to_string(),
                root_path: std::env::temp_dir(),
            },
            run_id: RunId("run-1".to_string()),
            step_id: StepId("step-1".to_string()),
            agent_name: "test-agent".to_string(),
            agent_prompt: "You are a test assistant.".to_string(),
            user_context: "Run the task.".to_string(),
            model: Some(ModelId("claude-sonnet-4-6".to_string())),
            tools: vec![],
            cwd: std::env::temp_dir(),
            timeout: None,
            cancel,
        };
        (backend, req)
    }

    // -----------------------------------------------------------------------
    // Happy path: passing script → Passed status with token usage + cost
    // -----------------------------------------------------------------------

    /// fake-claude-pass.sh emits a proper stream with usage.
    /// execute() must return Passed, non-empty token usage, and computed cost.
    #[tokio::test]
    async fn happy_path_returns_passed_with_usage_and_cost() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-claude-pass.sh"), cancel);

        let (sink, mut rx) = broadcast::channel(64);

        let outcome: ExecuteOutcome = backend
            .execute(req, sink)
            .await
            .expect("execute must not error on happy path");

        assert_eq!(
            outcome.status,
            StepStatus::Passed,
            "expected Passed, got {:?}",
            outcome.status
        );

        // Token usage must be populated
        assert!(
            outcome.token_usage.input_tokens > 0,
            "input_tokens should be > 0, got {}",
            outcome.token_usage.input_tokens
        );
        assert!(
            outcome.token_usage.output_tokens > 0,
            "output_tokens should be > 0, got {}",
            outcome.token_usage.output_tokens
        );

        // Cost must be computed (model is known in the pricing table)
        assert!(
            outcome.cost_usd.is_some(),
            "cost_usd must be Some for a known model"
        );
        assert!(
            outcome.cost_usd.unwrap() > 0.0,
            "cost_usd must be positive, got {:?}",
            outcome.cost_usd
        );

        // Events must have been forwarded — at least a TextDelta
        let mut events = Vec::new();
        while let Ok(env) = rx.try_recv() {
            events.push(env.event);
        }
        assert!(
            events.iter().any(|e| matches!(e, Event::TextDelta { .. })),
            "expected at least one TextDelta event to be forwarded; got: {events:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Error path: upstream error event → Failed + Error event in sink
    // -----------------------------------------------------------------------

    /// fake-claude-error.sh emits an authentication_error event.
    /// execute() must return Failed and forward an Error event with recoverable=false.
    #[tokio::test]
    async fn upstream_error_returns_failed_and_forwards_error_event() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-claude-error.sh"), cancel);

        let (sink, mut rx) = broadcast::channel(64);

        let outcome: ExecuteOutcome = backend
            .execute(req, sink)
            .await
            .expect("execute must return Ok even when stream has an error");

        assert_eq!(
            outcome.status,
            StepStatus::Failed,
            "expected Failed for upstream error, got {:?}",
            outcome.status
        );

        // Collect forwarded events
        let mut events = Vec::new();
        while let Ok(env) = rx.try_recv() {
            events.push(env.event);
        }

        // Must have forwarded an Error event that is non-recoverable
        let error_event = events.iter().find(|e| {
            matches!(
                e,
                Event::Error {
                    recoverable: false,
                    ..
                }
            )
        });
        assert!(
            error_event.is_some(),
            "expected a non-recoverable Error event in the sink; got: {events:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Cancel mid-stream → Failed with "cancelled" in summary
    // -----------------------------------------------------------------------

    /// fake-claude-trap.sh traps SIGTERM. We cancel after 200ms.
    /// execute() must return Failed and include "cancelled" or "subprocess_killed"
    /// in the summary.
    #[tokio::test]
    async fn cancel_mid_stream_returns_failed_with_cancelled_summary() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-claude-trap.sh"), cancel.clone());

        let (sink, _rx) = broadcast::channel(64);

        // Trigger cancel after 200ms
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            cancel_clone.cancel();
        });

        let outcome: ExecuteOutcome = backend
            .execute(req, sink)
            .await
            .expect("execute must return Ok even when cancelled");

        assert_eq!(
            outcome.status,
            StepStatus::Failed,
            "expected Failed for cancelled run, got {:?}",
            outcome.status
        );

        let summary_lower = outcome.summary.to_lowercase();
        assert!(
            summary_lower.contains("cancelled") || summary_lower.contains("subprocess_killed"),
            "expected 'cancelled' or 'subprocess_killed' in summary, got: {:?}",
            outcome.summary
        );
    }

    // -----------------------------------------------------------------------
    // Timeout: req.timeout fires cancel token → Failed with "timeout" summary
    // -----------------------------------------------------------------------

    /// fake-claude-trap.sh traps SIGTERM and runs indefinitely.
    /// Setting req.timeout to 200ms must cause execute() to return Failed with
    /// "timeout" in the summary (spec §11.4 error code).
    #[tokio::test]
    async fn execute_respects_timeout_and_fails_with_timeout_summary() {
        let binary = fixture_bin("fake-claude-trap.sh");
        let cancel = CancellationToken::new();
        let backend =
            ClaudeCodeBackend::with_binary_and_grace(binary.clone(), Duration::from_millis(300));
        let mut req = {
            let (_ignored_backend, r) = make_request(binary, cancel);
            r
        };
        req.timeout = Some(Duration::from_millis(200));

        let (sink, _rx) = broadcast::channel(64);
        let outcome = backend
            .execute(req, sink)
            .await
            .expect("execute must return Ok even on timeout");

        assert_eq!(
            outcome.status,
            StepStatus::Failed,
            "expected Failed for timed-out run, got {:?}",
            outcome.status
        );
        assert!(
            outcome.summary.contains("timeout"),
            "expected 'timeout' in summary, got: {}",
            outcome.summary
        );
    }
}
