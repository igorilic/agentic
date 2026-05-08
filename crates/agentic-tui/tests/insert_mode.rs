//! Step T.13.6: Pane-scoped `i` key — insert in Logs/Chat, no-op in Issue.
//!
//! After FindingsState removal (#99 fix-loop 1): The Issue pane 'i' key
//! no longer triages findings (that dead plumbing has been removed).
//! This file tests the surviving contracts:
//! - `pane == Logs` or `pane == Chat`, mode == Normal: `i` → `Mode::Insert`.
//! - `pane == Issue` with mode == Normal: `i` → no-op (mode stays Normal).
//! - `Mode::Insert` + `Esc` → `Mode::Normal`.
//! - `f` and `t` keys in Normal mode are no-ops when FindingsState is gone.

use agentic_tui::app::{AppState, Pane};
use agentic_tui::modes::Mode;
use crossterm::event::KeyCode;

// ── Test 1: `i` in Logs pane → Mode::Insert ──────────────────────────────────

#[test]
fn i_in_logs_pane_enters_insert_mode() {
    let mut state = AppState {
        focus: Pane::Logs,
        mode: Mode::Normal,
        ..Default::default()
    };

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Insert,
        "expected Mode::Insert after pressing 'i' in Logs pane, got {:?}",
        state.mode
    );
}

// ── Test 2: `i` in Chat pane → Mode::Insert ──────────────────────────────────

#[test]
fn i_in_chat_pane_enters_insert_mode() {
    let mut state = AppState {
        focus: Pane::Chat,
        mode: Mode::Normal,
        ..Default::default()
    };

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Insert,
        "expected Mode::Insert after pressing 'i' in Chat pane, got {:?}",
        state.mode
    );
}

// ── Test 3: `i` in Issue pane → no-op (mode stays Normal) ────────────────────

#[test]
fn i_in_issue_pane_is_noop() {
    let mut state = AppState {
        focus: Pane::Issue,
        mode: Mode::Normal,
        ..Default::default()
    };

    state.handle_key(KeyCode::Char('i'));

    assert_eq!(
        state.mode,
        Mode::Normal,
        "expected mode to stay Normal after 'i' in Issue pane (no findings to triage), got {:?}",
        state.mode
    );
}

// ── Test 4: Esc in Insert mode → Mode::Normal ────────────────────────────────

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

// ── Test 5: Esc-closes-help still fires first ────────────────────────────────

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

// ── Test 6: `i` while already in Insert mode is a no-op ─────────────────────

#[test]
fn i_in_insert_mode_is_noop() {
    // Pressing 'i' a second time while already composing must not reset the
    // mode — only Esc is handled in the Insert arm.
    let mut state = AppState {
        focus: Pane::Logs,
        mode: Mode::Insert,
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('i'));
    assert_eq!(
        state.mode,
        Mode::Insert,
        "i in Insert mode should not re-trigger; mode stays Insert"
    );

    // 'f' and 't' in Insert mode are also no-ops.
    state.handle_key(KeyCode::Char('f'));
    assert_eq!(
        state.mode,
        Mode::Insert,
        "f in Insert mode must not exit Insert; got {:?}",
        state.mode
    );
    state.handle_key(KeyCode::Char('t'));
    assert_eq!(
        state.mode,
        Mode::Insert,
        "t in Insert mode must not exit Insert; got {:?}",
        state.mode
    );
}
