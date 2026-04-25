//! End-to-end integration tests for `CopilotCliBackend::execute`.
//!
//! All tests are gated behind `#[cfg(unix)]` because:
//!   - Fixtures are shell scripts that require a Unix shell.
//!   - Cancellation uses SIGTERM/SIGKILL semantics.

#[cfg(unix)]
mod unix_tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use agentic_core::Event;
    use agentic_core::backends::copilot_cli::CopilotCliBackend;
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
    ) -> (CopilotCliBackend, ExecuteRequest) {
        let backend = CopilotCliBackend::with_binary_and_grace(binary, Duration::from_millis(300));
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
            model: Some(ModelId("claude-opus-4.6".to_string())),
            tools: vec![],
            cwd: std::env::temp_dir(),
            timeout: None,
            cancel,
        };
        (backend, req)
    }

    // -----------------------------------------------------------------------
    // Happy path: passing script → Passed status with token usage
    // -----------------------------------------------------------------------

    /// fake-copilot-pass.sh emits a valid stream.
    /// execute() must return Passed and emit TextDelta events with output_tokens > 0.
    #[tokio::test]
    async fn happy_path_returns_passed_and_forwards_events() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-copilot-pass.sh"), cancel);

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

        // Copilot fixture has 3 outputTokens
        assert!(
            outcome.token_usage.output_tokens > 0,
            "output_tokens should be > 0, got {}",
            outcome.token_usage.output_tokens
        );

        // Cost is None for Copilot (no pricing table yet)
        assert!(
            outcome.cost_usd.is_none(),
            "cost_usd should be None for Copilot (no pricing table)"
        );

        // Events must have been forwarded — at least a TextDelta
        let mut events = Vec::new();
        while let Ok(env) = rx.try_recv() {
            events.push(env.event);
        }
        assert!(
            events.iter().any(|e| matches!(e, Event::TextDelta { .. })),
            "expected at least one TextDelta event; got: {events:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Error path: non-zero exit → Failed
    // -----------------------------------------------------------------------

    /// fake-copilot-error.sh exits with code 2.
    /// execute() must return Failed with summary containing "subprocess exited" or "2".
    #[tokio::test]
    async fn nonzero_exit_returns_failed() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-copilot-error.sh"), cancel);

        let (sink, _rx) = broadcast::channel(64);

        let outcome: ExecuteOutcome = backend
            .execute(req, sink)
            .await
            .expect("execute must return Ok even when subprocess exits non-zero");

        assert_eq!(
            outcome.status,
            StepStatus::Failed,
            "expected Failed, got {:?}",
            outcome.status
        );

        let summary_lower = outcome.summary.to_lowercase();
        assert!(
            summary_lower.contains("subprocess exited") || summary_lower.contains('2'),
            "expected summary to contain 'subprocess exited' or '2', got: {:?}",
            outcome.summary
        );
    }

    // -----------------------------------------------------------------------
    // Cancel mid-stream → Failed with "cancelled" summary
    // -----------------------------------------------------------------------

    /// fake-copilot-trap.sh traps SIGTERM. Cancel after 200ms.
    /// execute() must return Failed with "cancelled" in the summary.
    #[tokio::test]
    async fn cancel_mid_stream_returns_failed_with_cancelled_summary() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-copilot-trap.sh"), cancel.clone());

        let (sink, _rx) = broadcast::channel(64);

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

        assert_eq!(
            outcome.summary, "cancelled",
            "expected summary == 'cancelled', got: {:?}",
            outcome.summary
        );
    }

    // -----------------------------------------------------------------------
    // Synthetic events: StepStarted first, StepComplete last
    // -----------------------------------------------------------------------

    /// CopilotCliBackend::execute must emit StepStarted as the FIRST event
    /// and StepComplete as the LAST event, wrapping any parser events.
    #[tokio::test]
    async fn execute_emits_step_started_before_parse_and_step_complete_after() {
        let cancel = CancellationToken::new();
        let (backend, req) = make_request(fixture_bin("fake-copilot-pass.sh"), cancel);

        let (sink, mut rx) = broadcast::channel(256);

        let outcome = backend
            .execute(req, sink)
            .await
            .expect("execute must not error on happy path");

        let mut kinds: Vec<&str> = Vec::new();
        while let Ok(env) = rx.try_recv() {
            match env.event {
                Event::StepStarted { .. } => kinds.push("StepStarted"),
                Event::StepComplete { .. } => kinds.push("StepComplete"),
                _ => {}
            }
        }

        assert!(
            kinds.first() == Some(&"StepStarted"),
            "first event kind should be StepStarted, got: {kinds:?}"
        );
        assert!(
            kinds.last() == Some(&"StepComplete"),
            "last event kind should be StepComplete, got: {kinds:?}"
        );
        assert_eq!(outcome.status, StepStatus::Passed);
    }
}
