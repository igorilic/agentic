//! Issue #103: y/s/n perm-card keys must publish `Event::PermissionResolved`
//! to the bus so the orchestrator's AsyncGate receives the decision.
//!
//! Also covers the `PermissionRisk` dedupe: after the dedupe the TUI's local
//! `app::PermissionRisk` should be the same type as the wire
//! `agentic_core::events::PermissionRisk`.

use agentic_core::events::{
    CURRENT_SCHEMA_VERSION, Event, EventBus, EventEnvelope, PermissionDecision, PermissionRisk,
    PermissionSource,
};
use agentic_tui::app::{AppState, PermissionRequest};
use crossterm::event::KeyCode;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_bus() -> Arc<EventBus> {
    Arc::new(EventBus::with_capacity(64))
}

/// Drain all envelopes from a receiver into a Vec (non-blocking).
fn drain(rx: &mut Receiver<EventEnvelope>) -> Vec<EventEnvelope> {
    let mut out = Vec::new();
    while let Ok(env) = rx.try_recv() {
        out.push(env);
    }
    out
}

/// Build a `PermissionRequest` with a specific `request_id`.
fn make_perm(request_id: &str) -> PermissionRequest {
    PermissionRequest {
        request_id: request_id.into(),
        agent: "developer".into(),
        command: "rm -rf node_modules".into(),
        reason: "clean deps".into(),
        scope: "shell.destructive".into(),
        risk: PermissionRisk::High,
    }
}

fn state_with_bus_and_perm(bus: Arc<EventBus>, request_id: &str) -> AppState {
    AppState {
        bus: Some(bus),
        pending_perms: vec![make_perm(request_id)],
        ..Default::default()
    }
}

// ── Test 1: y publishes PermissionResolved with AllowOnce ────────────────────

#[test]
fn y_publishes_permission_resolved_with_allow_once() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = state_with_bus_and_perm(Arc::clone(&bus), "req-1");

    state.handle_key(KeyCode::Char('y'));

    let published = drain(&mut rx);
    assert_eq!(
        published.len(),
        1,
        "expected exactly 1 envelope published after 'y', got {}",
        published.len()
    );

    match &published[0].event {
        Event::PermissionResolved {
            request_id,
            decision,
            source,
        } => {
            assert_eq!(request_id, "req-1", "request_id mismatch");
            assert_eq!(
                *decision,
                PermissionDecision::AllowOnce,
                "decision mismatch"
            );
            assert_eq!(*source, PermissionSource::User, "source should be User");
        }
        other => panic!("expected PermissionResolved, got {:?}", other),
    }

    assert!(
        state.pending_perms.is_empty(),
        "pending_perms should be empty after 'y'"
    );
}

// ── Test 2: s publishes PermissionResolved with AllowSession ─────────────────

#[test]
fn s_publishes_permission_resolved_with_allow_session() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = state_with_bus_and_perm(Arc::clone(&bus), "req-2");

    state.handle_key(KeyCode::Char('s'));

    let published = drain(&mut rx);
    assert_eq!(
        published.len(),
        1,
        "expected exactly 1 envelope published after 's', got {}",
        published.len()
    );

    match &published[0].event {
        Event::PermissionResolved {
            request_id,
            decision,
            source,
        } => {
            assert_eq!(request_id, "req-2", "request_id mismatch");
            assert_eq!(
                *decision,
                PermissionDecision::AllowSession,
                "decision mismatch"
            );
            assert_eq!(*source, PermissionSource::User, "source should be User");
        }
        other => panic!("expected PermissionResolved, got {:?}", other),
    }

    assert!(
        state.pending_perms.is_empty(),
        "pending_perms should be empty after 's'"
    );
}

// ── Test 3: n publishes PermissionResolved with Deny ─────────────────────────

#[test]
fn n_publishes_permission_resolved_with_deny() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = state_with_bus_and_perm(Arc::clone(&bus), "req-3");

    state.handle_key(KeyCode::Char('n'));

    let published = drain(&mut rx);
    assert_eq!(
        published.len(),
        1,
        "expected exactly 1 envelope published after 'n', got {}",
        published.len()
    );

    match &published[0].event {
        Event::PermissionResolved {
            request_id,
            decision,
            source,
        } => {
            assert_eq!(request_id, "req-3", "request_id mismatch");
            assert_eq!(*decision, PermissionDecision::Deny, "decision mismatch");
            assert_eq!(*source, PermissionSource::User, "source should be User");
        }
        other => panic!("expected PermissionResolved, got {:?}", other),
    }

    assert!(
        state.pending_perms.is_empty(),
        "pending_perms should be empty after 'n'"
    );
}

// ── Test 4: y with no pending request publishes nothing ──────────────────────

#[test]
fn y_with_no_pending_request_publishes_nothing() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = AppState {
        bus: Some(bus),
        ..Default::default()
    };
    // pending_perms is empty

    state.handle_key(KeyCode::Char('y'));

    let published = drain(&mut rx);
    assert!(
        published.is_empty(),
        "expected nothing published after 'y' with no pending perms, got {:?}",
        published.len()
    );
}

// ── Test 5: y publishes for first pending when multiple present ───────────────

#[test]
fn y_publishes_for_first_pending_when_multiple_present() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = AppState {
        bus: Some(Arc::clone(&bus)),
        pending_perms: vec![make_perm("req-first"), make_perm("req-second")],
        ..Default::default()
    };

    state.handle_key(KeyCode::Char('y'));

    let published = drain(&mut rx);
    assert_eq!(
        published.len(),
        1,
        "expected exactly 1 envelope published, got {}",
        published.len()
    );

    match &published[0].event {
        Event::PermissionResolved { request_id, .. } => {
            assert_eq!(
                request_id, "req-first",
                "should publish for the first pending, got {:?}",
                request_id
            );
        }
        other => panic!("expected PermissionResolved, got {:?}", other),
    }

    assert_eq!(state.pending_perms.len(), 1, "one perm should remain");
    assert_eq!(state.pending_perms[0].request_id, "req-second");
}

// ── Test 6: pending_perms_uses_wire_permission_risk (dedupe test) ─────────────
//
// After the dedupe: `app::PermissionRisk` IS `events::PermissionRisk`.
// Constructing a PermissionRequest with the wire type compiles without
// conversion, and the value stored in pending_perms is identical.

#[test]
fn pending_perms_uses_wire_permission_risk() {
    let mut state = AppState::default();

    // Build envelope using wire PermissionRisk directly.
    let env = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "evt-dedupe-test".into(),
        run_id: "run-001".into(),
        step_id: None,
        timestamp_ms: 0,
        event: Event::PermissionRequest {
            request_id: "r-dedupe".into(),
            agent: "developer".into(),
            tool: "Bash".into(),
            arg: "ls".into(),
            scope: "fs.read".into(),
            risk: PermissionRisk::High,
            reason: "read dir".into(),
        },
    };

    state.apply_envelope(&env);

    assert_eq!(state.pending_perms.len(), 1);
    // After dedupe: the risk field IS the wire type — no conversion
    assert_eq!(
        state.pending_perms[0].risk,
        PermissionRisk::High,
        "risk stored in pending_perms should be the wire PermissionRisk::High"
    );
}

// ── Test 7: envelope run_id is propagated to published envelope ───────────────
//
// When publishing PermissionResolved, the envelope's run_id should come from
// the AppState so the orchestrator can correlate it.

#[test]
fn published_envelope_contains_run_id() {
    let bus = make_bus();
    let mut rx = bus.subscribe();
    let mut state = state_with_bus_and_perm(Arc::clone(&bus), "req-run-id");
    state.run_label = Some("run-42".into());

    state.handle_key(KeyCode::Char('y'));

    let published = drain(&mut rx);
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].run_id, "run-42");
}
