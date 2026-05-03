//! Async permission gate — wraps [`ConfigGate`] and adds a bus-based
//! decision channel for the `Prompt` branch.
//!
//! When `evaluate` returns [`GateOutcome::Prompt`], [`AsyncGate::evaluate_async`]:
//!
//! 1. Mints a ULID `request_id`.
//! 2. Subscribes to the bus (per-call subscription — simpler than a
//!    long-running listener task; no shared routing map needed).
//! 3. Publishes [`Event::PermissionRequest`] on the [`EventBus`].
//! 4. Awaits a matching [`Event::PermissionResolved`] envelope via
//!    `tokio::select!` over: (a) per-call bus subscriber filtered by
//!    `request_id`, (b) configurable timeout, (c) external
//!    [`CancellationToken`].
//!
//! On timeout a synthetic [`Event::PermissionResolved`] is published so the
//! audit log and UI see the closure event. On cancellation no synthetic event
//! is published (the run is shutting down — no further audit value).

use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

use crate::events::{Event, EventBus, EventEnvelope, PermissionDecision, PermissionSource};
use crate::permissions::config::{OnTimeout, PermissionsConfig};
use crate::permissions::gate::{ConfigGate, GateOutcome, PermissionGate};

// ---------------------------------------------------------------------------
// Public API
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

impl std::fmt::Debug for AsyncGate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `ConfigGate` and `EventBus` do not implement `Debug`, so we surface
        // the fields that are most useful for diagnostics and omit the rest
        // via `finish_non_exhaustive`.
        f.debug_struct("AsyncGate")
            .field("timeout", &self.timeout)
            .field("agent", &self.agent)
            .finish_non_exhaustive()
    }
}

impl AsyncGate {
    /// Create a new `AsyncGate`.
    ///
    /// - `config`: permissions config (allow/deny lists + settings).
    /// - `bus`: the in-process event bus.
    /// - `timeout`: how long to wait for a human decision before applying
    ///   `default_on_timeout` (passed per-call so tests can override).
    /// - `agent`: agent name emitted in `PermissionRequest` envelopes.
    pub fn new(config: PermissionsConfig, bus: EventBus, timeout: Duration, agent: String) -> Self {
        let inner = ConfigGate::new(config);
        Self {
            inner,
            bus,
            timeout,
            agent,
        }
    }

    /// Evaluate `(tool, arg)` asynchronously.
    ///
    /// - Non-prompt outcomes are returned immediately without touching the bus.
    /// - For `Prompt` outcomes, publishes a `PermissionRequest` and awaits
    ///   a matching `PermissionResolved` within `self.timeout`.
    ///
    /// # Arguments
    ///
    /// - `run_id`: forwarded into published envelopes.
    /// - `step_id`: forwarded into published envelopes (`None` is fine).
    /// - `cancel`: cancelled externally to abort the pending decision early.
    /// - `default_on_timeout`: what to do when no decision arrives in time.
    pub async fn evaluate_async(
        &self,
        tool: &str,
        arg: &str,
        run_id: &str,
        step_id: Option<&str>,
        cancel: CancellationToken,
        default_on_timeout: OnTimeout,
    ) -> GateOutcome {
        // 1. Run sync evaluation first.
        match self.inner.evaluate(tool, arg) {
            // Non-prompt: return immediately, no bus interaction.
            non_prompt @ (GateOutcome::AnnotateAllow { .. } | GateOutcome::AnnotateDeny { .. }) => {
                non_prompt
            }
            GateOutcome::Prompt { risk } => {
                // 2. Mint a fresh request_id.
                let request_id = ulid::Ulid::new().to_string();

                // 3. Subscribe BEFORE publishing so we never miss a fast reply.
                let mut subscriber = self.bus.subscribe();

                // Derive scope from tool family (v1: tool name is the scope).
                let scope = tool.to_string();

                // 4. Publish PermissionRequest.
                self.bus.publish(EventEnvelope::now(
                    run_id.to_string(),
                    step_id.map(str::to_string),
                    Event::PermissionRequest {
                        request_id: request_id.clone(),
                        agent: self.agent.clone(),
                        tool: tool.to_string(),
                        arg: arg.to_string(),
                        scope,
                        risk,
                        // TODO(P.2.4): The orchestrator passes the LLM's "why"
                        // string here once gate-side context is wired (e.g. tool
                        // description from the pipeline's tool registry).  For now,
                        // the reason is empty and the UI displays the scope/risk
                        // fields instead.
                        reason: String::new(),
                    },
                ));

                // 5. Wait for matching PermissionResolved (or timeout/cancel).
                match wait_for_decision(&mut subscriber, &request_id, self.timeout, cancel).await {
                    WaitResult::Resolved { decision, source } => match decision {
                        PermissionDecision::AllowOnce | PermissionDecision::AllowSession => {
                            GateOutcome::AnnotateAllow { source }
                        }
                        PermissionDecision::Deny | PermissionDecision::TimedOut => {
                            GateOutcome::AnnotateDeny { source }
                        }
                    },

                    WaitResult::TimedOut => {
                        // Publish synthetic resolved event for audit/UI.
                        self.bus.publish(EventEnvelope::now(
                            run_id.to_string(),
                            step_id.map(str::to_string),
                            Event::PermissionResolved {
                                request_id,
                                decision: PermissionDecision::TimedOut,
                                source: PermissionSource::Timeout,
                            },
                        ));

                        match default_on_timeout {
                            OnTimeout::Allow => GateOutcome::AnnotateAllow {
                                source: PermissionSource::Timeout,
                            },
                            OnTimeout::Deny => GateOutcome::AnnotateDeny {
                                source: PermissionSource::Timeout,
                            },
                        }
                    }

                    WaitResult::Cancelled => GateOutcome::AnnotateDeny {
                        source: PermissionSource::Cancelled,
                    },
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal wait helper
// ---------------------------------------------------------------------------

enum WaitResult {
    Resolved {
        decision: PermissionDecision,
        source: PermissionSource,
    },
    TimedOut,
    Cancelled,
}

/// Wait for a `PermissionResolved` envelope whose `request_id` matches, or
/// until the timeout fires, or the cancel token is cancelled.
///
/// Envelopes with a different `request_id` are silently ignored and the loop
/// continues waiting.
async fn wait_for_decision(
    subscriber: &mut tokio::sync::broadcast::Receiver<EventEnvelope>,
    request_id: &str,
    timeout: Duration,
    cancel: CancellationToken,
) -> WaitResult {
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            biased;

            // Cancellation has highest priority.
            _ = cancel.cancelled() => return WaitResult::Cancelled,

            // Timeout.
            _ = &mut deadline => return WaitResult::TimedOut,

            // Bus event.
            recv = subscriber.recv() => match recv {
                Ok(envelope) => {
                    if let Event::PermissionResolved {
                        request_id: ref rid,
                        decision,
                        source,
                    } = envelope.event
                        && rid == request_id
                    {
                        return WaitResult::Resolved { decision, source };
                    }
                    // Different request_id or other event variant — keep waiting.
                }
                Err(RecvError::Lagged(_)) => {
                    // We lagged; some events were dropped from the broadcast
                    // buffer. Keep waiting — the matching resolved event may
                    // still arrive.
                }
                Err(RecvError::Closed) => {
                    // Bus shut down — treat as cancellation.
                    return WaitResult::Cancelled;
                }
            },
        }
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
            settings: PermissionsSettings { default_on_timeout },
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
                scope,
                ..
            } => {
                assert!(!request_id.is_empty(), "request_id must not be empty");
                assert_eq!(request_id.len(), 26, "ULID must be 26 chars");
                ulid::Ulid::from_string(&request_id).expect("request_id must be a parseable ULID");
                assert_eq!(agent, "test-agent");
                assert_eq!(tool, "CustomTool");
                assert_eq!(arg, "x");
                assert_eq!(risk, PermissionRisk::Low);
                assert_eq!(scope, "CustomTool", "scope must equal tool name");
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
            gate.evaluate_async("CustomTool", "x", "run-1", None, cancel2, OnTimeout::Deny)
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
        assert!(
            still_pending,
            "task must stay pending after mismatched request_id"
        );

        // Publish correct id to unblock background task.
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
            gate.evaluate_async("CustomTool", "x", "run-1", None, cancel, OnTimeout::Deny)
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

        // Verify synthetic PermissionResolved was published.
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
            gate.evaluate_async("CustomTool", "x", "run-1", None, cancel, OnTimeout::Allow)
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
            gate.evaluate_async("CustomTool", "x", "run-1", None, cancel2, OnTimeout::Deny)
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

        // No envelopes should be published — verify within a short window.
        let nothing = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await;
        assert!(
            nothing.is_err(),
            "no bus envelopes expected for non-prompt outcome"
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: concurrent_calls_are_independently_resolved
    //
    // Two concurrent evaluate_async calls on the same gate must each receive
    // ONLY their own PermissionResolved decision.  Per-call subscriber pattern
    // safety: even when resolutions arrive in reversed order, each task gets
    // its own decision and is not accidentally unblocked by the other's event.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // Test P.2.3-1: session_decision_caches_pattern_for_subsequent_calls
    //
    // First call to ("Bash", "ls -la") prompts. User resolves with AllowSession.
    // Second identical call returns AnnotateAllow { SessionAllowlist } WITHOUT
    // publishing a new PermissionRequest.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn session_decision_caches_pattern_for_subsequent_calls() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = Arc::new(AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        ));

        // --- First call: prompts the user ---
        let g1 = Arc::clone(&gate);
        let cancel1 = CancellationToken::new();
        let cancel1c = cancel1.clone();
        let handle1 = tokio::spawn(async move {
            g1.evaluate_async("Bash", "ls -la", "run-session", None, cancel1c, OnTimeout::Deny)
                .await
        });

        // Receive the PermissionRequest.
        let request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out waiting for PermissionRequest")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        // Resolve with AllowSession.
        bus.publish(EventEnvelope::now(
            "run-session".to_string(),
            None,
            Event::PermissionResolved {
                request_id,
                decision: PermissionDecision::AllowSession,
                source: PermissionSource::User,
            },
        ));

        let outcome1 = tokio::time::timeout(Duration::from_millis(500), handle1)
            .await
            .expect("task1 timed out")
            .expect("task1 panicked");

        assert_eq!(
            outcome1,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::User
            },
            "first call must resolve to AnnotateAllow{{User}}"
        );

        // --- Second call: must hit session cache, no bus publish ---
        let cancel2 = CancellationToken::new();
        let outcome2 = gate
            .evaluate_async("Bash", "ls -la", "run-session", None, cancel2, OnTimeout::Deny)
            .await;

        assert_eq!(
            outcome2,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::SessionAllowlist
            },
            "second call must resolve immediately via session cache"
        );

        // Verify no new PermissionRequest was published (within a short window).
        let nothing = tokio::time::timeout(Duration::from_millis(30), async {
            loop {
                let env = rx.recv().await.expect("bus closed");
                if matches!(env.event, Event::PermissionRequest { .. }) {
                    return true; // found one — unexpected
                }
            }
        })
        .await;

        assert!(
            nothing.is_err(),
            "no PermissionRequest should be published for the cached second call"
        );
    }

    // -----------------------------------------------------------------------
    // Test P.2.3-2: session_pattern_is_exact_arg_match
    //
    // Session entry for ("Bash", "ls -la") does NOT match ("Bash", "ls -la /tmp").
    // A new PermissionRequest must be published for the different arg.
    //
    // DESIGN NOTE: session matching is exact-arg, not glob (Q2 minimality).
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn session_pattern_is_exact_arg_match() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = Arc::new(AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        ));

        // Step 1: cache ("Bash", "ls -la") via AllowSession.
        let g1 = Arc::clone(&gate);
        let cancel1 = CancellationToken::new();
        let cancel1c = cancel1.clone();
        let handle1 = tokio::spawn(async move {
            g1.evaluate_async("Bash", "ls -la", "run-exact", None, cancel1c, OnTimeout::Deny)
                .await
        });

        let request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        bus.publish(EventEnvelope::now(
            "run-exact".to_string(),
            None,
            Event::PermissionResolved {
                request_id,
                decision: PermissionDecision::AllowSession,
                source: PermissionSource::User,
            },
        ));

        tokio::time::timeout(Duration::from_millis(500), handle1)
            .await
            .expect("task1 timed out")
            .expect("task1 panicked");

        // Step 2: different arg — must NOT hit session cache.
        let g2 = Arc::clone(&gate);
        let cancel2 = CancellationToken::new();
        let cancel2c = cancel2.clone();
        let handle2 = tokio::spawn(async move {
            g2.evaluate_async(
                "Bash",
                "ls -la /tmp", // different arg — exact match only (Q2)
                "run-exact",
                None,
                cancel2c,
                OnTimeout::Deny,
            )
            .await
        });

        // A new PermissionRequest must appear.
        let got_new_request = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                let env = rx.recv().await.expect("bus closed");
                if matches!(env.event, Event::PermissionRequest { .. }) {
                    return true;
                }
            }
        })
        .await;

        assert!(
            got_new_request.is_ok(),
            "different arg must not hit session cache — new PermissionRequest expected"
        );

        cancel2.cancel();
        let _ = handle2.await;
    }

    // -----------------------------------------------------------------------
    // Test P.2.3-3: run_complete_clears_session_allowlist
    //
    // After caching an entry, publish Event::RunComplete for that run_id
    // (via the envelope's run_id field). A subsequent identical call must
    // prompt again.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn run_complete_clears_session_allowlist() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = Arc::new(AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        ));

        // Step 1: cache via AllowSession.
        let g1 = Arc::clone(&gate);
        let cancel1 = CancellationToken::new();
        let cancel1c = cancel1.clone();
        let handle1 = tokio::spawn(async move {
            g1.evaluate_async("Bash", "ls", "run-clear", None, cancel1c, OnTimeout::Deny)
                .await
        });

        let request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        bus.publish(EventEnvelope::now(
            "run-clear".to_string(),
            None,
            Event::PermissionResolved {
                request_id,
                decision: PermissionDecision::AllowSession,
                source: PermissionSource::User,
            },
        ));

        tokio::time::timeout(Duration::from_millis(500), handle1)
            .await
            .expect("task1 timed out")
            .expect("task1 panicked");

        // Verify cached — second call should return SessionAllowlist immediately.
        let cancel_check = CancellationToken::new();
        let outcome_cached = gate
            .evaluate_async("Bash", "ls", "run-clear", None, cancel_check, OnTimeout::Deny)
            .await;
        assert_eq!(
            outcome_cached,
            GateOutcome::AnnotateAllow {
                source: PermissionSource::SessionAllowlist
            },
            "must be cached before RunComplete"
        );

        // Step 2: publish RunComplete for this run_id (run_id is in the envelope).
        // NOTE: Event::RunComplete has no run_id field — run_id lives in the
        // EventEnvelope wrapper. The listener task must read envelope.run_id.
        bus.publish(EventEnvelope::now(
            "run-clear".to_string(),
            None,
            Event::RunComplete {
                status: crate::events::RunStatus::Completed,
                duration_ms: 100,
                summary: "done".to_string(),
            },
        ));

        // Give the listener task time to process the event.
        tokio::time::sleep(Duration::from_millis(30)).await;

        // Step 3: same call must now prompt again (cache was cleared).
        let g3 = Arc::clone(&gate);
        let cancel3 = CancellationToken::new();
        let cancel3c = cancel3.clone();
        let handle3 = tokio::spawn(async move {
            g3.evaluate_async("Bash", "ls", "run-clear", None, cancel3c, OnTimeout::Deny)
                .await
        });

        let got_new_request = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                let env = rx.recv().await.expect("bus closed");
                if matches!(env.event, Event::PermissionRequest { .. }) {
                    return true;
                }
            }
        })
        .await;

        assert!(
            got_new_request.is_ok(),
            "RunComplete must clear session cache — new PermissionRequest expected"
        );

        cancel3.cancel();
        let _ = handle3.await;
    }

    // -----------------------------------------------------------------------
    // Test P.2.3-4: cross_run_isolation
    //
    // Session entry cached under run_id_1 must NOT be visible under run_id_2.
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn cross_run_isolation() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let gate = Arc::new(AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        ));

        // Step 1: cache ("Bash", "ls -la") under run-A via AllowSession.
        let g1 = Arc::clone(&gate);
        let cancel1 = CancellationToken::new();
        let cancel1c = cancel1.clone();
        let handle1 = tokio::spawn(async move {
            g1.evaluate_async("Bash", "ls -la", "run-A", None, cancel1c, OnTimeout::Deny)
                .await
        });

        let request_id = loop {
            let env = tokio::time::timeout(Duration::from_millis(500), rx.recv())
                .await
                .expect("timed out")
                .expect("bus closed");
            if let Event::PermissionRequest { request_id, .. } = env.event {
                break request_id;
            }
        };

        bus.publish(EventEnvelope::now(
            "run-A".to_string(),
            None,
            Event::PermissionResolved {
                request_id,
                decision: PermissionDecision::AllowSession,
                source: PermissionSource::User,
            },
        ));

        tokio::time::timeout(Duration::from_millis(500), handle1)
            .await
            .expect("task1 timed out")
            .expect("task1 panicked");

        // Step 2: same tool/arg under run-B must NOT hit the session cache.
        let g2 = Arc::clone(&gate);
        let cancel2 = CancellationToken::new();
        let cancel2c = cancel2.clone();
        let handle2 = tokio::spawn(async move {
            g2.evaluate_async("Bash", "ls -la", "run-B", None, cancel2c, OnTimeout::Deny)
                .await
        });

        // A new PermissionRequest must appear for run-B.
        let got_new_request = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                let env = rx.recv().await.expect("bus closed");
                if let Event::PermissionRequest { .. } = env.event {
                    return true;
                }
            }
        })
        .await;

        assert!(
            got_new_request.is_ok(),
            "run-B must not share session state with run-A"
        );

        cancel2.cancel();
        let _ = handle2.await;
    }

    #[tokio::test]
    async fn concurrent_calls_are_independently_resolved() {
        use std::sync::Arc;

        let bus = EventBus::new();
        let mut sub = bus.subscribe();
        let gate = Arc::new(AsyncGate::new(
            empty_config(OnTimeout::Deny),
            bus.clone(),
            Duration::from_secs(5),
            "test-agent".to_string(),
        ));

        let cancel = CancellationToken::new();

        // Spawn two concurrent evaluate_async calls on the same gate.
        let g1 = Arc::clone(&gate);
        let c1 = cancel.clone();
        let task1 = tokio::spawn(async move {
            g1.evaluate_async("ToolOne", "x", "run-c", Some("step-1"), c1, OnTimeout::Deny)
                .await
        });

        let g2 = Arc::clone(&gate);
        let c2 = cancel.clone();
        let task2 = tokio::spawn(async move {
            g2.evaluate_async("ToolTwo", "y", "run-c", Some("step-2"), c2, OnTimeout::Deny)
                .await
        });

        // Collect both PermissionRequest envelopes and capture their request_ids.
        let mut req_ids: Vec<(String, String)> = Vec::new(); // (tool, request_id)
        while req_ids.len() < 2 {
            let envelope = tokio::time::timeout(Duration::from_millis(500), sub.recv())
                .await
                .expect("timed out waiting for PermissionRequest envelopes")
                .expect("bus closed");
            if let Event::PermissionRequest {
                request_id, tool, ..
            } = envelope.event
            {
                req_ids.push((tool, request_id));
            }
        }

        let id_one = req_ids
            .iter()
            .find(|(t, _)| t == "ToolOne")
            .map(|(_, id)| id.clone())
            .expect("PermissionRequest for ToolOne not found");
        let id_two = req_ids
            .iter()
            .find(|(t, _)| t == "ToolTwo")
            .map(|(_, id)| id.clone())
            .expect("PermissionRequest for ToolTwo not found");

        // Publish resolutions in REVERSED order: ToolTwo's decision first.
        bus.publish(EventEnvelope::now(
            "run-c".to_string(),
            Some("step-2".to_string()),
            Event::PermissionResolved {
                request_id: id_two,
                decision: PermissionDecision::Deny,
                source: PermissionSource::User,
            },
        ));
        bus.publish(EventEnvelope::now(
            "run-c".to_string(),
            Some("step-1".to_string()),
            Event::PermissionResolved {
                request_id: id_one,
                decision: PermissionDecision::AllowOnce,
                source: PermissionSource::User,
            },
        ));

        let r1 = tokio::time::timeout(Duration::from_millis(500), task1)
            .await
            .expect("task1 timed out")
            .expect("task1 panicked");
        let r2 = tokio::time::timeout(Duration::from_millis(500), task2)
            .await
            .expect("task2 timed out")
            .expect("task2 panicked");

        assert!(
            matches!(
                r1,
                GateOutcome::AnnotateAllow {
                    source: PermissionSource::User
                }
            ),
            "task1 (ToolOne, AllowOnce) → AnnotateAllow{{User}}, got {r1:?}"
        );
        assert!(
            matches!(
                r2,
                GateOutcome::AnnotateDeny {
                    source: PermissionSource::User
                }
            ),
            "task2 (ToolTwo, Deny) → AnnotateDeny{{User}}, got {r2:?}"
        );
    }
}
