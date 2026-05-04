//! Step T.13.2: Wire `y` / `s` / `n` keys to resolve a pending permission.
//!
//! When `state.pending_perms` is non-empty AND in Normal mode:
//! - `'y'` → pop the first pending perm, set `flash` to `Flash { text: "✓ once: <prefix> \"<command>\"" }`.
//! - `'s'` → pop, set `flash` to `Flash { text: "✓ session: <prefix> \"<command>\"" }`.
//! - `'n'` → pop, set `flash` to `Flash { text: "✗ denied: <prefix> \"<command>\"" }`.
//!
//! The prefix is derived from the scope field: `scope.split('.').next()` (e.g. `"shell.test"` → `"shell"`).
//!
//! When `pending_perms` is empty: the keys are no-ops (no flash, no panic).
//! In Command mode: y/s/n fall through to the command buffer.

use agentic_tui::app::{AppState, Pane, PermissionRequest, PermissionRisk};
use agentic_tui::modes::Mode;
use crossterm::event::KeyCode;

// ── Helper ────────────────────────────────────────────────────────────────────

fn perm_request(command: &str) -> PermissionRequest {
    PermissionRequest {
        request_id: "test-r1".into(),
        agent: "developer".into(),
        command: command.into(),
        reason: "test".into(),
        scope: "shell.test".into(),
        risk: PermissionRisk::High,
    }
}

// ── Test 1: 'y' pops perm and flashes "✓ once:" ──────────────────────────────

#[test]
fn key_y_pops_pending_perm_and_flashes_once() {
    let mut state = AppState {
        pending_perms: vec![perm_request("rm -rf node_modules")],
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('y'));

    assert!(
        state.pending_perms.is_empty(),
        "expected pending_perms to be empty after 'y', got {:?} items",
        state.pending_perms.len()
    );

    let flash_text = state
        .flash
        .as_ref()
        .map(|f| f.text.as_str())
        .expect("expected flash to be Some after 'y'");
    assert_eq!(
        flash_text, "✓ once: shell \"rm -rf node_modules\"",
        "expected full flash text with scope prefix and quoted command, got {:?}",
        flash_text
    );
}

// ── Test 2: 's' pops perm and flashes "✓ session:" ───────────────────────────

#[test]
fn key_s_pops_pending_perm_and_flashes_session() {
    let mut state = AppState {
        pending_perms: vec![perm_request("rm -rf node_modules")],
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('s'));

    assert!(
        state.pending_perms.is_empty(),
        "expected pending_perms to be empty after 's', got {:?} items",
        state.pending_perms.len()
    );

    let flash_text = state
        .flash
        .as_ref()
        .map(|f| f.text.as_str())
        .expect("expected flash to be Some after 's'");
    assert_eq!(
        flash_text, "✓ session: shell \"rm -rf node_modules\"",
        "expected full flash text with scope prefix and quoted command, got {:?}",
        flash_text
    );
}

// ── Test 3: 'n' pops perm and flashes "✗ denied:" ────────────────────────────

#[test]
fn key_n_pops_pending_perm_and_flashes_denied() {
    let mut state = AppState {
        pending_perms: vec![perm_request("rm -rf node_modules")],
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('n'));

    assert!(
        state.pending_perms.is_empty(),
        "expected pending_perms to be empty after 'n', got {:?} items",
        state.pending_perms.len()
    );

    let flash_text = state
        .flash
        .as_ref()
        .map(|f| f.text.as_str())
        .expect("expected flash to be Some after 'n'");
    assert_eq!(
        flash_text, "✗ denied: shell \"rm -rf node_modules\"",
        "expected full flash text with scope prefix and quoted command, got {:?}",
        flash_text
    );
}

// ── Test 4: perm keys are no-ops when no pending perms ───────────────────────

#[test]
fn perm_keys_no_op_when_no_pending_perms() {
    for key in ['y', 's', 'n'] {
        let mut state = AppState {
            pending_perms: vec![],
            ..Default::default()
        };
        state.handle_key(KeyCode::Char(key));

        assert!(
            state.pending_perms.is_empty(),
            "expected pending_perms to stay empty after '{}', got {:?} items",
            key,
            state.pending_perms.len()
        );
        assert!(
            state.flash.is_none(),
            "expected flash to stay None after '{}' with no pending perms, got {:?}",
            key,
            state.flash
        );
    }
}

// ── Test 5: perm keys pop only the first when multiple pending ────────────────

#[test]
fn perm_keys_pop_only_first_when_multiple_pending() {
    let mut state = AppState {
        pending_perms: vec![perm_request("cmd1"), perm_request("cmd2")],
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('y'));

    assert_eq!(
        state.pending_perms.len(),
        1,
        "expected 1 remaining perm after 'y', got {:?}",
        state.pending_perms.len()
    );
    assert_eq!(
        state.pending_perms[0].command, "cmd2",
        "expected remaining perm to be 'cmd2', got {:?}",
        state.pending_perms[0].command
    );

    let flash_text = state
        .flash
        .as_ref()
        .map(|f| f.text.as_str())
        .expect("expected flash to be Some");
    // The flash text should include "cmd1" (the popped command) with scope prefix "shell".
    assert_eq!(
        flash_text, "✓ once: shell \"cmd1\"",
        "expected full flash text for popped command, got {:?}",
        flash_text
    );
    assert!(
        !flash_text.contains("cmd2"),
        "expected flash NOT to contain 'cmd2' (the remaining command), got {:?}",
        flash_text
    );
}

// ── Test 6: perm keys ignored in Command mode ────────────────────────────────

#[test]
fn perm_keys_ignored_in_command_mode() {
    let mut state = AppState {
        mode: Mode::Command {
            buffer: String::new(),
        },
        pending_perms: vec![perm_request("rm")],
        ..Default::default()
    };
    state.handle_key(KeyCode::Char('y'));

    // Perm should NOT have been popped — Command mode doesn't resolve perms.
    assert_eq!(
        state.pending_perms.len(),
        1,
        "expected pending_perms to stay at 1 in Command mode after 'y', got {:?}",
        state.pending_perms.len()
    );

    // Flash must not be set.
    assert!(
        state.flash.is_none(),
        "expected flash to stay None in Command mode, got {:?}",
        state.flash
    );

    // Bonus: command buffer must have received 'y'.
    assert!(
        matches!(&state.mode, Mode::Command { buffer } if buffer == "y"),
        "expected command buffer to contain 'y' after key in Command mode, got {:?}",
        state.mode
    );
}

// ── Test 7: perm keys don't break existing pane switch ───────────────────────

#[test]
fn perm_keys_dont_break_existing_pane_switch() {
    let mut state = AppState {
        focus: Pane::Logs,
        pending_perms: vec![],
        ..Default::default()
    };

    // '1' still switches to Logs (no-op, already there — just must not panic).
    state.handle_key(KeyCode::Char('1'));
    assert_eq!(
        state.focus,
        Pane::Logs,
        "expected focus to stay Pane::Logs after '1', got {:?}",
        state.focus
    );

    // 'y' with no pending perm is a no-op.
    state.handle_key(KeyCode::Char('y'));
    assert_eq!(
        state.focus,
        Pane::Logs,
        "expected focus to stay Pane::Logs after 'y' (no pending perm), got {:?}",
        state.focus
    );
    assert!(
        state.flash.is_none(),
        "expected no flash after 'y' with no pending perm"
    );
}
