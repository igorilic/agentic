//! Step T.13.6: Pane-scoped `i` key — insert in Logs/Chat, triage in Issue.
//!
//! Contract:
//! - `pane == Logs` or `pane == Chat`, mode == Normal: `i` → `Mode::Insert`.
//! - `pane == Issue` with a finding present at cursor: `i` → `Triage::Ignore`.
//! - `pane == Issue` with no findings (cursor hits nothing): `i` → no-op.
//! - `Mode::Insert` + `Esc` → `Mode::Normal`.
//! - `f` and `t` triage unconditionally (NOT scoped — regression guard).

use agentic_core::events::Severity;
use agentic_tui::app::{AppState, Pane};
use agentic_tui::findings::{Finding, FindingsState, Triage};
use agentic_tui::modes::Mode;
use crossterm::event::KeyCode;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Build a minimal `FindingsState` seeded with one finding at cursor 0.
fn state_with_finding() -> FindingsState {
    let finding = Finding {
        id: "f-001".into(),
        severity: Severity::Error,
        file: None,
        line: None,
        message: "test finding".into(),
        triage: None,
    };
    FindingsState {
        items: vec![finding],
        cursor: 0,
    }
}

// ── Test 1: `i` in Logs pane → Mode::Insert, findings unchanged ──────────────

#[test]
fn i_in_logs_pane_enters_insert_mode() {
    let mut state = AppState {
        focus: Pane::Logs,
        mode: Mode::Normal,
        findings: state_with_finding(),
        ..Default::default()
    };
    let findings_count_before = state.findings.items.len();
    let triage_before = state.findings.items[0].triage;

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Insert,
        "expected Mode::Insert after pressing 'i' in Logs pane, got {:?}",
        state.mode
    );
    assert_eq!(
        state.findings.items.len(),
        findings_count_before,
        "findings count must not change when 'i' entered insert mode"
    );
    assert_eq!(
        state.findings.items[0].triage, triage_before,
        "findings triage must not change when 'i' entered insert mode in Logs, got {:?}",
        state.findings.items[0].triage
    );
}

// ── Test 2: `i` in Chat pane → Mode::Insert ──────────────────────────────────

#[test]
fn i_in_chat_pane_enters_insert_mode() {
    let mut state = AppState {
        focus: Pane::Chat,
        mode: Mode::Normal,
        findings: state_with_finding(),
        ..Default::default()
    };
    let triage_before = state.findings.items[0].triage;

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Insert,
        "expected Mode::Insert after pressing 'i' in Chat pane, got {:?}",
        state.mode
    );
    assert_eq!(
        state.findings.items[0].triage, triage_before,
        "findings triage must not change when 'i' entered insert mode in Chat, got {:?}",
        state.findings.items[0].triage
    );
}

// ── Test 3: `i` in Issue pane with finding → Triage::Ignore, mode stays Normal

#[test]
fn i_in_issue_pane_with_selection_triages_ignore() {
    let mut state = AppState {
        focus: Pane::Issue,
        mode: Mode::Normal,
        findings: state_with_finding(),
        ..Default::default()
    };
    assert!(
        !state.findings.items.is_empty(),
        "precondition: findings must be non-empty"
    );

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.findings.items[0].triage,
        Some(Triage::Ignore),
        "expected finding triage to be Ignore after 'i' in Issue pane with selection, got {:?}",
        state.findings.items[0].triage
    );
    assert_eq!(
        state.mode,
        Mode::Normal,
        "expected mode to stay Normal after triage in Issue pane, got {:?}",
        state.mode
    );
}

// ── Test 4: `i` in Issue pane with no findings → no-op ───────────────────────

#[test]
fn i_in_issue_pane_without_selection_is_noop() {
    let mut state = AppState {
        focus: Pane::Issue,
        mode: Mode::Normal,
        findings: FindingsState::default(), // empty — cursor=0 hits nothing
        ..Default::default()
    };

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Normal,
        "expected mode to stay Normal after 'i' in Issue pane with no findings, got {:?}",
        state.mode
    );
    assert!(
        state.findings.items.is_empty(),
        "expected findings to remain empty, got {:?} items",
        state.findings.items.len()
    );
}

// ── Test 5: Esc in Insert mode → Mode::Normal ────────────────────────────────

#[test]
fn esc_in_insert_mode_returns_to_normal() {
    let mut state = AppState {
        mode: Mode::Insert,
        ..Default::default()
    };

    state.handle_key(KeyCode::Esc);

    assert_eq!(
        state.mode,
        Mode::Normal,
        "expected Mode::Normal after Esc in Insert mode, got {:?}",
        state.mode
    );
}

// ── Test 6: Esc-closes-help still fires first (T.13.5 guard) ─────────────────

#[test]
fn esc_closes_help_before_insert_mode_exit() {
    let mut state = AppState {
        mode: Mode::Insert,
        help_open: true,
        ..Default::default()
    };

    state.handle_key(KeyCode::Esc);

    assert!(
        !state.help_open,
        "expected help_open to be false after Esc, got true"
    );
    // Mode should NOT have changed — help-close Esc fires first and returns.
    assert_eq!(
        state.mode,
        Mode::Insert,
        "expected mode to stay Insert (Esc was consumed by help close), got {:?}",
        state.mode
    );
}

// ── Test 7: Regression — `f` and `t` triage unconditionally in non-Issue pane

#[test]
fn f_and_t_keys_still_triage_unconditionally_in_logs() {
    // 'f' in Logs pane — should still triage Fix (NOT scoped like 'i').
    let mut state = AppState {
        focus: Pane::Logs,
        mode: Mode::Normal,
        findings: state_with_finding(),
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('f'));
    assert_eq!(
        state.findings.items[0].triage,
        Some(Triage::Fix),
        "expected Triage::Fix after 'f' in Logs pane (unconditional), got {:?}",
        state.findings.items[0].triage
    );

    // 't' in Logs pane — should still triage TechDebt.
    let mut state2 = AppState {
        focus: Pane::Logs,
        mode: Mode::Normal,
        findings: state_with_finding(),
        ..Default::default()
    };
    state2.handle_key(KeyCode::Char('t'));
    assert_eq!(
        state2.findings.items[0].triage,
        Some(Triage::TechDebt),
        "expected Triage::TechDebt after 't' in Logs pane (unconditional), got {:?}",
        state2.findings.items[0].triage
    );
}
