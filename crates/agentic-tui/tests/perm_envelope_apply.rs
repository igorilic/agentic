//! Step P.5.1: `apply_envelope` routes `Event::PermissionRequest` into
//! `AppState::pending_perms` and `Event::PermissionResolved` removes the
//! matching entry by `request_id`.
//!
//! These tests exercise the state-machine; rendering tests stay in
//! `perm_card.rs`.

use agentic_core::events::{
    Event, EventEnvelope, PermissionDecision, PermissionRisk as WireRisk, PermissionSource,
    CURRENT_SCHEMA_VERSION,
};
use agentic_tui::app::{AppState, PermissionRequest, PermissionRisk};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build an `EventEnvelope` wrapping any `Event`. The run/step IDs and
/// timestamps are fixed values because the tests don't care about them.
fn envelope(event: Event) -> EventEnvelope {
    EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "evt-test-001".into(),
        run_id: "run-test-001".into(),
        step_id: None,
        timestamp_ms: 0,
        event,
    }
}

/// Build a `Event::PermissionRequest` envelope with the given `request_id`.
/// All other fields are fixed so tests can focus on the field under test.
fn perm_request_event(request_id: &str) -> EventEnvelope {
    envelope(Event::PermissionRequest {
        request_id: request_id.into(),
        agent: "developer".into(),
        tool: "Bash".into(),
        arg: "rm -rf node_modules".into(),
        scope: "shell.destructive".into(),
        risk: WireRisk::High,
        reason: "destructive shell".into(),
    })
}

/// Seed a `PermissionRequest` directly into `state.pending_perms` using the
/// TUI-local struct (no bus involved). Used to set up pre-conditions for
/// `PermissionResolved` tests.
fn seed_perm(state: &mut AppState, request_id: &str) {
    state.pending_perms.push(PermissionRequest {
        request_id: request_id.into(),
        agent: "developer".into(),
        command: "rm -rf node_modules".into(),
        reason: "destructive shell".into(),
        scope: "shell.destructive".into(),
        risk: PermissionRisk::High,
    });
}

// ── Test 1: PermissionRequest envelope appends to pending_perms ──────────────

/// Applying a `PermissionRequest` envelope must push a new entry to
/// `state.pending_perms` with all fields mapped correctly.
#[test]
fn apply_permission_request_envelope_appends_to_pending_perms() {
    let mut state = AppState::default();
    let env = envelope(Event::PermissionRequest {
        request_id: "r1".into(),
        agent: "developer".into(),
        tool: "Bash".into(),
        arg: "rm -rf node_modules".into(),
        scope: "shell.destructive".into(),
        risk: WireRisk::High,
        reason: "destructive shell".into(),
    });

    state.apply_envelope(&env);

    assert_eq!(
        state.pending_perms.len(),
        1,
        "expected 1 pending perm after PermissionRequest envelope, got {}",
        state.pending_perms.len()
    );

    let perm = &state.pending_perms[0];
    assert_eq!(perm.request_id, "r1", "request_id mismatch");
    assert_eq!(perm.agent, "developer", "agent mismatch");
    assert_eq!(perm.command, "rm -rf node_modules", "command (mapped from arg) mismatch");
    assert_eq!(perm.reason, "destructive shell", "reason mismatch");
    assert_eq!(perm.scope, "shell.destructive", "scope mismatch");
    assert_eq!(perm.risk, PermissionRisk::High, "risk mismatch (expected High)");
}

// ── Test 2: PermissionResolved removes the matching request ──────────────────

/// Applying a `PermissionResolved` with a known `request_id` must remove
/// exactly that entry from `state.pending_perms`.
#[test]
fn apply_permission_resolved_removes_matching_request() {
    let mut state = AppState::default();
    seed_perm(&mut state, "r1");
    assert_eq!(state.pending_perms.len(), 1);

    let env = envelope(Event::PermissionResolved {
        request_id: "r1".into(),
        decision: PermissionDecision::AllowOnce,
        source: PermissionSource::User,
    });

    state.apply_envelope(&env);

    assert!(
        state.pending_perms.is_empty(),
        "expected pending_perms to be empty after PermissionResolved for r1, got {} items",
        state.pending_perms.len()
    );
}

// ── Test 3: unmatched PermissionResolved is a no-op ──────────────────────────

/// A `PermissionResolved` for an unknown `request_id` must leave
/// `state.pending_perms` unchanged.
#[test]
fn unmatched_resolved_is_noop() {
    let mut state = AppState::default();
    seed_perm(&mut state, "r1");

    let env = envelope(Event::PermissionResolved {
        request_id: "r2".into(),
        decision: PermissionDecision::Deny,
        source: PermissionSource::User,
    });

    state.apply_envelope(&env);

    assert_eq!(
        state.pending_perms.len(),
        1,
        "expected 1 pending perm after unmatched PermissionResolved, got {}",
        state.pending_perms.len()
    );
    assert_eq!(
        state.pending_perms[0].request_id, "r1",
        "expected remaining perm to have request_id='r1'"
    );
}

// ── Test 4: multiple requests resolve independently ───────────────────────────

/// Applying two `PermissionRequest` envelopes followed by `PermissionResolved`
/// for the first must leave only the second entry in `pending_perms`.
#[test]
fn multiple_requests_resolve_independently() {
    let mut state = AppState::default();

    // Push two requests via envelopes.
    state.apply_envelope(&perm_request_event("r1"));

    let env2 = envelope(Event::PermissionRequest {
        request_id: "r2".into(),
        agent: "architect".into(),
        tool: "Write".into(),
        arg: "git push --force".into(),
        scope: "git.push.force".into(),
        risk: WireRisk::Medium,
        reason: "force push after rebase".into(),
    });
    state.apply_envelope(&env2);

    assert_eq!(state.pending_perms.len(), 2, "expected 2 perms after 2 PermissionRequest envelopes");

    // Resolve the first.
    let resolve_r1 = envelope(Event::PermissionResolved {
        request_id: "r1".into(),
        decision: PermissionDecision::AllowSession,
        source: PermissionSource::User,
    });
    state.apply_envelope(&resolve_r1);

    assert_eq!(
        state.pending_perms.len(),
        1,
        "expected 1 perm after resolving r1, got {}",
        state.pending_perms.len()
    );
    assert_eq!(
        state.pending_perms[0].request_id, "r2",
        "expected remaining perm to have request_id='r2'"
    );
    assert_eq!(
        state.pending_perms[0].risk,
        PermissionRisk::Medium,
        "expected remaining perm to have risk=Medium"
    );
}
