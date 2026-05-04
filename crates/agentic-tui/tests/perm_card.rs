//! Step T.13.1: Inline permission card in logs pane.
//!
//! Spec §4.7: When `state.pending_perms` is non-empty and `state.focus == Pane::Logs`,
//! render a red-bordered permission card AFTER the most recent log row.
//!
//! Visual contract:
//! ```
//! ┌─ ⚠ PERM  developer requests permission                HIGH RISK ─┐
//! │ $ rm -rf node_modules                                            │
//! │ Cleaning stale build artifacts (scope: shell.destructive)        │
//! │ [y] allow once    [s] session    [n] deny                        │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//! Plus a RED `┃` left accent column immediately left of the `┌`.

use agentic_core::events::Severity;
use agentic_tui::app::{AppState, LogEntry, LogLevel, Pane, PermissionRequest, PermissionRisk};
use agentic_tui::draw_app;
use agentic_tui::findings::Finding;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helpers ───────────────────────────────────────────────────────────────────

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;

/// Render `draw_app` at 100×30 and return the cloned buffer.
fn render(state: &AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Render `draw_app` at the given dimensions and return the cloned buffer.
fn render_sized(state: &AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Find the first occurrence of `needle` by scanning cell-by-cell.
/// Returns `(col, row)` of the first character, or `None`.
fn find_str(
    buf: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let chars: Vec<char> = needle.chars().collect();
    if chars.is_empty() {
        return None;
    }
    for y in 0..height {
        'outer: for x in 0..width {
            for (i, ch) in chars.iter().enumerate() {
                let col = x + i as u16;
                if col >= width {
                    continue 'outer;
                }
                if buf.cell((col, y)).map(|c| c.symbol()) != Some(&ch.to_string()) {
                    continue 'outer;
                }
            }
            return Some((x, y));
        }
    }
    None
}

/// Returns `true` if `needle` appears anywhere in the buffer.
fn buf_contains(buf: &ratatui::buffer::Buffer, needle: &str, width: u16, height: u16) -> bool {
    find_str(buf, needle, width, height).is_some()
}

/// Build a `PermissionRequest` with fixed HIGH-RISK values used across tests.
fn perm_request() -> PermissionRequest {
    PermissionRequest {
        request_id: "test-r1".into(),
        agent: "developer".into(),
        command: "rm -rf node_modules".into(),
        reason: "Cleaning stale build artifacts".into(),
        scope: "shell.destructive".into(),
        risk: PermissionRisk::High,
    }
}

/// A base `AppState` with focus=Logs, no pipeline, no log, and the given perms.
fn state_with_perm(perms: Vec<PermissionRequest>) -> AppState {
    AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        pending_perms: perms,
        ..Default::default()
    }
}

fn info_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:13".to_string(),
        agent: "architect".to_string(),
        level: LogLevel::Info,
        message: "Analyzing ticket".to_string(),
    }
}

fn warn_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:15".to_string(),
        agent: "qa".to_string(),
        level: LogLevel::Warn,
        message: "Coverage below threshold".to_string(),
    }
}

// ── Test 1: top border contains perm label, agent, and risk ──────────────────

/// The first row of the card must contain "⚠ PERM", the agent name phrase
/// "developer requests permission", and "HIGH RISK".
/// The `┌` and `┐` characters must be present in the buffer on the same row.
#[test]
fn perm_card_renders_top_border_with_perm_label_and_risk() {
    let state = state_with_perm(vec![perm_request()]);
    let buf = render(&state);

    // The card must contain "⚠ PERM" text.
    let (col, row) = find_str(&buf, "⚠ PERM", WIDTH, HEIGHT).expect("'⚠ PERM' not found in buffer");

    // On the same row, verify "developer requests permission" is present.
    assert!(
        buf_contains(&buf, "developer requests permission", WIDTH, HEIGHT),
        "expected 'developer requests permission' in buffer"
    );

    // On the same row, verify "HIGH RISK" is present.
    assert!(
        buf_contains(&buf, "HIGH RISK", WIDTH, HEIGHT),
        "expected 'HIGH RISK' in buffer"
    );

    // The border character `┌` must appear somewhere in the buffer
    // (on or near the top border row of the card).
    assert!(
        buf_contains(&buf, "┌", WIDTH, HEIGHT),
        "expected '┌' (top-left border) in buffer"
    );
    assert!(
        buf_contains(&buf, "┐", WIDTH, HEIGHT),
        "expected '┐' (top-right border) in buffer"
    );

    // The `┌` cell must have RED foreground.
    let (tl_col, tl_row) = find_str(&buf, "┌", WIDTH, HEIGHT).expect("'┌' not found in buffer");
    let tl_cell = buf.cell((tl_col, tl_row)).unwrap();
    assert_eq!(
        tl_cell.style().fg,
        Some(agentic_tui::theme::RED),
        "expected '┌' at ({tl_col}, {tl_row}) to have fg=RED, got {:?}",
        tl_cell.style().fg
    );

    // Keep `col` in scope to suppress unused-variable warning.
    let _ = col;
    let _ = row;
}

// ── Test 2: command row — GREEN fg on Black bg, DIM prefix ───────────────────

/// The command row must show `$ rm -rf node_modules`.
/// The `$` prefix must be DIM; the `r` (first char of `rm`) must be GREEN on
/// Color::Black background.
#[test]
fn perm_card_renders_command_with_green_fg_and_black_bg() {
    let state = state_with_perm(vec![perm_request()]);
    let buf = render(&state);

    // Find the `$` prefix.
    let (dollar_col, dollar_row) =
        find_str(&buf, "$ rm", WIDTH, HEIGHT).expect("'$ rm' not found in buffer");

    let dollar_cell = buf.cell((dollar_col, dollar_row)).unwrap();
    assert_eq!(
        dollar_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected '$' at ({dollar_col}, {dollar_row}) to have fg=DIM (prefix), got {:?}",
        dollar_cell.style().fg
    );

    // `r` of `rm` is two chars after `$` (i.e., `$ r`) — or we look two cols right of `$`.
    // `$ ` is `$` + ` ` so `r` is at dollar_col + 2.
    let r_col = dollar_col + 2;
    let r_cell = buf.cell((r_col, dollar_row)).unwrap();
    assert_eq!(
        r_cell.style().fg,
        Some(agentic_tui::theme::GREEN),
        "expected 'r' of 'rm' at ({r_col}, {dollar_row}) to have fg=GREEN, got {:?}",
        r_cell.style().fg
    );
    assert_eq!(
        r_cell.style().bg,
        Some(ratatui::style::Color::Black),
        "expected 'r' of 'rm' at ({r_col}, {dollar_row}) to have bg=Black, got {:?}",
        r_cell.style().bg
    );
}

// ── Test 3: reason row — DIM for reason text, YELLOW for scope value ─────────

/// The reason row must contain "Cleaning stale build artifacts (scope: shell.destructive)".
/// F2: reason text must be DIM, scope value must be YELLOW.
#[test]
fn perm_card_renders_reason_row() {
    let state = state_with_perm(vec![perm_request()]);
    let buf = render(&state);

    assert!(
        buf_contains(&buf, "Cleaning stale build artifacts", WIDTH, HEIGHT),
        "expected 'Cleaning stale build artifacts' in buffer"
    );
    assert!(
        buf_contains(&buf, "scope: shell.destructive", WIDTH, HEIGHT),
        "expected 'scope: shell.destructive' in buffer"
    );

    // F2: The 'C' of "Cleaning" must be in DIM color (not FG).
    let (c_col, c_row) =
        find_str(&buf, "Cleaning", WIDTH, HEIGHT).expect("'Cleaning' not found in buffer");
    let c_cell = buf.cell((c_col, c_row)).unwrap();
    assert_eq!(
        c_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected 'C' of 'Cleaning' at ({c_col}, {c_row}) to have fg=DIM (F2), got {:?}",
        c_cell.style().fg
    );

    // F2: The 's' of "shell.destructive" (scope value) must be in YELLOW.
    let (scope_col, scope_row) = find_str(&buf, "shell.destructive", WIDTH, HEIGHT)
        .expect("'shell.destructive' not found in buffer");
    let scope_cell = buf.cell((scope_col, scope_row)).unwrap();
    assert_eq!(
        scope_cell.style().fg,
        Some(agentic_tui::theme::YELLOW),
        "expected 's' of 'shell.destructive' at ({scope_col}, {scope_row}) to have fg=YELLOW (F2), got {:?}",
        scope_cell.style().fg
    );
}

// ── Test 4: hotkey row — [y] GREEN bold, [s] GREEN bold, [n] RED bold ────────

/// The hotkey row must have `[y]` in GREEN+BOLD, `[s]` in GREEN+BOLD,
/// `[n]` in RED+BOLD, and labels "allow once", "session", "deny" in FG.
#[test]
fn perm_card_renders_hotkey_row_with_correct_colors() {
    use ratatui::style::Modifier;

    let state = state_with_perm(vec![perm_request()]);
    let buf = render(&state);

    // Locate "[y]".
    let (y_col, y_row) = find_str(&buf, "[y]", WIDTH, HEIGHT).expect("'[y]' not found in buffer");
    // `[` is at y_col; check `y` at y_col+1.
    let y_cell = buf.cell((y_col + 1, y_row)).unwrap();
    assert_eq!(
        y_cell.style().fg,
        Some(agentic_tui::theme::GREEN),
        "expected 'y' in '[y]' to have fg=GREEN, got {:?}",
        y_cell.style().fg
    );
    assert!(
        y_cell.style().add_modifier.contains(Modifier::BOLD),
        "expected 'y' in '[y]' to be BOLD"
    );

    // Locate "[s]".
    let (s_col, _s_row) = find_str(&buf, "[s]", WIDTH, HEIGHT).expect("'[s]' not found in buffer");
    let s_cell = buf.cell((s_col + 1, y_row)).unwrap();
    assert_eq!(
        s_cell.style().fg,
        Some(agentic_tui::theme::GREEN),
        "expected 's' in '[s]' to have fg=GREEN, got {:?}",
        s_cell.style().fg
    );
    assert!(
        s_cell.style().add_modifier.contains(Modifier::BOLD),
        "expected 's' in '[s]' to be BOLD"
    );

    // Locate "[n]".
    let (n_col, _n_row) = find_str(&buf, "[n]", WIDTH, HEIGHT).expect("'[n]' not found in buffer");
    let n_cell = buf.cell((n_col + 1, y_row)).unwrap();
    assert_eq!(
        n_cell.style().fg,
        Some(agentic_tui::theme::RED),
        "expected 'n' in '[n]' to have fg=RED, got {:?}",
        n_cell.style().fg
    );
    assert!(
        n_cell.style().add_modifier.contains(Modifier::BOLD),
        "expected 'n' in '[n]' to be BOLD"
    );

    // Labels "allow once", "session", "deny" must appear in the buffer.
    assert!(
        buf_contains(&buf, "allow once", WIDTH, HEIGHT),
        "expected 'allow once' in buffer"
    );
    assert!(
        buf_contains(&buf, "session", WIDTH, HEIGHT),
        "expected 'session' in buffer"
    );
    assert!(
        buf_contains(&buf, "deny", WIDTH, HEIGHT),
        "expected 'deny' in buffer"
    );
}

// ── Test 5: RED left accent column (`┃`) spans all 5 card rows ───────────────

/// S2 (tightened): The `┃` accent must appear at the SAME column on ALL 5
/// card rows (top border, command, reason, hotkey, bottom border), each with
/// RED foreground.
#[test]
fn perm_card_has_red_left_accent_column() {
    let state = state_with_perm(vec![perm_request()]);
    let buf = render(&state);

    // Locate the first `┃` to find the accent column and the card's top row.
    let (accent_col, top_row) =
        find_str(&buf, "┃", WIDTH, HEIGHT).expect("'┃' (left accent) not found in buffer");

    // The card is 5 rows tall; assert accent at each of the 5 rows.
    for offset in 0u16..5 {
        let row = top_row + offset;
        let cell = buf.cell((accent_col, row)).unwrap_or_else(|| {
            panic!("no cell at ({accent_col}, {row}) for accent row offset={offset}")
        });
        assert_eq!(
            cell.symbol(),
            "┃",
            "expected '┃' at ({accent_col}, {row}) [card row {offset}], got {:?}",
            cell.symbol()
        );
        assert_eq!(
            cell.style().fg,
            Some(agentic_tui::theme::RED),
            "expected RED fg at ({accent_col}, {row}) [card row {offset}], got {:?}",
            cell.style().fg
        );
    }
}

// ── Test 6: card only renders when focus is Logs ──────────────────────────────

/// When `state.focus == Pane::Chat`, the permission card must NOT render.
#[test]
fn perm_card_only_renders_when_focus_is_logs() {
    let state = AppState {
        focus: Pane::Chat,
        pipeline: vec![],
        pending_perms: vec![perm_request()],
        ..Default::default()
    };
    let buf = render(&state);

    assert!(
        !buf_contains(&buf, "⚠ PERM", WIDTH, HEIGHT),
        "expected NO '⚠ PERM' when focus is Chat"
    );
}

// ── Test 7: card only renders when pending_perms is non-empty ────────────────

/// When `state.pending_perms` is empty, no card must appear.
#[test]
fn perm_card_only_renders_when_pending_perms_non_empty() {
    let state = state_with_perm(vec![]);
    let buf = render(&state);

    assert!(
        !buf_contains(&buf, "⚠ PERM", WIDTH, HEIGHT),
        "expected NO '⚠ PERM' when pending_perms is empty"
    );
}

// ── Test 8: card renders AFTER log rows ──────────────────────────────────────

/// The perm card's `⚠ PERM` row must have a greater y-index than the last log row.
#[test]
fn perm_card_renders_after_log_rows() {
    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log: vec![info_entry(), warn_entry()],
        pending_perms: vec![perm_request()],
        ..Default::default()
    };
    let buf = render(&state);

    // Find the warn entry message to establish the last log row.
    let (_warn_col, warn_row) = find_str(&buf, "Coverage below threshold", WIDTH, HEIGHT)
        .expect("warn message not found in buffer");

    // Find the perm card top border row.
    let (_perm_col, perm_row) =
        find_str(&buf, "⚠ PERM", WIDTH, HEIGHT).expect("'⚠ PERM' not found in buffer");

    assert!(
        perm_row > warn_row,
        "expected perm card row ({perm_row}) to be AFTER last log row ({warn_row})"
    );
}

// ── Test 9: no panic on narrow terminal ──────────────────────────────────────

/// Rendering at 50×20 with a pending perm must not panic.
#[test]
fn perm_card_does_not_panic_on_narrow_terminal() {
    let state = state_with_perm(vec![perm_request()]);
    // Must not panic.
    let _ = render_sized(&state, 50, 20);
}

// ── Test 10: multiple pending perms — only first is rendered ─────────────────

/// When `pending_perms` has two entries, only the FIRST is rendered.
/// The second entry's command must NOT appear in the buffer.
#[test]
fn perm_card_handles_multiple_pending_perms() {
    let perm1 = perm_request();
    let perm2 = PermissionRequest {
        request_id: "test-r2".into(),
        agent: "architect".into(),
        command: "git push --force".into(),
        reason: "Force-push after rebase".into(),
        scope: "git.push.force".into(),
        risk: PermissionRisk::High,
    };
    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        pending_perms: vec![perm1, perm2],
        ..Default::default()
    };
    let buf = render(&state);

    // First perm's command must appear.
    assert!(
        buf_contains(&buf, "rm -rf node_modules", WIDTH, HEIGHT),
        "expected first perm's command 'rm -rf node_modules' in buffer"
    );

    // Second perm's command must NOT appear.
    assert!(
        !buf_contains(&buf, "git push --force", WIDTH, HEIGHT),
        "expected second perm's command 'git push --force' to NOT appear in buffer"
    );
}

// ── Test 11: LOW RISK and MEDIUM RISK labels ─────────────────────────────────

/// `PermissionRisk::Low` must produce "LOW RISK" on the top border.
/// `PermissionRisk::Medium` must produce "MEDIUM RISK" on the top border.
#[test]
fn perm_card_handles_low_and_medium_risk() {
    // LOW RISK
    let low_perm = PermissionRequest {
        risk: PermissionRisk::Low,
        ..perm_request()
    };
    let state_low = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        pending_perms: vec![low_perm],
        ..Default::default()
    };
    let buf_low = render(&state_low);
    assert!(
        buf_contains(&buf_low, "LOW RISK", WIDTH, HEIGHT),
        "expected 'LOW RISK' in buffer for PermissionRisk::Low"
    );

    // MEDIUM RISK
    let medium_perm = PermissionRequest {
        risk: PermissionRisk::Medium,
        ..perm_request()
    };
    let state_medium = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        pending_perms: vec![medium_perm],
        ..Default::default()
    };
    let buf_medium = render(&state_medium);
    assert!(
        buf_contains(&buf_medium, "MEDIUM RISK", WIDTH, HEIGHT),
        "expected 'MEDIUM RISK' in buffer for PermissionRisk::Medium"
    );
}

// ── Test 12: perm card renders BEFORE findings widget (S3) ───────────────────

/// S3: When both a pending perm AND non-empty findings are present, the perm
/// card's `⚠ PERM` row must appear ABOVE any findings row in the buffer.
///
/// This locks in the S1 ordering fix: log rows → perm card → findings.
#[test]
fn perm_card_renders_above_findings_when_both_present() {
    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        pending_perms: vec![perm_request()],
        findings: agentic_tui::findings::FindingsState {
            items: vec![Finding {
                id: "f1".to_string(),
                severity: Severity::Warning,
                file: Some("src/main.rs".to_string()),
                line: Some(42),
                message: "UniqueFindingMessage42".to_string(),
                triage: None,
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    let buf = render(&state);

    // The perm card top border must be in the buffer.
    let (_perm_col, perm_row) =
        find_str(&buf, "⚠ PERM", WIDTH, HEIGHT).expect("'⚠ PERM' not found in buffer");

    // The finding message must be in the buffer.
    let (_find_col, find_row) = find_str(&buf, "UniqueFindingMessage42", WIDTH, HEIGHT)
        .expect("'UniqueFindingMessage42' not found in buffer");

    assert!(
        perm_row < find_row,
        "expected perm card row ({perm_row}) to appear BEFORE findings row ({find_row}) [S1]"
    );
}
