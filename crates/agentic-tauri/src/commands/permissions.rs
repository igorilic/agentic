use agentic_core::events::{Event, EventBus, EventEnvelope, PermissionDecision, PermissionSource};
use tauri::State;

use super::events::EventBusState;

/// Parse decision string and publish a `PermissionResolved` envelope onto the
/// bus. Extracted from the Tauri command so tests can call it without needing
/// a real `tauri::State`.
async fn permission_decide_inner(
    _bus: &EventBus,
    _request_id: String,
    _decision: String,
    _run_id: String,
    _step_id: Option<String>,
) -> Result<(), String> {
    unimplemented!("permission_decide_inner: not yet implemented")
}

/// Tauri command. Called by the web UI when the user clicks Allow /
/// Allow-for-session / Deny on a permission card.
///
/// Publishes `Event::PermissionResolved` onto the event bus and returns
/// immediately (fire-and-forget). The gate's `evaluate_async` future is
/// unblocked by the bus subscriber it already holds — no response channel
/// is needed here.
///
/// `decision` must be one of `"once"`, `"session"`, or `"deny"`. Any other
/// value returns `Err("invalid decision: ...")` and publishes nothing.
///
/// `run_id` must be a valid ULID string. Invalid values return
/// `Err("invalid run_id: ...")` and publish nothing.
#[tauri::command]
pub async fn permission_decide(
    state: State<'_, EventBusState>,
    request_id: String,
    decision: String,
    run_id: String,
    step_id: Option<String>,
) -> Result<(), String> {
    permission_decide_inner(&state.bus, request_id, decision, run_id, step_id).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_run_id() -> String {
        ulid::Ulid::new().to_string()
    }

    /// Receive one envelope from a subscriber or panic after a short timeout.
    async fn recv_one(rx: &mut tokio::sync::broadcast::Receiver<EventEnvelope>) -> EventEnvelope {
        tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv())
            .await
            .expect("timed out waiting for envelope")
            .expect("channel closed unexpectedly")
    }

    #[tokio::test]
    async fn permission_decide_publishes_resolved_envelope_for_once() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let run_id = valid_run_id();

        let result = permission_decide_inner(
            &bus,
            "req-x".into(),
            "once".into(),
            run_id.clone(),
            None,
        )
        .await;

        assert!(result.is_ok());
        let envelope = recv_one(&mut rx).await;
        assert_eq!(envelope.run_id, run_id);
        assert_eq!(envelope.step_id, None);
        match envelope.event {
            Event::PermissionResolved {
                request_id,
                decision,
                source,
            } => {
                assert_eq!(request_id, "req-x");
                assert_eq!(decision, PermissionDecision::AllowOnce);
                assert_eq!(source, PermissionSource::User);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn permission_decide_session_value() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        permission_decide_inner(
            &bus,
            "req-session".into(),
            "session".into(),
            valid_run_id(),
            None,
        )
        .await
        .unwrap();

        let envelope = recv_one(&mut rx).await;
        match envelope.event {
            Event::PermissionResolved { decision, .. } => {
                assert_eq!(decision, PermissionDecision::AllowSession);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn permission_decide_deny_value() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        permission_decide_inner(
            &bus,
            "req-deny".into(),
            "deny".into(),
            valid_run_id(),
            None,
        )
        .await
        .unwrap();

        let envelope = recv_one(&mut rx).await;
        match envelope.event {
            Event::PermissionResolved { decision, .. } => {
                assert_eq!(decision, PermissionDecision::Deny);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn permission_decide_invalid_value_returns_err() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let result = permission_decide_inner(
            &bus,
            "req-z".into(),
            "fhqwhgads".into(),
            valid_run_id(),
            None,
        )
        .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("invalid decision"),
            "expected 'invalid decision' in error, got: {err_msg}"
        );

        // Verify no envelope was published.
        assert!(
            rx.try_recv().is_err(),
            "expected no envelope published on invalid decision"
        );
    }

    #[tokio::test]
    async fn permission_decide_returns_quickly() {
        let bus = EventBus::new();
        let _rx = bus.subscribe();

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            permission_decide_inner(
                &bus,
                "req-fast".into(),
                "once".into(),
                valid_run_id(),
                None,
            ),
        )
        .await;

        assert!(
            result.is_ok(),
            "permission_decide_inner did not return within 50ms"
        );
        assert!(result.unwrap().is_ok());
    }

    #[tokio::test]
    async fn permission_decide_invalid_run_id_returns_err() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let result = permission_decide_inner(
            &bus,
            "req-bad-run".into(),
            "once".into(),
            "not-a-ulid".into(),
            None,
        )
        .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("invalid run_id"),
            "expected 'invalid run_id' in error, got: {err_msg}"
        );

        // Verify no envelope was published.
        assert!(
            rx.try_recv().is_err(),
            "expected no envelope published on invalid run_id"
        );
    }
}
