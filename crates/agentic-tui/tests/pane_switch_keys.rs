//! Step T.11.5: Direct-jump keys `1`/`2`/`3` to switch panes.
//!
//! Spec §4.8 status-line hint: "1/2/3 to switch panes".
//! '1' → Pane::Logs, '2' → Pane::Chat, '3' → Pane::Issue.
//! Only active in Normal mode; Command mode must NOT redirect focus.

use agentic_tui::app::{AppEvent, AppState, Pane};
use agentic_tui::modes::Mode;
use crossterm::event::KeyCode;

// ── Test 1: '1' jumps to Logs ─────────────────────────────────────────────

#[test]
fn key_1_switches_to_logs() {
    let mut state = AppState {
        focus: Pane::Issue,
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('1'));
    assert_eq!(
        state.focus,
        Pane::Logs,
        "expected focus = Pane::Logs after pressing '1', got {:?}",
        state.focus
    );
}

// ── Test 2: '2' jumps to Chat ─────────────────────────────────────────────

#[test]
fn key_2_switches_to_chat() {
    let mut state = AppState {
        focus: Pane::Logs,
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('2'));
    assert_eq!(
        state.focus,
        Pane::Chat,
        "expected focus = Pane::Chat after pressing '2', got {:?}",
        state.focus
    );
}

// ── Test 3: '3' jumps to Issue ────────────────────────────────────────────

#[test]
fn key_3_switches_to_issue() {
    let mut state = AppState {
        focus: Pane::Logs,
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('3'));
    assert_eq!(
        state.focus,
        Pane::Issue,
        "expected focus = Pane::Issue after pressing '3', got {:?}",
        state.focus
    );
}

// ── Test 4: '1' is idempotent when already on Logs ────────────────────────

#[test]
fn pane_switch_keys_idempotent() {
    let mut state = AppState {
        focus: Pane::Logs,
        ..Default::default()
    };
    // Must not panic and focus must remain Logs.
    state.handle_key(KeyCode::Char('1'));
    assert_eq!(
        state.focus,
        Pane::Logs,
        "expected focus to stay Pane::Logs on idempotent '1' press, got {:?}",
        state.focus
    );
}

// ── Test 5: digits ignored in Command mode ────────────────────────────────

#[test]
fn pane_switch_keys_ignored_in_command_mode() {
    let mut state = AppState {
        focus: Pane::Logs,
        mode: Mode::Command {
            buffer: String::new(),
        },
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('2'));
    // Focus must stay on Logs — digit was consumed by the command buffer.
    assert_eq!(
        state.focus,
        Pane::Logs,
        "expected focus to stay Pane::Logs in Command mode, got {:?}",
        state.focus
    );
}

// ── Test 6: direct-jump keys interoperate with Tab cycling ────────────────

/// Tab cycles normally; '3' and '1' then jump directly.
#[test]
fn pane_switch_keys_dont_break_existing_tab_cycle() {
    let mut state = AppState {
        focus: Pane::Logs,
        ..Default::default()
    };

    // Tab: Logs → Chat
    state.handle_key(KeyCode::Tab);
    assert_eq!(state.focus, Pane::Chat, "Tab: expected Chat after Logs");

    // '3': Chat → Issue (direct jump)
    state.handle_key(KeyCode::Char('3'));
    assert_eq!(state.focus, Pane::Issue, "'3': expected Issue");

    // '1': Issue → Logs (direct jump)
    state.handle_key(KeyCode::Char('1'));
    assert_eq!(state.focus, Pane::Logs, "'1': expected Logs");

    // Tab from Logs still cycles to Chat (T.11.4 intact)
    state.handle(AppEvent::ToggleFocus);
    assert_eq!(state.focus, Pane::Chat, "ToggleFocus from Logs: expected Chat");
}
