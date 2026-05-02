//! Step T.13.7 — Integration test for `draw_app` composition (spec §4).
//!
//! Locks the top-to-bottom layout contract end-to-end:
//!   Row 0:    title bar
//!   Row 1:    issue header
//!   Rows 2–6: pipeline bar (4 card rows + 1 hint row) — only when non-empty
//!   Rows 7–8: tab bar (label row + underline row)
//!   Rows 9–38:body pane (body range for 140×40 terminal)
//!   Row 39:   status line
//!
//! When pipeline is empty, the strip collapses to 0 rows, so the tab bar
//! lands at rows 2–3 instead of 7–8.

use agentic_tui::app::{AgentInstance, AgentRunStatus, AppState, ChatMessage, Pane};
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helpers ──────────────────────────────────────────────────────────────────

const W: u16 = 140;
const H: u16 = 40;

/// Collect every symbol in a given row into a single string.
fn row_string(buffer: &ratatui::buffer::Buffer, y: u16) -> String {
    (0..W)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Collect all rows into a flat string (for substring searches).
fn buffer_string(buffer: &ratatui::buffer::Buffer) -> String {
    (0..H)
        .flat_map(|y| (0..W).map(move |x| (x, y)))
        .map(|(x, y)| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Find the row index containing `needle` (first match).
fn find_row(buffer: &ratatui::buffer::Buffer, needle: &str) -> Option<u16> {
    let chars: Vec<char> = needle.chars().collect();
    if chars.is_empty() {
        return None;
    }
    for y in 0..H {
        'outer: for x in 0..W {
            for (i, ch) in chars.iter().enumerate() {
                let col = x + i as u16;
                if col >= W {
                    continue 'outer;
                }
                if buffer.cell((col, y)).unwrap().symbol() != ch.to_string() {
                    continue 'outer;
                }
            }
            return Some(y);
        }
    }
    None
}

/// Build a fully-seeded state with non-empty pipeline.
fn full_state() -> AppState {
    AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Test issue".into()),
        pipeline: vec![
            AgentInstance {
                label: "01 Architect".into(),
                status: AgentRunStatus::Done,
            },
            AgentInstance {
                label: "02 Developer".into(),
                status: AgentRunStatus::Active,
            },
        ],
        ..Default::default()
    }
}

// ── Test 1: title bar at row 0 ────────────────────────────────────────────────

/// The title bar signature (`agentic` text in row 0) must land at row 0.
#[test]
fn draw_app_title_bar_at_row_zero() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row0 = row_string(&buffer, 0);
    assert!(
        row0.contains("agentic"),
        "expected 'agentic' in row 0 (title bar), got:\n{row0}"
    );
}

// ── Test 2: issue header at row 1 ─────────────────────────────────────────────

/// The issue header (with AGT-204 run label) must appear in row 1.
#[test]
fn draw_app_issue_header_at_row_one() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row1 = row_string(&buffer, 1);
    assert!(
        row1.contains("AGT-204"),
        "expected 'AGT-204' in row 1 (issue header), got:\n{row1}"
    );
}

// ── Test 3: pipeline bar occupies rows 2–6 (5 rows total) ────────────────────

/// With non-empty pipeline, rows 2–6 must contain pipeline content.
/// The strip is exactly 5 rows: 4 card rows + 1 hint row.
#[test]
fn draw_app_pipeline_bar_at_rows_2_to_6() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // At least one of rows 2–6 must contain pipeline content.
    let pipeline_rows: Vec<String> = (2..=6).map(|y| row_string(&buffer, y)).collect();
    let any_pipeline_content = pipeline_rows.iter().any(|row| {
        row.contains("┌")      // top border of a card
        || row.contains("Architect")
        || row.contains("[a]dd")
    });
    assert!(
        any_pipeline_content,
        "expected pipeline content (card borders or labels) in rows 2–6; rows were:\n{}",
        pipeline_rows.join("\n")
    );

    // Tab bar must NOT appear in rows 2–6 when pipeline is non-empty.
    let no_tab_in_pipeline = pipeline_rows.iter().all(|row| !row.contains("① logs"));
    assert!(
        no_tab_in_pipeline,
        "tab bar content '① logs' must not appear in pipeline rows 2–6"
    );
}

// ── Test 4: pipeline hint at the bottom of the pipeline strip ─────────────────

/// The hint `[a]dd` must appear in the LAST row of the pipeline strip (row 6
/// when pipeline starts at row 2 and is 5 rows tall).
#[test]
fn draw_app_pipeline_hint_at_pipeline_bottom() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let hint_row = find_row(&buffer, "[a]dd")
        .expect("expected '[a]dd' hint to appear somewhere in the buffer");

    // Pipeline starts at row 2, is 5 rows tall → hint row = 2 + 4 = 6.
    assert_eq!(
        hint_row, 6,
        "expected pipeline hint '[a]dd' at row 6 (bottom of 5-row pipeline strip starting at row 2), found at row {hint_row}"
    );
}

// ── Test 5: tab bar at rows 7–8 (with non-empty pipeline) ────────────────────

/// With non-empty pipeline, the tab bar must appear at rows 7–8:
/// row 7 contains `① logs`, row 8 contains the `─` underline.
#[test]
fn draw_app_tab_bar_at_rows_7_and_8() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row7 = row_string(&buffer, 7);
    assert!(
        row7.contains("① logs"),
        "expected '① logs' in row 7 (tab bar label row), got:\n{row7}"
    );

    let row8 = row_string(&buffer, 8);
    assert!(
        row8.contains("─"),
        "expected '─' underline in row 8 (tab bar underline row), got:\n{row8}"
    );
}

// ── Test 6: body starts after tab bar ────────────────────────────────────────

/// The body pane starts at row 9 (after tab bar at rows 7–8 with non-empty
/// pipeline). Row 9 must not contain tab bar or pipeline artefacts.
#[test]
fn draw_app_body_starts_after_tab_bar() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row9 = row_string(&buffer, 9);

    assert!(
        !row9.contains("① logs"),
        "tab bar content '① logs' must not bleed into body row 9; got:\n{row9}"
    );
    assert!(
        !row9.contains("[a]dd"),
        "pipeline hint '[a]dd' must not bleed into body row 9; got:\n{row9}"
    );
}

// ── Test 7: status line at the last row ──────────────────────────────────────

/// The status line must appear at the last row (row H-1 = 39) and contain
/// `NORMAL` (the mode label for the default Normal mode).
#[test]
fn draw_app_status_line_at_last_row() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let last_row = row_string(&buffer, H - 1);
    assert!(
        last_row.contains("NORMAL"),
        "expected 'NORMAL' mode label in last row (row {}), got:\n{last_row}",
        H - 1
    );
}

// ── Test 8: body renders chat content when focus is Chat ─────────────────────

/// When `focus = Pane::Chat` and a chat message is seeded, the message text
/// must appear in the body region (rows 9–38 with non-empty pipeline).
#[test]
fn draw_app_body_renders_chat_pane_when_focus_is_chat() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        focus: Pane::Chat,
        chat: vec![ChatMessage::User("hello from chat".into())],
        ..full_state()
    };
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find the row containing the chat message.
    let msg_row = find_row(&buffer, "hello from chat")
        .expect("expected 'hello from chat' to appear in the buffer");

    // Body starts at row 9 (pipeline 5 rows + tab bar 2 rows + 2 fixed rows).
    // Status line is at row 39, so body ends at row 38.
    assert!(
        (9..=H - 2).contains(&msg_row),
        "expected 'hello from chat' in body range [9..{}], found at row {msg_row}",
        H - 2
    );

    // Pipeline hint must not be in the body area.
    let body_has_pipeline_hint = (9..=H - 2)
        .map(|y| row_string(&buffer, y))
        .any(|row| row.contains("[a]dd"));
    assert!(
        !body_has_pipeline_hint,
        "pipeline hint '[a]dd' must not appear in body rows 9–38"
    );
}

// ── Test 9: help overlay renders above all other content ─────────────────────

/// When `help_open = true`, `KEYBINDINGS` must appear somewhere in the buffer
/// and must be within the terminal area (any row is acceptable — the overlay
/// centers itself).
#[test]
fn draw_app_help_overlay_renders_when_open() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        help_open: true,
        ..full_state()
    };
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let full = buffer_string(&buffer);
    assert!(
        full.contains("KEYBINDINGS"),
        "expected 'KEYBINDINGS' in buffer when help_open=true; got (truncated):\n{}",
        &full[..full.len().min(500)]
    );
}

/// Help overlay must NOT render when `help_open = false`.
#[test]
fn draw_app_help_overlay_hidden_when_closed() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &full_state())).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let full = buffer_string(&buffer);
    assert!(
        !full.contains("KEYBINDINGS"),
        "expected NO 'KEYBINDINGS' when help_open=false; got (truncated):\n{}",
        &full[..full.len().min(500)]
    );
}

// ── Test 10: empty pipeline collapses the strip ───────────────────────────────

/// With `pipeline = vec![]`, the pipeline strip collapses to 0 rows.
/// The tab bar must land at rows 2–3 instead of 7–8.
#[test]
fn draw_app_no_pipeline_collapses_strip() {
    let backend = TestBackend::new(W, H);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        pipeline: vec![],
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // With no pipeline, tab bar should be at rows 2–3.
    let row2 = row_string(&buffer, 2);
    assert!(
        row2.contains("① logs"),
        "expected '① logs' at row 2 when pipeline is empty, got:\n{row2}"
    );

    // Row 3 should contain the underline.
    let row3 = row_string(&buffer, 3);
    assert!(
        row3.contains("─"),
        "expected '─' underline at row 3 when pipeline is empty, got:\n{row3}"
    );

    // Title bar must still be at row 0.
    let row0 = row_string(&buffer, 0);
    assert!(
        row0.contains("agentic"),
        "expected 'agentic' still at row 0 when pipeline is empty, got:\n{row0}"
    );
}
