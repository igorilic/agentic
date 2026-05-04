use std::collections::HashMap;
use std::sync::Arc;

use agentic_core::events::{EventBus, EventEnvelope, EventHistoryBuffer};
use tauri::{AppHandle, Emitter, Runtime, State};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// Per-app shared state holding the EventBus. Created at app setup time and
/// stored in Tauri's managed state.
///
/// The `forwarder` slot tracks the active background task spawned by
/// `subscribe_events`. Re-invoking the command aborts the previous handle and
/// installs a new one, so the webview receives exactly one stream of envelopes
/// regardless of how many times the frontend re-attaches (e.g., during Vite
/// HMR which re-invokes on every save).
pub struct EventBusState {
    pub bus: Arc<EventBus>,
    /// Handle to the active subscriber forwarder, if any. Re-invoking
    /// `subscribe_events` aborts the previous handle and replaces it,
    /// so the webview receives exactly one stream of envelopes regardless
    /// of how many times the frontend re-attaches (e.g., during Vite HMR).
    forwarder: Mutex<Option<JoinHandle<()>>>,
    /// Active run cancellation tokens, keyed by run_id.
    cancellations: Mutex<HashMap<String, CancellationToken>>,
    /// Per-run ring buffer of recent envelopes for mid-run webview reattach.
    pub history: EventHistoryBuffer,
}

impl EventBusState {
    /// Construct from a shared bus. Safe to call from either an active
    /// tokio runtime context (e.g., `#[tokio::test]`) or a synchronous
    /// entry point (e.g., Tauri 2's `setup` hook, which runs *outside* the
    /// runtime). When no ambient runtime is detected, the constructor
    /// transparently enters `tauri::async_runtime` to register the
    /// history-buffer task — without this fallback, `tokio::spawn` inside
    /// `EventHistoryBuffer::spawn_default` panics with "no reactor running".
    pub fn new(bus: Arc<EventBus>) -> Self {
        let history = match tokio::runtime::Handle::try_current() {
            Ok(_) => EventHistoryBuffer::spawn_default(&bus),
            Err(_) => {
                tauri::async_runtime::block_on(async { EventHistoryBuffer::spawn_default(&bus) })
            }
        };
        Self {
            bus,
            forwarder: Mutex::new(None),
            cancellations: Mutex::new(HashMap::new()),
            history,
        }
    }

    /// Register a cancellation token for `run_id`.
    pub async fn register_cancellation(&self, run_id: String, token: CancellationToken) {
        self.cancellations.lock().await.insert(run_id, token);
    }

    /// Cancel the run identified by `run_id`. Returns `true` if a token was
    /// found and cancelled, `false` if no such run is registered (idempotent).
    pub async fn cancel(&self, run_id: &str) -> bool {
        let mut map = self.cancellations.lock().await;
        if let Some(token) = map.remove(run_id) {
            token.cancel();
            true
        } else {
            false
        }
    }
}

/// The frontend channel name for forwarded envelopes. Frontend listens via
/// `window.listen("agentic://event", handler)`.
pub const EVENT_CHANNEL: &str = "agentic://event";

/// Tauri command. Subscribes to the EventBus and forwards every envelope as
/// a `tauri::Event` named `agentic://event`. Spawns a background tokio task.
///
/// Re-invoking this command aborts any previously spawned forwarder and
/// replaces it with a fresh one. This prevents Vite HMR from accumulating
/// N duplicate background tasks after N hot-reloads.
///
/// Returns immediately after spawning. The frontend MUST register a listener
/// before invoking this command, or events sent before listener registration
/// will be lost.
#[tauri::command]
pub async fn subscribe_events<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EventBusState>,
) -> Result<(), String> {
    let mut subscriber = state.bus.subscribe();

    // Spawn the new forwarder.
    let new_handle = tokio::spawn(async move {
        loop {
            match subscriber.recv().await {
                Ok(envelope) => {
                    if let Err(e) = app.emit(EVENT_CHANNEL, &envelope) {
                        tracing::warn!(error = %e, "subscribe_events: emit failed");
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("subscribe_events: bus closed; forwarder exiting");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "subscribe_events: lagged behind bus");
                    // continue; broadcast catches up at next call
                }
            }
        }
    });

    // Atomically swap: take any old handle and abort it, install new one.
    let mut slot = state.forwarder.lock().await;
    if let Some(old) = slot.take() {
        old.abort();
        tracing::debug!("subscribe_events: aborted previous forwarder (re-invocation)");
    }
    *slot = Some(new_handle);

    Ok(())
}

/// Tauri command. Returns the buffered event envelopes for the given `run_id`.
///
/// Called by the frontend on mount (before or alongside `subscribe_events`) to
/// pre-seed the event list with events published before the live listener
/// attached — closing the silent data-loss gap on mid-run webview attach.
#[tauri::command]
pub async fn get_event_history(
    state: State<'_, EventBusState>,
    run_id: String,
) -> Result<Vec<EventEnvelope>, String> {
    Ok(state.history.get(&run_id).await)
}

#[cfg(test)]
mod tests {
    use agentic_core::events::{
        Event, EventEnvelope, PermissionDecision, PermissionRisk, PermissionSource,
    };

    fn valid_run_id() -> String {
        ulid::Ulid::new().to_string()
    }

    /// Regression test: `Event::PermissionRequest` serialises to the JSON shape
    /// the TypeScript frontend expects. The forwarder calls `app.emit(CHANNEL, &envelope)`,
    /// which delegates serialisation to serde — so confirming the serde output here
    /// covers the entire risk surface without needing a live `AppHandle`.
    #[test]
    fn forwards_permission_request_envelope() {
        let env = EventEnvelope::now(
            valid_run_id(),
            None,
            Event::PermissionRequest {
                request_id: "req-01J".into(),
                agent: "developer".into(),
                tool: "Bash".into(),
                arg: "rm -rf node_modules".into(),
                scope: "shell.destructive".into(),
                risk: PermissionRisk::High,
                reason: "destructive shell".into(),
            },
        );

        let json = serde_json::to_value(&env).unwrap();

        // Envelope-level fields
        assert!(json["event_id"].is_string(), "event_id must be a string");
        assert!(
            json["timestamp_ms"].is_number(),
            "timestamp_ms must be a number"
        );

        // Event discriminant: tag="type", content="data", rename_all="PascalCase"
        assert_eq!(json["event"]["type"], "PermissionRequest");

        // Nested payload fields (under "data")
        let data = &json["event"]["data"];
        assert_eq!(data["request_id"], "req-01J");
        assert_eq!(data["agent"], "developer");
        assert_eq!(data["tool"], "Bash");
        assert_eq!(data["arg"], "rm -rf node_modules");
        assert_eq!(data["scope"], "shell.destructive");
        assert_eq!(data["risk"], "high");
        assert_eq!(data["reason"], "destructive shell");
    }

    /// Regression test: `Event::PermissionResolved` serialises correctly.
    #[test]
    fn forwards_permission_resolved_envelope() {
        let env = EventEnvelope::now(
            valid_run_id(),
            Some("step-01".into()),
            Event::PermissionResolved {
                request_id: "req-01J".into(),
                decision: PermissionDecision::AllowOnce,
                source: PermissionSource::User,
            },
        );

        let json = serde_json::to_value(&env).unwrap();

        // Envelope-level fields
        assert!(json["event_id"].is_string(), "event_id must be a string");
        assert_eq!(json["step_id"], "step-01");

        // Event discriminant
        assert_eq!(json["event"]["type"], "PermissionResolved");

        // Nested payload fields
        let data = &json["event"]["data"];
        assert_eq!(data["request_id"], "req-01J");
        assert_eq!(data["decision"], "allow_once");
        assert_eq!(data["source"], "user");
    }

    /// S-3: Covers all PermissionRisk variants — rename-fragile snake_case assertions.
    #[test]
    fn permission_risk_serializes_to_snake_case() {
        fn risk_json(risk: PermissionRisk) -> serde_json::Value {
            let env = EventEnvelope::now(
                valid_run_id(),
                None,
                Event::PermissionRequest {
                    request_id: "req-x".into(),
                    agent: "developer".into(),
                    tool: "Bash".into(),
                    arg: "echo hi".into(),
                    scope: "shell".into(),
                    risk,
                    reason: "test".into(),
                },
            );
            let json = serde_json::to_value(&env).unwrap();
            json["event"]["data"]["risk"].clone()
        }

        assert_eq!(risk_json(PermissionRisk::Low), "low");
        assert_eq!(risk_json(PermissionRisk::Medium), "medium");
        assert_eq!(risk_json(PermissionRisk::High), "high");
    }

    /// S-3: Covers all PermissionDecision and PermissionSource variants — rename-fragile assertions.
    #[test]
    fn permission_decision_and_source_serialize_to_snake_case() {
        fn resolved_json(
            decision: PermissionDecision,
            source: PermissionSource,
        ) -> serde_json::Value {
            let env = EventEnvelope::now(
                valid_run_id(),
                None,
                Event::PermissionResolved {
                    request_id: "req-x".into(),
                    decision,
                    source,
                },
            );
            serde_json::to_value(&env).unwrap()
        }

        // PermissionDecision variants
        let data =
            &resolved_json(PermissionDecision::AllowOnce, PermissionSource::User)["event"]["data"];
        assert_eq!(data["decision"], "allow_once");

        let data = &resolved_json(PermissionDecision::AllowSession, PermissionSource::User)["event"]
            ["data"];
        assert_eq!(data["decision"], "allow_session");

        let data =
            &resolved_json(PermissionDecision::Deny, PermissionSource::User)["event"]["data"];
        assert_eq!(data["decision"], "deny");

        let data =
            &resolved_json(PermissionDecision::TimedOut, PermissionSource::User)["event"]["data"];
        assert_eq!(data["decision"], "timed_out");

        // PermissionSource variants
        let data =
            &resolved_json(PermissionDecision::AllowOnce, PermissionSource::User)["event"]["data"];
        assert_eq!(data["source"], "user");

        let data = &resolved_json(
            PermissionDecision::AllowOnce,
            PermissionSource::AllowlistConfig,
        )["event"]["data"];
        assert_eq!(data["source"], "allowlist_config");

        let data = &resolved_json(
            PermissionDecision::AllowOnce,
            PermissionSource::DenylistConfig,
        )["event"]["data"];
        assert_eq!(data["source"], "denylist_config");

        let data = &resolved_json(
            PermissionDecision::AllowOnce,
            PermissionSource::SessionAllowlist,
        )["event"]["data"];
        assert_eq!(data["source"], "session_allowlist");

        let data = &resolved_json(PermissionDecision::AllowOnce, PermissionSource::Timeout)["event"]
            ["data"];
        assert_eq!(data["source"], "timeout");

        let data = &resolved_json(PermissionDecision::AllowOnce, PermissionSource::Cancelled)["event"]
            ["data"];
        assert_eq!(data["source"], "cancelled");
    }
}
