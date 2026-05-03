//! Async permission gate — wraps [`ConfigGate`] and adds a bus-based
//! decision channel for the `Prompt` branch.
//!
//! See P.2.2 in docs/redesign/todo.md for the full design contract.

use std::time::Duration;

use tokio_util::sync::CancellationToken;

use crate::events::EventBus;
use crate::permissions::config::{OnTimeout, PermissionsConfig};
use crate::permissions::gate::{ConfigGate, GateOutcome, PermissionGate};

// ---------------------------------------------------------------------------
// Public API — STUBS (GREEN implementation to follow)
// ---------------------------------------------------------------------------

/// Async permission gate.
///
/// Wraps [`ConfigGate`] and adds bus-based interactive prompting for the
/// `Prompt` branch. Non-prompt outcomes (allow/deny from config) are returned
/// synchronously without any bus interaction.
pub struct AsyncGate {
    inner: ConfigGate,
    bus: EventBus,
    timeout: Duration,
    agent: String,
}

impl AsyncGate {
    /// Create a new `AsyncGate`.
    pub fn new(config: PermissionsConfig, bus: EventBus, timeout: Duration, agent: String) -> Self {
        let inner = ConfigGate::new(config);
        Self {
            inner,
            bus,
            timeout,
            agent,
        }
    }

    /// Evaluate `(tool, arg)` asynchronously. STUB — panics.
    pub async fn evaluate_async(
        &self,
        _tool: &str,
        _arg: &str,
        _run_id: &str,
        _step_id: Option<&str>,
        _cancel: CancellationToken,
        _default_on_timeout: OnTimeout,
    ) -> GateOutcome {
        todo!("P.2.2 GREEN: implement evaluate_async")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{
        Event, EventBus, EventEnvelope, PermissionDecision, PermissionRisk, PermissionSource,
    };
    use crate::permissions::config::{PermissionRule, PermissionsConfig, PermissionsSettings};

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn empty_config(default_on_timeout: OnTimeout) -> PermissionsConfig {
        PermissionsConfig {
            allowlist: vec![],
            denylist: vec![],
            settings: PermissionsSettings {
                default_on_timeout,
            },
        }
    }

    fn allow_config(pattern: &str) -> PermissionsConfig {
        PermissionsConfig {
            allowlist: vec![PermissionRule {
                pattern: pattern.to_string(),
            }],
            denylist: vec![],
            settings: PermissionsSettings::default(),
        }
    }

    fn test_gate(
        config: PermissionsConfig,
        timeout_ms: u64,
    ) -> (AsyncGate, tokio::sync::broadcast::Receiver<EventEnvelope>) {
        let bus = EventBus::new();
        let subscriber = bus.subscribe();
        let gate = AsyncGate::new(
            config,
            bus,
            Duration::from_millis(timeout_ms),
            "test-agent".to_string(),
        );
        (gate, subscriber)
    }

    async fn next_resolved(
        rx: &mut tokio::sync::broadcast::Receiver<EventEnvelope>,
    ) -> (String, PermissionDecision, PermissionSource) {
        loop {
            let env = rx.recv().await.expect("bus closed unexpectedly");
            if let Event::PermissionResolved {
                request_id,
                decision,
                source,
            } = env.event
            {
                return (request_id, decision, source);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: prompt_emits_permission_request_envelope
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn prompt_emits_permission_request_envelope() {
        let (gate, mut rx) = test_gate(empty_config(OnTimeout::Deny), 5_000);

        let cancel = CancellationToken::new();
        let cancel2 = cancel.clone();
        let _task = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                Some("step-1"),
                cancel2,
                OnTimeout::Deny,
            )
            .await
        });

        let envelope = tokio::time::timeout(Duration::from_millis(500), rx.recv())
            .await
            .expect("timed out waiting for PermissionRequest")
            .expect("bus closed");

        match envelope.event {
            Event::PermissionRequest {
                request_id,
                agent,
                tool,
                arg,
                risk,
                ..
            } => {
                assert!(!request_id.is_empty(), "request_id must not be empty");
                assert_eq!(request_id.len(), 26, "ULID must be 26 chars");
                ulid::Ulid::from_string(&request_id)
                    .expect("request_id must be a parseable ULID");
                assert_eq!(agent, "test-agent");
                assert_eq!(tool, "CustomTool");
                assert_eq!(arg, "x");
                assert_eq!(risk, PermissionRisk::Low);
            }
            other => panic!("expected PermissionRequest, got: {other:?}"),
        }

        cancel.cancel();
    }

    // -----------------------------------------------------------------------
    // Test 2: decision_resolves_pending_request
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn decision_resolves_pending_request() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        );

        let cancel = CancellationToken::new();
        let cancel2 = cancel.clone();
        let handle = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                Some("step-1"),
                cancel2,
                OnTimeout::Deny,
            )
            .await
        });

        let request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out waiting for PermissionRequest")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        bus.publish(EventEnvelope::now(
            "run-1".to_string(),
            Some("step-1".to_string()),
            Event::PermissionResolved {
                request_id: request_id.clone(),
                decision: PermissionDecision::AllowOnce,
                source: PermissionSource::User,
            },
        ));

        let outcome = tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("timed out awaiting task")
            .expect("task panicked");

        assert_eq!(
            outcome,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::User
            },
            "AllowOnce decision from User must resolve to AnnotateAllow {{ User }}"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: mismatched_request_id_is_ignored
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn mismatched_request_id_is_ignored() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        );

        let cancel = CancellationToken::new();
        let cancel2 = cancel.clone();
        let handle = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                None,
                cancel2,
                OnTimeout::Deny,
            )
            .await
        });

        let correct_request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out waiting for PermissionRequest")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        // Publish a resolved with a DIFFERENT request_id.
        bus.publish(EventEnvelope::now(
            "run-1".to_string(),
            None,
            Event::PermissionResolved {
                request_id: "01WRONG00000000000000000000".to_string(),
                decision: PermissionDecision::Deny,
                source: PermissionSource::User,
            },
        ));

        // The task should still be pending — sleep wins.
        let still_pending = tokio::select! {
            _ = handle => false,
            _ = tokio::time::sleep(Duration::from_millis(50)) => true,
        };
        assert!(still_pending, "task must stay pending after mismatched request_id");

        // Publish correct id to unblock (consumed by the background task).
        bus.publish(EventEnvelope::now(
            "run-1".to_string(),
            None,
            Event::PermissionResolved {
                request_id: correct_request_id,
                decision: PermissionDecision::AllowOnce,
                source: PermissionSource::User,
            },
        ));

        cancel.cancel();
    }

    // -----------------------------------------------------------------------
    // Test 4: timeout_resolves_to_deny_by_default
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn timeout_resolves_to_deny_by_default() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus,
            Duration::from_millis(50),
            "test-agent".to_string(),
        );

        let cancel = CancellationToken::new();
        let handle = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                None,
                cancel,
                OnTimeout::Deny,
            )
            .await
        });

        let outcome = tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("task took too long")
            .expect("task panicked");

        assert_eq!(
            outcome,
            GateOutcome::AnnotateDeny {
                source: PermissionSource::Timeout
            },
        );

        let (_, decision, source) = next_resolved(&mut rx).await;
        assert_eq!(decision, PermissionDecision::TimedOut);
        assert_eq!(source, PermissionSource::Timeout);
    }

    // -----------------------------------------------------------------------
    // Test 5: timeout_resolves_to_allow_when_configured
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn timeout_resolves_to_allow_when_configured() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = AsyncGate::new(
            empty_config(OnTimeout::Allow),
            bus,
            Duration::from_millis(50),
            "test-agent".to_string(),
        );

        let cancel = CancellationToken::new();
        let handle = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                None,
                cancel,
                OnTimeout::Allow,
            )
            .await
        });

        let outcome = tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("task took too long")
            .expect("task panicked");

        assert_eq!(
            outcome,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::Timeout
            },
        );

        let (_, decision, source) = next_resolved(&mut rx).await;
        assert_eq!(decision, PermissionDecision::TimedOut);
        assert_eq!(source, PermissionSource::Timeout);
    }

    // -----------------------------------------------------------------------
    // Test 6: cancellation_drops_pending
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn cancellation_drops_pending() {
        let (gate, _rx) = test_gate(empty_config(OnTimeout::Deny), 5_000);

        let cancel = CancellationToken::new();
        let cancel2 = cancel.clone();
        let handle = tokio::spawn(async move {
            gate.evaluate_async(
                "CustomTool",
                "x",
                "run-1",
                None,
                cancel2,
                OnTimeout::Deny,
            )
            .await
        });

        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();

        let outcome = tokio::time::timeout(Duration::from_millis(500), handle)
            .await
            .expect("task took too long after cancellation")
            .expect("task panicked");

        assert_eq!(
            outcome,
            GateOutcome::AnnotateDeny {
                source: PermissionSource::Cancelled
            },
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: non_prompt_outcome_skips_async_path
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn non_prompt_outcome_skips_async_path() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = AsyncGate::new(
            allow_config("Bash(ls)"),
            bus,
            Duration::from_secs(5),
            "test-agent".to_string(),
        );

        let cancel = CancellationToken::new();
        let outcome = gate
            .evaluate_async("Bash", "ls", "run-1", None, cancel, OnTimeout::Deny)
            .await;

        assert_eq!(
            outcome,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::AllowlistConfig
            },
        );

        let nothing = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(
            nothing.is_err(),
            "no bus envelopes expected for non-prompt outcome"
        );
    }
}
