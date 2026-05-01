//! Step T.11.2: ASCII pipeline bar — spec §4.4.
//!
//! Renders a 4-row strip of agent status boxes:
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │ ✓ 01 Arch   │──▶ │ ● 02 Dev    │──▶ │ ○ 03 QA     │
//! │ DONE        │    │ ACTIVE      │    │ QUEUED      │
//! └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! Color contracts per spec §4.4:
//! - Done glyph `✓` → GREEN
//! - Active glyph `●` → YELLOW; active ACTIVE label → YELLOW
//! - Queued glyph `○` → FG_DIM (theme::DIM)
//! - Failed glyph `✗` → RED; failed FAILED label → RED

use agentic_tui::app::{AgentInstance, AgentRunStatus, AppState};
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helper constructors ────────────────────────────────────────────────────

fn architect_done() -> AgentInstance {
    AgentInstance {
        label: "01 Architect".to_string(),
        status: AgentRunStatus::Done,
    }
}

fn developer_active() -> AgentInstance {
    AgentInstance {
        label: "02 Developer".to_string(),
        status: AgentRunStatus::Active,
    }
}

fn qa_queued() -> AgentInstance {
    AgentInstance {
        label: "03 QA".to_string(),
        status: AgentRunStatus::Queued,
    }
}

fn reviewer_queued() -> AgentInstance {
    AgentInstance {
        label: "04 Reviewer".to_string(),
        status: AgentRunStatus::Queued,
    }
}

fn failed_agent() -> AgentInstance {
    AgentInstance {
        label: "01 Architect".to_string(),
        status: AgentRunStatus::Failed,
    }
}

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

/// Return all cells in a given row.
fn row_cells(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> Vec<&ratatui::buffer::Cell> {
    (0..width).map(|x| buffer.cell((x, y)).unwrap()).collect()
}

/// Find the first occurrence of `needle` in the buffer and return the (col, row)
/// of its first character. Returns None if not found.
fn find_in_buffer(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let first_char = needle.chars().next()?;
    let first_str = first_char.to_string();

    for y in 0..height {
        for x in 0..width {
            let cell = buffer.cell((x, y)).unwrap();
            if cell.symbol() == first_str {
                // Check if the remaining chars match
                let matches = needle.chars().enumerate().all(|(i, ch)| {
                    let col = x + i as u16;
                    col < width && buffer.cell((col, y)).unwrap().symbol() == ch.to_string()
                });
                if matches {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

// ── Render setup ───────────────────────────────────────────────────────────

fn make_four_card_state() -> AppState {
    AppState {
        pipeline: vec![
            architect_done(),
            developer_active(),
            qa_queued(),
            reviewer_queued(),
        ],
        ..Default::default()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

/// T.11.2 — Test 1a: Top border row has 4× `┌─` and ZERO `──▶` connectors.
/// Per spec §4.4: the top row is just corners/dashes; connectors belong on the content row.
#[test]
fn top_border_row_has_four_open_corners_and_no_connectors() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Pipeline bar starts at row 2 (after title=0, header=1).
    let top_row = row_string(&buffer, 2, 140);

    let corner_count = top_row.matches("┌─").count();
    assert_eq!(
        corner_count, 4,
        "expected 4× '┌─' in top border row, got {corner_count}; row:\n{top_row}"
    );

    let connector_count = top_row.matches("──▶").count();
    assert_eq!(
        connector_count, 0,
        "expected 0× '──▶' in top border row (connectors belong on content row), got {connector_count}; row:\n{top_row}"
    );
}

/// T.11.2 — Test 1b: Content row (row with glyphs) has 3× `──▶` connectors.
/// Per spec §4.4: `│ ✓ 01 Arch │──▶ │ ● 02 Dev │──▶ │ ...`
#[test]
fn content_row_has_three_connectors_between_cards() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find the content row by locating "01 Architect".
    let (_col, content_row) = find_in_buffer(&buffer, "01 Architect", 140, 40)
        .expect("'01 Architect' not found in buffer");

    let content_row_str = row_string(&buffer, content_row, 140);
    let connector_count = content_row_str.matches("──▶").count();
    assert_eq!(
        connector_count, 3,
        "expected 3× '──▶' on content row {content_row}, got {connector_count}; row:\n{content_row_str}"
    );
}

/// T.11.2 — Test 2: Content row shows glyph+label for each agent.
#[test]
fn content_row_shows_agent_glyphs_and_labels() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 140, 40);

    assert!(
        full.contains("✓ 01 Architect"),
        "expected '✓ 01 Architect' in buffer"
    );
    assert!(
        full.contains("● 02 Developer"),
        "expected '● 02 Developer' in buffer"
    );
    assert!(full.contains("○ 03 QA"), "expected '○ 03 QA' in buffer");
    assert!(
        full.contains("○ 04 Reviewer"),
        "expected '○ 04 Reviewer' in buffer"
    );
}

/// T.11.2 — Test 3: Status row shows status words for each card.
#[test]
fn status_row_shows_status_words() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 140, 40);

    assert!(full.contains("DONE"), "expected 'DONE' in buffer");
    assert!(full.contains("ACTIVE"), "expected 'ACTIVE' in buffer");
    // Two QUEUED entries.
    let queued_count = full.matches("QUEUED").count();
    assert!(
        queued_count >= 2,
        "expected at least 2× 'QUEUED' in buffer, got {queued_count}"
    );
}

/// T.11.2 — Test 4: Active card ACTIVE word is styled in YELLOW.
#[test]
fn active_card_status_word_is_yellow() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let yellow = agentic_tui::theme::YELLOW;
    let (col, row) =
        find_in_buffer(&buffer, "ACTIVE", 140, 40).expect("'ACTIVE' not found in buffer");

    // Check each cell of "ACTIVE" (6 chars) has fg=YELLOW.
    for i in 0..6u16 {
        let cell = buffer.cell((col + i, row)).unwrap();
        assert_eq!(
            cell.style().fg,
            Some(yellow),
            "expected 'ACTIVE' cell {i} at ({}, {row}) to have fg=YELLOW, got {:?}",
            col + i,
            cell.style().fg
        );
    }
}

/// T.11.2 — Test 5: Done card `✓` glyph is styled in GREEN.
#[test]
fn done_card_checkmark_glyph_is_green() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let green = agentic_tui::theme::GREEN;
    let (col, row) = find_in_buffer(&buffer, "✓", 140, 40).expect("'✓' not found in buffer");

    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(green),
        "expected '✓' at ({col}, {row}) to have fg=GREEN, got {:?}",
        cell.style().fg
    );
}

/// T.11.2 — Test 6: Queued card `○` glyph is styled in FG_DIM (theme::DIM).
#[test]
fn queued_card_circle_glyph_is_dim() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let dim = agentic_tui::theme::DIM;

    // Find the first `○` glyph (should be the QA queued card).
    let (col, row) = find_in_buffer(&buffer, "○", 140, 40).expect("'○' not found in buffer");

    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(dim),
        "expected '○' at ({col}, {row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );
}

/// T.11.2 — Test 7: Failed card `✗` glyph is styled in RED and FAILED word appears.
#[test]
fn failed_card_x_glyph_is_red_and_failed_word_present() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        pipeline: vec![failed_agent()],
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let full = buffer_string(&buffer, 140, 40);
    assert!(
        full.contains("FAILED"),
        "expected 'FAILED' in buffer for failed agent"
    );

    let red = agentic_tui::theme::RED;
    let (col, row) = find_in_buffer(&buffer, "✗", 140, 40).expect("'✗' not found in buffer");

    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(red),
        "expected '✗' at ({col}, {row}) to have fg=RED, got {:?}",
        cell.style().fg
    );
}

/// T.11.2 — Test 8: Empty pipeline renders no pipeline rows (no panic, body starts at row 4).
/// Layout with empty pipeline: row 0=title, row 1=issue header, rows 2-3=tab bar, row 4+=body.
#[test]
fn empty_pipeline_renders_no_pipeline_bar_without_panic() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default(); // pipeline = vec![]

    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // With empty pipeline the tab bar lands at rows 2-3. Row 4 is where the body starts.
    // Row 4 should render body content (NOT all HEADER_BG).
    let header_bg = agentic_tui::theme::HEADER_BG;
    let all_header: bool = (0..140u16)
        .map(|x| buffer.cell((x, 4)).unwrap())
        .all(|cell| cell.style().bg == Some(header_bg));
    assert!(
        !all_header,
        "row 4 should render body content when pipeline is empty, not all HEADER_BG"
    );
}

/// T.11.2 — Test 9: Connectors `──▶` on the content row are styled in BORDER color.
#[test]
fn connectors_use_border_color() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let border_color = agentic_tui::theme::BORDER;

    // Connectors live on the content row (where glyphs and labels appear), not the top border.
    let (_col, content_row) = find_in_buffer(&buffer, "01 Architect", 140, 40)
        .expect("'01 Architect' not found in buffer");

    let row = row_cells(&buffer, content_row, 140);

    let arrow_pos = row
        .iter()
        .position(|cell| cell.symbol() == "▶")
        .expect("'▶' not found on content row — connector may be on wrong row");

    let arrow_cell = buffer.cell((arrow_pos as u16, content_row)).unwrap();
    assert_eq!(
        arrow_cell.style().fg,
        Some(border_color),
        "expected '▶' connector at col {arrow_pos} to have fg=BORDER, got {:?}",
        arrow_cell.style().fg
    );
}

/// T.11.2 — Test 10: Active card border uses YELLOW (not default BORDER color).
#[test]
fn active_card_border_is_yellow() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let yellow = agentic_tui::theme::YELLOW;

    // Active card is the second card (02 Developer). Its top-left corner `┌`
    // should be YELLOW. Search row 2 for YELLOW-styled `┌`.
    let found_yellow_corner = (0..140u16).any(|x| {
        let cell = buffer.cell((x, 2)).unwrap();
        cell.symbol() == "┌" && cell.style().fg == Some(yellow)
    });

    assert!(
        found_yellow_corner,
        "expected at least one YELLOW '┌' on the pipeline top border row for the active card"
    );
}

/// T.11.2 (F-2) — Test 11: Active card interior cells have the dark amber tint bg.
/// Per spec §4.4 + hand-off tui-view.jsx: active card uses a tinted background
/// (ACTIVE_TINT = Color::Rgb(0x1c, 0x1a, 0x10)) for interior cells.
#[test]
fn active_card_interior_cells_have_tint_bg() {
    use ratatui::style::Color;

    // This constant must match pipeline_bar.rs ACTIVE_TINT.
    let active_tint = Color::Rgb(0x1c, 0x1a, 0x10);

    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find the `e` in "Developer" which is an interior cell of the active card.
    // The active card content row contains "● 02 Developer".
    let (col, row) =
        find_in_buffer(&buffer, "02 Developer", 140, 40).expect("'02 Developer' not found");

    // The `e` of "Developer" is at offset 3 within "02 Developer" — that's an interior cell.
    // Use `D` at offset 3 of "02 Developer" -> "Developer" starts 3 chars in ("02 ")
    let dev_col = col + 3; // 'D' of 'Developer'
    let cell = buffer.cell((dev_col, row)).unwrap();
    assert_eq!(
        cell.style().bg,
        Some(active_tint),
        "expected active card interior cell at ({dev_col}, {row}) to have bg=ACTIVE_TINT \
         Color::Rgb(0x1c, 0x1a, 0x10), got {:?}",
        cell.style().bg
    );
}

/// T.11.2 (F-2) — Test 12: Non-active card interior cells keep HEADER_BG.
/// Done/Queued cards must NOT receive the active tint.
#[test]
fn non_active_card_interior_cells_keep_header_bg() {
    let header_bg = agentic_tui::theme::HEADER_BG;

    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find an interior cell of the done architect card: "01 Architect"
    let (col, row) =
        find_in_buffer(&buffer, "01 Architect", 140, 40).expect("'01 Architect' not found");

    // Use the `0` of "01 Architect" — a label interior cell.
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().bg,
        Some(header_bg),
        "expected done card interior cell at ({col}, {row}) to have bg=HEADER_BG, got {:?}",
        cell.style().bg
    );
}

/// T.11.2 (TD-1) — Test 13: Single-card pipeline renders without panic, no connectors.
#[test]
fn single_card_pipeline_has_no_connectors() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        pipeline: vec![architect_done()],
        ..Default::default()
    };

    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Top border row: exactly 1× `┌─` and 0× `──▶`.
    let top_row = row_string(&buffer, 2, 140);
    let corner_count = top_row.matches("┌─").count();
    assert_eq!(
        corner_count, 1,
        "expected 1× '┌─' in top border row for single card, got {corner_count}; row:\n{top_row}"
    );
    let connector_count = top_row.matches("──▶").count();
    assert_eq!(
        connector_count, 0,
        "expected 0× '──▶' in top border row for single card, got {connector_count}; row:\n{top_row}"
    );

    // Content row contains the label.
    let full = buffer_string(&buffer, 140, 40);
    assert!(
        full.contains("✓ 01 Architect"),
        "expected '✓ 01 Architect' in buffer for single-card pipeline"
    );
}

// ── T.11.3 tests ──────────────────────────────────────────────────────────

/// T.11.3 — Test 1: Hint row directly below the bottom card border contains
/// the exact affordance string `[a]dd  [r]eorder  [d]rop` (two spaces between
/// each affordance), per spec §4.4 "Hint footer in DIM 1 row below the cards."
#[test]
fn hint_row_below_pipeline_contains_a_r_d_affordances() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Pipeline bar starts at row 2 (after title=0, header=1).
    // 4 card rows (0–3) + 1 hint row = the hint is on row 2+4 = row 6.
    let hint_row = row_string(&buffer, 6, 140);
    assert!(
        hint_row.contains("[a]dd  [r]eorder  [d]rop"),
        "expected hint row at row 6 to contain '[a]dd  [r]eorder  [d]rop'; got:\n{hint_row}"
    );
}

/// T.11.3 — Test 2: Hint row text uses FG_DIM (theme::DIM) fg and HEADER_BG bg.
/// Checks `[a]dd`, `[r]eorder`, `[d]rop` affordance chars plus trailing fill cells.
#[test]
fn hint_row_uses_fg_dim_color() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let dim = agentic_tui::theme::DIM;
    let header_bg = agentic_tui::theme::HEADER_BG;

    // Find the `[` of `[a]dd` on the hint row (row 6).
    let (col, row) = find_in_buffer(&buffer, "[a]dd", 140, 40)
        .expect("'[a]dd' not found in buffer — hint row missing");

    assert_eq!(
        row, 6,
        "expected '[a]dd' to be on row 6 (hint row), found on row {row}"
    );

    // The `a` is at col+1 — check its style.
    let a_cell = buffer.cell((col + 1, row)).unwrap();
    assert_eq!(
        a_cell.style().fg,
        Some(dim),
        "expected 'a' in '[a]dd' at ({}, {row}) to have fg=DIM, got {:?}",
        col + 1,
        a_cell.style().fg
    );
    assert_eq!(
        a_cell.style().bg,
        Some(header_bg),
        "expected 'a' in '[a]dd' at ({}, {row}) to have bg=HEADER_BG, got {:?}",
        col + 1,
        a_cell.style().bg
    );

    // S3: Also check `r` in `[r]eorder` and `d` in `[d]rop` — ensures all three
    // affordance chars are styled DIM, not just `a`.
    let (reorder_col, reorder_row) = find_in_buffer(&buffer, "[r]eorder", 140, 40)
        .expect("'[r]eorder' not found in buffer — hint row missing");
    assert_eq!(
        reorder_row, 6,
        "expected '[r]eorder' on row 6, found row {reorder_row}"
    );
    let r_cell = buffer.cell((reorder_col + 1, reorder_row)).unwrap();
    assert_eq!(
        r_cell.style().fg,
        Some(dim),
        "expected 'r' in '[r]eorder' at ({}, {reorder_row}) to have fg=DIM, got {:?}",
        reorder_col + 1,
        r_cell.style().fg
    );
    assert_eq!(
        r_cell.style().bg,
        Some(header_bg),
        "expected 'r' in '[r]eorder' at ({}, {reorder_row}) to have bg=HEADER_BG, got {:?}",
        reorder_col + 1,
        r_cell.style().bg
    );

    let (drop_col, drop_row) = find_in_buffer(&buffer, "[d]rop", 140, 40)
        .expect("'[d]rop' not found in buffer — hint row missing");
    assert_eq!(
        drop_row, 6,
        "expected '[d]rop' on row 6, found row {drop_row}"
    );
    let d_cell = buffer.cell((drop_col + 1, drop_row)).unwrap();
    assert_eq!(
        d_cell.style().fg,
        Some(dim),
        "expected 'd' in '[d]rop' at ({}, {drop_row}) to have fg=DIM, got {:?}",
        drop_col + 1,
        d_cell.style().fg
    );
    assert_eq!(
        d_cell.style().bg,
        Some(header_bg),
        "expected 'd' in '[d]rop' at ({}, {drop_row}) to have bg=HEADER_BG, got {:?}",
        drop_col + 1,
        d_cell.style().bg
    );

    // S3: Assert at least one trailing-fill cell (past the end of hint text) has
    // bg=HEADER_BG. hint text "[a]dd  [r]eorder  [d]rop" is 24 chars; trailing
    // cells at col >= col+24 on the hint row should still be HEADER_BG.
    let trailing_col = col + 24; // one past the end of the hint text
    assert!(
        trailing_col < 140,
        "trailing_col {trailing_col} is outside buffer width — test setup error"
    );
    let trailing_cell = buffer.cell((trailing_col, row)).unwrap();
    assert_eq!(
        trailing_cell.style().bg,
        Some(header_bg),
        "expected trailing fill cell at ({trailing_col}, {row}) to have bg=HEADER_BG, got {:?}",
        trailing_cell.style().bg
    );
}

/// T.11.3 (S2) — Narrow terminal: drawing a single-card pipeline into a 10×10 terminal
/// must not panic. `set_cell` silently no-ops past buf width, so text clips without
/// crashing. This test locks in the panic-safety contract for narrow terminals.
#[test]
fn hint_row_does_not_panic_on_narrow_terminal() {
    let backend = TestBackend::new(10, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        pipeline: vec![architect_done()],
        ..Default::default()
    };
    // Must not panic — success is simply returning without unwinding.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
}

/// T.11.3 — Test 3: Empty pipeline renders no hint row — body starts at row 4
/// (row 0=title, row 1=issue header, rows 2-3=tab bar, row 4+=body).
/// Previously body started at row 2 (before the tab bar was added in T.11.4).
#[test]
fn empty_pipeline_renders_no_hint_row() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default(); // pipeline = vec![]

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Row 8 is where the hint would be if a 5-row pipeline strip were rendered
    // (rows 2-6 pipeline + rows 7-8 tab bar = hint would be at row 8 if empty pipeline pushed body).
    // With empty pipeline the tab bar lands at rows 2-3 and body at row 4+.
    // Row 8 must NOT have HEADER_BG across all columns — it's inside the body.
    let header_bg = agentic_tui::theme::HEADER_BG;
    let all_header: bool = (0..140u16)
        .map(|x| buffer.cell((x, 8)).unwrap())
        .all(|cell| cell.style().bg == Some(header_bg));
    assert!(
        !all_header,
        "row 8 should not be all HEADER_BG when pipeline is empty (no hint should render there)"
    );

    // Verify: body content is NOT pushed down — it must start at row 4 when pipeline is empty.
    // Row 4 should NOT be all HEADER_BG (the body's Block widgets don't fill with HEADER_BG).
    let all_header_row4: bool = (0..140u16)
        .map(|x| buffer.cell((x, 4)).unwrap())
        .all(|cell| cell.style().bg == Some(header_bg));
    assert!(
        !all_header_row4,
        "row 4 should render body content when pipeline is empty (body must start at row 4)"
    );
}

/// T.11.2 (TD-1) — Test 14: Top border row gaps between cards are HEADER_BG spaces.
/// The 4-column gap after card 0's `┐` and before card 1's `┌` must be space chars
/// with bg=HEADER_BG (no connector overwriting on the top border row).
#[test]
fn gaps_on_top_border_row_are_header_bg_spaces() {
    let header_bg = agentic_tui::theme::HEADER_BG;

    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Find the first `┐` on the top border row (row 2) — right edge of card 0.
    // After it come 4 gap columns before the next `┌`.
    let top_border_row = 2u16;
    let first_close_corner_col = (0..140u16)
        .find(|&x| {
            let cell = buffer.cell((x, top_border_row)).unwrap();
            cell.symbol() == "┐"
        })
        .expect("no '┐' found on top border row");

    // The 4 gap columns immediately follow the first `┐`.
    let gap_start = first_close_corner_col + 1;
    for dx in 0..4u16 {
        let gx = gap_start + dx;
        let cell = buffer.cell((gx, top_border_row)).unwrap();
        assert_eq!(
            cell.symbol(),
            " ",
            "expected gap cell at col {gx} on top border row to be ' ', got {:?}",
            cell.symbol()
        );
        assert_eq!(
            cell.style().bg,
            Some(header_bg),
            "expected gap cell at col {gx} on top border row to have bg=HEADER_BG, got {:?}",
            cell.style().bg
        );
    }
}
