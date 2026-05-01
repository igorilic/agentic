//! Step T.11.4: Tab bar widget — spec §4.5.
//!
//! Two-row strip rendered between the pipeline bar and body pane:
//!   Row 0: `① logs   ② chat   ③ issue` + `? for help` right-aligned in DIM
//!   Row 1: `─` underline in ACCENT under the active tab only
//!
//! Colour contracts:
//!   - Active tab: fg=ACCENT, bg=HEADER_BG, BOLD modifier
//!   - Inactive tabs: fg=DIM, bg=HEADER_BG
//!   - Help hint: fg=DIM, bg=HEADER_BG
//!   - Underline cells: symbol="─", fg=ACCENT, bg=HEADER_BG
//!   - Non-underline cells on row 1: bg=HEADER_BG

use agentic_tui::app::{AppState, Pane};
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Collect every symbol in a given row into a single string.
fn row_string(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    (0..width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Collect all rows into a flat string (for substring searches).
fn buffer_string(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Find the first occurrence of `needle` by scanning cell-by-cell.
/// Returns (col, row) of the first character of the match, or None.
fn find_in_buffer(
    buffer: &ratatui::buffer::Buffer,
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
                if buffer.cell((col, y)).unwrap().symbol() != ch.to_string() {
                    continue 'outer;
                }
            }
            return Some((x, y));
        }
    }
    None
}

/// Build a default state for Logs pane with empty pipeline so the tab bar
/// lands immediately after the issue-header row (row 0=title, row 1=issue
/// header, rows 2–3 = tab bar, row 4+ = body).
fn logs_state() -> AppState {
    AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        ..Default::default()
    }
}

// ── Test 1: three tab labels + help hint ──────────────────────────────────

/// Tab bar row must contain "① logs", "② chat", "③ issue", and "? for help".
#[test]
fn tab_bar_renders_three_tabs_with_help_hint() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 140, 40);

    assert!(
        full.contains("① logs"),
        "expected '① logs' in buffer; got:\n{full}"
    );
    assert!(
        full.contains("② chat"),
        "expected '② chat' in buffer; got:\n{full}"
    );
    assert!(
        full.contains("③ issue"),
        "expected '③ issue' in buffer; got:\n{full}"
    );
    assert!(
        full.contains("? for help"),
        "expected '? for help' in buffer; got:\n{full}"
    );
}

// ── Test 2: active tab is ACCENT + BOLD ───────────────────────────────────

/// When pane = Logs, the 'l' in "logs" must have fg=ACCENT and BOLD.
#[test]
fn tab_bar_active_tab_is_accent_bold() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find "① logs" — the ① is a single char but multi-byte; find_in_buffer
    // scans by char so it handles it correctly.
    let (col, row) =
        find_in_buffer(&buffer, "① logs", 140, 40).expect("'① logs' not found in buffer");

    // 'l' is the second char of "logs" which starts after "① " (2 chars).
    // "① logs" = ['①', ' ', 'l', 'o', 'g', 's'] → 'l' is at offset 2.
    let l_col = col + 2;
    let cell = buffer.cell((l_col, row)).unwrap();

    let accent = agentic_tui::theme::ACCENT;
    assert_eq!(
        cell.style().fg,
        Some(accent),
        "expected 'l' in active '① logs' at ({l_col}, {row}) to have fg=ACCENT, got {:?}",
        cell.style().fg
    );
    assert!(
        cell.style().add_modifier.contains(Modifier::BOLD),
        "expected 'l' in active '① logs' at ({l_col}, {row}) to have BOLD modifier, got modifier={:?}",
        cell.style().add_modifier
    );
}

// ── Test 3: inactive tabs are DIM ─────────────────────────────────────────

/// When pane = Logs, "chat" and "issue" tabs must use fg=DIM.
#[test]
fn tab_bar_inactive_tabs_are_dim() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let dim = agentic_tui::theme::DIM;

    // Find "② chat" — 'c' is the 3rd char (offset 2 from '②').
    let (chat_col, chat_row) =
        find_in_buffer(&buffer, "② chat", 140, 40).expect("'② chat' not found in buffer");
    let c_col = chat_col + 2;
    let c_cell = buffer.cell((c_col, chat_row)).unwrap();
    assert_eq!(
        c_cell.style().fg,
        Some(dim),
        "expected 'c' in '② chat' at ({c_col}, {chat_row}) to have fg=DIM, got {:?}",
        c_cell.style().fg
    );

    // Find "③ issue" — 'i' is at offset 2.
    let (issue_col, issue_row) =
        find_in_buffer(&buffer, "③ issue", 140, 40).expect("'③ issue' not found in buffer");
    let i_col = issue_col + 2;
    let i_cell = buffer.cell((i_col, issue_row)).unwrap();
    assert_eq!(
        i_cell.style().fg,
        Some(dim),
        "expected 'i' in '③ issue' at ({i_col}, {issue_row}) to have fg=DIM, got {:?}",
        i_cell.style().fg
    );
}

// ── Test 4: help hint is DIM ──────────────────────────────────────────────

/// The '?' in "? for help" must have fg=DIM.
#[test]
fn tab_bar_help_hint_is_dim() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let dim = agentic_tui::theme::DIM;

    let (col, row) =
        find_in_buffer(&buffer, "? for help", 140, 40).expect("'? for help' not found in buffer");

    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(dim),
        "expected '?' in '? for help' at ({col}, {row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );
}

// ── Test 5: underline appears under active tab ────────────────────────────

/// Row 1 of the tab bar (1 row below the label row) must have '─' cells
/// with fg=ACCENT under the active tab, but NOT under inactive tabs.
#[test]
fn tab_bar_underline_appears_under_active_tab() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let accent = agentic_tui::theme::ACCENT;

    // Find "① logs" — the label row.
    let (logs_col, label_row) =
        find_in_buffer(&buffer, "① logs", 140, 40).expect("'① logs' not found in buffer");

    let underline_row = label_row + 1;

    // "① logs" is 6 chars wide. Check that cells at that column range on the
    // underline row are '─' with fg=ACCENT.
    // Use the 'l' column (offset 2 from logs_col) to avoid the circled digit width issues.
    let check_col = logs_col + 2; // 'l' column
    let cell = buffer.cell((check_col, underline_row)).unwrap();
    assert_eq!(
        cell.symbol(),
        "─",
        "expected '─' on underline row {underline_row} at col {check_col} (under active '① logs'), got {:?}",
        cell.symbol()
    );
    assert_eq!(
        cell.style().fg,
        Some(accent),
        "expected '─' at ({check_col}, {underline_row}) to have fg=ACCENT, got {:?}",
        cell.style().fg
    );

    // Find "② chat" and assert the underline row there is NOT '─'.
    let (chat_col, _chat_label_row) =
        find_in_buffer(&buffer, "② chat", 140, 40).expect("'② chat' not found in buffer");
    let chat_check_col = chat_col + 2; // 'c' column
    let chat_underline_cell = buffer.cell((chat_check_col, underline_row)).unwrap();
    assert_ne!(
        chat_underline_cell.symbol(),
        "─",
        "expected no '─' underline at ({chat_check_col}, {underline_row}) under inactive '② chat', got {:?}",
        chat_underline_cell.symbol()
    );
}

// ── Test 6: highlight moves when pane changes ─────────────────────────────

/// Switching pane changes which tab has the underline.
#[test]
fn tab_bar_highlight_moves_when_pane_changes() {
    let accent = agentic_tui::theme::ACCENT;

    // --- Logs pane ---
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let logs_s = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &logs_s)).unwrap();
    let buf_logs = terminal.backend().buffer().clone();

    let (logs_col, label_row_logs) =
        find_in_buffer(&buf_logs, "① logs", 140, 40).expect("'① logs' not found");
    let underline_row = label_row_logs + 1;
    let logs_check = logs_col + 2;
    assert_eq!(
        buf_logs.cell((logs_check, underline_row)).unwrap().symbol(),
        "─",
        "Logs: expected '─' underline at ({logs_check}, {underline_row})"
    );

    // --- Chat pane ---
    let chat_s = AppState {
        focus: Pane::Chat,
        pipeline: vec![],
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &chat_s)).unwrap();
    let buf_chat = terminal.backend().buffer().clone();

    let (chat_col, label_row_chat) =
        find_in_buffer(&buf_chat, "② chat", 140, 40).expect("'② chat' not found");
    let underline_row_chat = label_row_chat + 1;
    let chat_check = chat_col + 2;
    assert_eq!(
        buf_chat.cell((chat_check, underline_row_chat)).unwrap().symbol(),
        "─",
        "Chat: expected '─' underline at ({chat_check}, {underline_row_chat})"
    );

    // Verify Logs tab no longer has underline when Chat is active.
    let logs_check_in_chat_render = logs_col + 2;
    let logs_cell_when_chat = buf_chat
        .cell((logs_check_in_chat_render, underline_row_chat))
        .unwrap();
    assert_ne!(
        logs_cell_when_chat.symbol(),
        "─",
        "Chat pane: expected no '─' underline under '① logs' tab"
    );

    // Also verify ACCENT on chat underline cell.
    let chat_underline_cell = buf_chat.cell((chat_check, underline_row_chat)).unwrap();
    assert_eq!(
        chat_underline_cell.style().fg,
        Some(accent),
        "Chat: expected '─' at ({chat_check}, {underline_row_chat}) to have fg=ACCENT"
    );

    // --- Issue pane ---
    let issue_s = AppState {
        focus: Pane::Issue,
        pipeline: vec![],
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &issue_s)).unwrap();
    let buf_issue = terminal.backend().buffer().clone();

    let (issue_col, label_row_issue) =
        find_in_buffer(&buf_issue, "③ issue", 140, 40).expect("'③ issue' not found");
    let underline_row_issue = label_row_issue + 1;
    let issue_check = issue_col + 2;
    assert_eq!(
        buf_issue.cell((issue_check, underline_row_issue)).unwrap().symbol(),
        "─",
        "Issue: expected '─' underline at ({issue_check}, {underline_row_issue})"
    );
}

// ── Test 7: HEADER_BG continuity ─────────────────────────────────────────

/// Both rows of the tab bar must have at least one cell with bg=HEADER_BG.
#[test]
fn tab_bar_uses_header_bg_continuity() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = logs_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let header_bg = agentic_tui::theme::HEADER_BG;

    // Find the label row.
    let (_col, label_row) =
        find_in_buffer(&buffer, "① logs", 140, 40).expect("'① logs' not found in buffer");
    let underline_row = label_row + 1;

    let label_row_has_header_bg = (0..140u16)
        .map(|x| buffer.cell((x, label_row)).unwrap())
        .any(|cell| cell.style().bg == Some(header_bg));
    assert!(
        label_row_has_header_bg,
        "expected at least one cell on label row {label_row} to have bg=HEADER_BG"
    );

    let underline_row_has_header_bg = (0..140u16)
        .map(|x| buffer.cell((x, underline_row)).unwrap())
        .any(|cell| cell.style().bg == Some(header_bg));
    assert!(
        underline_row_has_header_bg,
        "expected at least one cell on underline row {underline_row} to have bg=HEADER_BG"
    );
}

// ── Test 8: Pane enum — Logs is the default ───────────────────────────────

/// Default AppState must start with focus = Pane::Logs.
#[test]
fn pane_default_is_logs() {
    let s = AppState::default();
    assert_eq!(
        s.focus,
        Pane::Logs,
        "expected default focus = Pane::Logs, got {:?}",
        s.focus
    );
}

// ── Test 9: ToggleFocus cycles three ways ─────────────────────────────────

/// Tab key cycles Logs → Chat → Issue → Logs.
#[test]
fn toggle_focus_cycles_three_panes() {
    use agentic_tui::app::AppEvent;
    let mut s = AppState::default(); // focus = Logs
    assert_eq!(s.focus, Pane::Logs);

    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Chat, "Logs → Chat");

    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Issue, "Chat → Issue");

    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Logs, "Issue → Logs");
}
