//! Step T.12.3: Issue pane tests.
//!
//! Spec §4.6 Issue variant:
//!   - Issue ID (`run_label`) in ACCENT bold.
//!   - Title (`run_title`) bold FG.
//!   - Label chips `▏<label>▕` with DIM borders and FG text.
//!   - Description paragraphs each in FG.
//!   - Acceptance checklist prefixed `[ ]` in DIM, item text in FG.
//!
//! Body restructure tests (single-pane visibility) also live here.

use agentic_tui::app::{AppState, ChatMessage, LogEntry, LogLevel, Pane};
use agentic_tui::draw_app;
use agentic_tui::theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Seed a full AppState for the issue pane, focused on Issue.
fn issue_state() -> AppState {
    AppState {
        focus: Pane::Issue,
        run_label: Some("AGT-204".into()),
        run_title: Some("Add multi-tenant rate limiting".into()),
        run_labels: vec!["backend".into(), "api".into()],
        run_body: vec!["First paragraph.".into(), "Second paragraph.".into()],
        run_acceptance: vec![
            "Limits applied per tenant".into(),
            "Rate enforced via Redis".into(),
        ],
        ..Default::default()
    }
}

/// Render draw_app at `width×height` and return a cloned buffer.
fn render_at(state: &AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Render at 100×30 (plenty of space for all issue content).
fn render(state: &AppState) -> ratatui::buffer::Buffer {
    render_at(state, 100, 30)
}

/// Find the first occurrence of `needle` scanning cell-by-cell in a row range.
/// Returns `(col, row)` of the first character of the match, or `None`.
fn find_in_rows(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    row_start: u16,
    row_end: u16,
) -> Option<(u16, u16)> {
    let chars: Vec<char> = needle.chars().collect();
    if chars.is_empty() {
        return None;
    }
    for y in row_start..row_end {
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

/// Find the first occurrence of `needle` in the full buffer.
fn find_in_buffer(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    find_in_rows(buffer, needle, width, 0, height)
}

/// Find the first occurrence of `needle` in the body area (rows 4+).
/// Row 0=title, 1=issue header, 2-3=tab bar; row 4 is body start
/// (pipeline bar is 0 because pipeline is empty).
fn find_in_body(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    find_in_rows(buffer, needle, width, 4, height)
}

// ── Test 1: id in ACCENT bold ─────────────────────────────────────────────────

/// `run_label = "AGT-204"` — find the cell in the body area (row 4+).
/// Assert `fg == ACCENT` AND `add_modifier.contains(BOLD)`.
///
/// We search in body rows (4+) to avoid the issue header at row 1,
/// which renders the same label in FG (not ACCENT).
#[test]
fn issue_pane_renders_id_in_accent_bold() {
    let state = issue_state();
    let buffer = render(&state);

    let (col, row) =
        find_in_body(&buffer, "AGT-204", 100, 30).expect("'AGT-204' not found in body area");
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::ACCENT),
        "expected 'A' of 'AGT-204' at ({col},{row}) to have fg=ACCENT, got {:?}",
        cell.style().fg
    );
    assert!(
        cell.style().add_modifier.contains(Modifier::BOLD),
        "expected 'A' of 'AGT-204' at ({col},{row}) to have BOLD modifier, got {:?}",
        cell.style().add_modifier
    );
}

// ── Test 2: title in bold FG ─────────────────────────────────────────────────

/// `run_title = "Add multi-tenant rate limiting"` — find in body area (row 4+).
/// Assert `add_modifier.contains(BOLD)`.
///
/// We search in body rows (4+) to avoid the issue header at row 1,
/// which renders the same title in DIM (not BOLD FG).
#[test]
fn issue_pane_renders_title_in_bold_fg() {
    let state = issue_state();
    let buffer = render(&state);

    let (col, row) = find_in_body(&buffer, "Add multi-tenant", 100, 30)
        .expect("'Add multi-tenant' not found in body area");
    let cell = buffer.cell((col, row)).unwrap();
    assert!(
        cell.style().add_modifier.contains(Modifier::BOLD),
        "expected title to have BOLD modifier at ({col},{row}), got {:?}",
        cell.style().add_modifier
    );
}

// ── Test 3: label chips with side borders ─────────────────────────────────────

/// `run_labels = ["backend", "api"]` — render must contain both
/// `▏backend▕` and `▏api▕` (spec §4.6: "label chips with 1 px border").
#[test]
fn issue_pane_renders_label_chips_with_side_borders() {
    let state = issue_state();
    let buffer = render(&state);

    // Both chips must appear in body rows (4+).
    // Only ▏…▕ (U+258F/U+2595) is spec-compliant; │…│ (U+2502) is rejected.
    assert!(
        find_in_body(&buffer, "▏backend▕", 100, 30).is_some(),
        "expected 'backend' chip with ▏…▕ side borders in body area"
    );
    assert!(
        find_in_body(&buffer, "▏api▕", 100, 30).is_some(),
        "expected 'api' chip with ▏…▕ side borders in body area"
    );

    // Chips appear in declared order: backend row <= api row (or same row).
    let backend_pos =
        find_in_body(&buffer, "backend", 100, 30).expect("'backend' not found in body area");
    let api_pos = find_in_body(&buffer, "api", 100, 30).expect("'api' not found in body area");
    assert!(
        backend_pos.1 <= api_pos.1,
        "expected 'backend' chip (row {}) to appear before or on same row as 'api' (row {})",
        backend_pos.1,
        api_pos.1
    );
    // If same row, backend must be at lower column.
    if backend_pos.1 == api_pos.1 {
        assert!(
            backend_pos.0 < api_pos.0,
            "on same row, expected 'backend' col ({}) < 'api' col ({})",
            backend_pos.0,
            api_pos.0
        );
    }
}

// ── Test 4: description paragraphs in FG ─────────────────────────────────────

/// Both `run_body` paragraphs must be visible and the first cell of
/// "First paragraph." must have `fg == FG`.
#[test]
fn issue_pane_renders_description_paragraphs_in_fg() {
    let state = issue_state();
    let buffer = render(&state);

    let (col, row) = find_in_body(&buffer, "First paragraph.", 100, 30)
        .expect("'First paragraph.' not found in body area");
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::FG),
        "expected 'F' of 'First paragraph.' at ({col},{row}) to have fg=FG, got {:?}",
        cell.style().fg
    );

    // Second paragraph must also be present and have a blank line between them.
    let (_, second_row) = find_in_body(&buffer, "Second paragraph.", 100, 30)
        .expect("'Second paragraph.' not found in body area");
    assert_eq!(
        second_row,
        row + 2,
        "expected 'Second paragraph.' at row {} (first_row {} + 2 for blank-line gap), got {}",
        row + 2,
        row,
        second_row
    );
}

// ── Test 5: acceptance items as unchecked boxes ───────────────────────────────

/// `run_acceptance = ["Limits applied per tenant", …]`
/// Each row must begin with `[ ]` (DIM) followed by item text (FG).
#[test]
fn issue_pane_renders_acceptance_as_unchecked_boxes() {
    let state = issue_state();
    let buffer = render(&state);

    // Find "[ ]" prefix in body area (row 4+).
    let unchecked_pos =
        find_in_body(&buffer, "[ ]", 100, 30).expect("'[ ]' prefix not found in body area");
    let (col, row) = unchecked_pos;

    // The `[` cell must be DIM.
    let bracket_cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        bracket_cell.style().fg,
        Some(theme::DIM),
        "expected '[' of '[ ]' at ({col},{row}) to have fg=DIM, got {:?}",
        bracket_cell.style().fg
    );

    // The item text "Limits applied per tenant" must be present.
    assert!(
        find_in_body(&buffer, "Limits applied per tenant", 100, 30).is_some(),
        "expected acceptance item text 'Limits applied per tenant' in body area"
    );

    // Find the "L" of the item text and assert it has fg=FG.
    let (item_col, item_row) = find_in_body(&buffer, "Limits applied per tenant", 100, 30)
        .expect("'Limits applied per tenant' not found in body area");
    let item_cell = buffer.cell((item_col, item_row)).unwrap();
    assert_eq!(
        item_cell.style().fg,
        Some(theme::FG),
        "expected 'L' of acceptance item at ({item_col},{item_row}) to have fg=FG, got {:?}",
        item_cell.style().fg
    );
}

// ── Test 6: empty state does not panic ───────────────────────────────────────

/// All `run_*` fields None / empty — render must not panic.
#[test]
fn issue_pane_handles_empty_state_gracefully() {
    let state = AppState {
        focus: Pane::Issue,
        ..Default::default()
    };
    // Must not panic.
    render(&state);
}

// ── Test 7: no panic on narrow terminal ──────────────────────────────────────

/// Render at 30×10 — must not panic.
#[test]
fn issue_pane_does_not_panic_on_narrow_terminal() {
    let state = issue_state();
    // Must not panic.
    render_at(&state, 30, 10);
}

// ── Test 8: HEADER_BG continuity ─────────────────────────────────────────────

/// At least one cell in the issue pane area must have `bg == HEADER_BG`.
#[test]
fn issue_pane_uses_header_bg_continuity() {
    let state = issue_state();
    let buffer = render(&state);

    let has_header_bg = (0..30_u16).any(|y| {
        (0..100_u16).any(|x| {
            buffer
                .cell((x, y))
                .map(|c| c.style().bg == Some(theme::HEADER_BG))
                .unwrap_or(false)
        })
    });
    assert!(
        has_header_bg,
        "expected at least one cell with bg=HEADER_BG for issue pane continuity"
    );
}

// ── Test 9a: single-pane — only logs visible when focus=Logs ─────────────────

/// When focus=Logs: log content must be visible, chat/issue markers must not.
#[test]
fn body_renders_only_logs_when_focus_is_logs() {
    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log: vec![LogEntry {
            timestamp: "12:34:56".into(),
            agent: "architect".into(),
            level: LogLevel::Info,
            message: "LOG_VISIBLE_MARKER".into(),
        }],
        chat: vec![
            ChatMessage::System("CHAT_DIVIDER".into()),
            ChatMessage::User("CHAT_USER_TEXT".into()),
        ],
        // Seed issue fields so if issue pane leaked, we'd see AGT-LOGTEST.
        run_label: Some("AGT-LOGTEST".into()),
        ..Default::default()
    };

    let buffer = render_at(&state, 100, 30);

    // Log content must be visible.
    assert!(
        find_in_buffer(&buffer, "LOG_VISIBLE_MARKER", 100, 30).is_some(),
        "expected log content 'LOG_VISIBLE_MARKER' visible in Logs focus"
    );

    // Chat-specific markers must NOT be visible.
    assert!(
        find_in_buffer(&buffer, "CHAT_DIVIDER", 100, 30).is_none(),
        "expected chat divider 'CHAT_DIVIDER' to be absent in Logs focus"
    );

    // Issue content must NOT be visible.
    assert!(
        find_in_buffer(&buffer, "AGT-LOGTEST", 100, 30).is_none(),
        "expected issue id 'AGT-LOGTEST' to be absent in Logs focus"
    );
}

// ── Test 9b: single-pane — only chat visible when focus=Chat ─────────────────

/// When focus=Chat: chat content visible, log/issue content not.
#[test]
fn body_renders_only_chat_when_focus_is_chat() {
    let state = AppState {
        focus: Pane::Chat,
        pipeline: vec![],
        log: vec![LogEntry {
            timestamp: "12:34:56".into(),
            agent: "architect".into(),
            level: LogLevel::Info,
            message: "LOG_HIDDEN_MARKER".into(),
        }],
        chat: vec![ChatMessage::User("CHAT_USER_ONLY".into())],
        run_label: Some("AGT-CHATTEST".into()),
        ..Default::default()
    };

    let buffer = render_at(&state, 100, 30);

    // Chat content ("you" label) must be visible.
    assert!(
        find_in_buffer(&buffer, "you", 100, 30).is_some(),
        "expected 'you' user label visible in Chat focus"
    );

    // Log content must NOT be visible.
    assert!(
        find_in_buffer(&buffer, "LOG_HIDDEN_MARKER", 100, 30).is_none(),
        "expected log content 'LOG_HIDDEN_MARKER' to be absent in Chat focus"
    );

    // Issue content must NOT be visible.
    assert!(
        find_in_buffer(&buffer, "AGT-CHATTEST", 100, 30).is_none(),
        "expected issue id 'AGT-CHATTEST' to be absent in Chat focus"
    );
}

// ── Test 9c: single-pane — only issue visible when focus=Issue ───────────────

/// When focus=Issue: issue content visible in body area, log/chat content not.
#[test]
fn body_renders_only_issue_when_focus_is_issue() {
    let state = AppState {
        focus: Pane::Issue,
        pipeline: vec![],
        log: vec![LogEntry {
            timestamp: "12:34:56".into(),
            agent: "architect".into(),
            level: LogLevel::Info,
            message: "LOG_ISSUE_HIDDEN".into(),
        }],
        chat: vec![ChatMessage::User("CHAT_ISSUE_HIDDEN".into())],
        run_label: Some("AGT-999".into()),
        run_title: Some("Issue pane title".into()),
        ..Default::default()
    };

    let buffer = render_at(&state, 100, 30);

    // Issue content must be visible in body area (row 4+).
    // Note: run_label also appears in issue_header (row 1) with FG styling —
    // we search body area to confirm the issue_pane itself renders it.
    assert!(
        find_in_body(&buffer, "AGT-999", 100, 30).is_some(),
        "expected 'AGT-999' visible in Issue pane body"
    );

    // Log content must NOT be visible anywhere.
    assert!(
        find_in_buffer(&buffer, "LOG_ISSUE_HIDDEN", 100, 30).is_none(),
        "expected log content 'LOG_ISSUE_HIDDEN' to be absent in Issue focus"
    );

    // Chat content must NOT be visible anywhere.
    assert!(
        find_in_buffer(&buffer, "CHAT_ISSUE_HIDDEN", 100, 30).is_none(),
        "expected chat content 'CHAT_ISSUE_HIDDEN' to be absent in Issue focus"
    );
}

// ── Test 10: DESCRIPTION section header in DIM ───────────────────────────────

/// The "DESCRIPTION" header must appear in DIM fg, on the row immediately
/// above the first paragraph (with one blank-line separator row between
/// the previous block and the header, and the header on the row right before
/// the paragraph).
#[test]
fn issue_pane_renders_description_section_header_in_dim() {
    let state = issue_state();
    let buffer = render(&state);

    // Find the "D" of "DESCRIPTION".
    let (col, header_row) = find_in_body(&buffer, "DESCRIPTION", 100, 30)
        .expect("'DESCRIPTION' header not found in body area");
    let cell = buffer.cell((col, header_row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::DIM),
        "expected 'D' of 'DESCRIPTION' at ({col},{header_row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );

    // The first paragraph must appear immediately after the header (no blank row
    // between the header and its content).
    let (_, para_row) = find_in_body(&buffer, "First paragraph.", 100, 30)
        .expect("'First paragraph.' not found in body area");
    assert_eq!(
        para_row,
        header_row + 1,
        "expected 'First paragraph.' at header_row+1 ({}), got {}",
        header_row + 1,
        para_row
    );
}

// ── Test 11: ACCEPTANCE section header in DIM ────────────────────────────────

/// The "ACCEPTANCE" header must appear in DIM fg, on the row immediately
/// above the first acceptance item.
#[test]
fn issue_pane_renders_acceptance_section_header_in_dim() {
    let state = issue_state();
    let buffer = render(&state);

    let (col, header_row) = find_in_body(&buffer, "ACCEPTANCE", 100, 30)
        .expect("'ACCEPTANCE' header not found in body area");
    let cell = buffer.cell((col, header_row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::DIM),
        "expected 'A' of 'ACCEPTANCE' at ({col},{header_row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );
}

// ── Test 12: skips DESCRIPTION header when body is empty ─────────────────────

/// When `run_body` is empty, "DESCRIPTION" must NOT appear in the buffer.
#[test]
fn issue_pane_skips_description_header_when_body_empty() {
    let state = AppState {
        focus: Pane::Issue,
        run_label: Some("AGT-100".into()),
        run_title: Some("Some title".into()),
        run_labels: vec!["backend".into()],
        run_body: vec![],
        run_acceptance: vec!["An acceptance item".into()],
        ..Default::default()
    };
    let buffer = render(&state);

    assert!(
        find_in_body(&buffer, "DESCRIPTION", 100, 30).is_none(),
        "expected 'DESCRIPTION' header to be absent when run_body is empty"
    );
}

// ── Test 13: skips ACCEPTANCE header when acceptance is empty ────────────────

/// When `run_acceptance` is empty, "ACCEPTANCE" must NOT appear in the buffer.
#[test]
fn issue_pane_skips_acceptance_header_when_acceptance_empty() {
    let state = AppState {
        focus: Pane::Issue,
        run_label: Some("AGT-100".into()),
        run_title: Some("Some title".into()),
        run_labels: vec!["backend".into()],
        run_body: vec!["A paragraph.".into()],
        run_acceptance: vec![],
        ..Default::default()
    };
    let buffer = render(&state);

    assert!(
        find_in_body(&buffer, "ACCEPTANCE", 100, 30).is_none(),
        "expected 'ACCEPTANCE' header to be absent when run_acceptance is empty"
    );
}
