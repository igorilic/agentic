#![cfg(feature = "testing")]

use std::path::PathBuf;

use agentic_core::{
    Backend, Event, ExecuteRequest, ModelId, RunId, ScriptedBackend, StepId, StepStatus, ToolName,
    WorkspaceRef,
};
use tokio_util::sync::CancellationToken;

fn default_request() -> (ExecuteRequest, CancellationToken) {
    let cancel = CancellationToken::new();
    let req = ExecuteRequest {
        workspace: WorkspaceRef {
            id: "ws1".to_string(),
            root_path: PathBuf::from("/tmp/ws1"),
        },
        run_id: RunId("run1".to_string()),
        step_id: StepId("step1".to_string()),
        agent_name: "test".to_string(),
        agent_prompt: "prompt".to_string(),
        user_context: "ctx".to_string(),
        model: Some(ModelId("fake-model".to_string())),
        tools: vec![ToolName("Read".to_string())],
        cwd: PathBuf::from("/tmp/ws1"),
        timeout: None,
        cancel: cancel.clone(),
    };
    (req, cancel)
}

fn sample_script_5_events() -> Vec<Event> {
    vec![
        Event::StepStarted {
            agent: "test".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::TextDelta { content: "hello".to_string() },
        Event::TextDelta { content: "world".to_string() },
        Event::TextDelta { content: "!".to_string() },
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "done".to_string(),
            token_usage: agentic_core::TokenUsage::default(),
            cost_usd: None,
            duration_ms: 100,
        },
    ]
}

#[tokio::test]
async fn sink_receives_all_5_events_in_order() {
    let (req, _cancel) = default_request();
    let (sink, mut rx) = tokio::sync::broadcast::channel(32);

    let backend = ScriptedBackend::new(sample_script_5_events());
    backend.execute(req, sink).await.expect("execute");

    // Drain receiver; expect 5 envelopes in order.
    let mut received = Vec::new();
    while let Ok(env) = rx.try_recv() {
        received.push(env);
    }
    assert_eq!(received.len(), 5, "expected 5 envelopes, got {}", received.len());

    // Spot-check order via variant matching.
    assert!(matches!(received[0].event, Event::StepStarted { .. }));
    assert!(matches!(received[1].event, Event::TextDelta { .. }));
    assert!(matches!(received[4].event, Event::StepComplete { .. }));
}

#[tokio::test]
async fn respects_cancellation_and_drops_remaining_events() {
    let (req, cancel) = default_request();
    cancel.cancel(); // Pre-cancel

    let (sink, mut rx) = tokio::sync::broadcast::channel(32);
    let backend = ScriptedBackend::new(sample_script_5_events());
    let outcome = backend.execute(req, sink).await.expect("execute");

    assert_eq!(outcome.status, StepStatus::Failed);
    assert_eq!(outcome.summary, "cancelled");

    // No events should have been emitted (cancel checked before each event).
    let count = {
        let mut n = 0;
        while rx.try_recv().is_ok() {
            n += 1;
        }
        n
    };
    assert_eq!(count, 0, "expected 0 events after pre-cancel, got {count}");
}

#[tokio::test]
async fn normal_completion_returns_passed_outcome() {
    let (req, _cancel) = default_request();
    let (sink, _rx) = tokio::sync::broadcast::channel(32);

    let backend = ScriptedBackend::new(sample_script_5_events());
    let outcome = backend.execute(req, sink).await.expect("execute");
    assert_eq!(outcome.status, StepStatus::Passed);
}

#[tokio::test]
async fn unrecoverable_error_in_script_produces_failed_outcome() {
    let (req, _cancel) = default_request();
    let (sink, _rx) = tokio::sync::broadcast::channel(32);

    let script = vec![
        Event::StepStarted {
            agent: "test".to_string(),
            model: ModelId("fake".to_string()),
        },
        Event::Error {
            code: "boom".to_string(),
            message: "something bad".to_string(),
            recoverable: false,
            retry_after_ms: None,
        },
    ];
    let backend = ScriptedBackend::new(script);
    let outcome = backend.execute(req, sink).await.expect("execute");
    assert_eq!(outcome.status, StepStatus::Failed);
}
