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
fn row_cells(
    buffer: &ratatui::buffer::Buffer,
    y: u16,
    width: u16,
) -> Vec<&ratatui::buffer::Cell> {
    (0..width)
        .map(|x| buffer.cell((x, y)).unwrap())
        .collect()
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
                let mut matches = true;
                let mut col = x;
                for ch in needle.chars() {
                    if col >= width {
                        matches = false;
                        break;
                    }
                    let c = buffer.cell((col, y)).unwrap();
                    if c.symbol() != ch.to_string() {
                        matches = false;
                        break;
                    }
                    col += 1;
                }
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

/// T.11.2 — Test 1: Top-row borders contain 4× `┌─` and 3× `──▶`.
#[test]
fn top_row_has_four_open_corners_and_three_connectors() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Pipeline bar starts at row 2 (after title=0, header=1).
    // Top border is at row 2.
    let top_row = row_string(&buffer, 2, 140);

    let corner_count = top_row.matches("┌─").count();
    assert_eq!(
        corner_count, 4,
        "expected 4× '┌─' in top border row, got {corner_count}; row:\n{top_row}"
    );

    let connector_count = top_row.matches("──▶").count();
    assert_eq!(
        connector_count, 3,
        "expected 3× '──▶' in top border row, got {connector_count}; row:\n{top_row}"
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
    assert!(
        full.contains("○ 03 QA"),
        "expected '○ 03 QA' in buffer"
    );
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
    let (col, row) = find_in_buffer(&buffer, "ACTIVE", 140, 40)
        .expect("'ACTIVE' not found in buffer");

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
    let (col, row) = find_in_buffer(&buffer, "✓", 140, 40)
        .expect("'✓' not found in buffer");

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
    let (col, row) = find_in_buffer(&buffer, "○", 140, 40)
        .expect("'○' not found in buffer");

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
    let (col, row) = find_in_buffer(&buffer, "✗", 140, 40)
        .expect("'✗' not found in buffer");

    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(red),
        "expected '✗' at ({col}, {row}) to have fg=RED, got {:?}",
        cell.style().fg
    );
}

/// T.11.2 — Test 8: Empty pipeline renders no pipeline rows (no panic, body starts at row 2).
#[test]
fn empty_pipeline_renders_no_pipeline_bar_without_panic() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default(); // pipeline = vec![]

    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Row 2 should be body content (no HEADER_BG on all cells, same as title_bar test).
    let header_bg = agentic_tui::theme::HEADER_BG;
    let all_header: bool = (0..140u16)
        .map(|x| buffer.cell((x, 2)).unwrap())
        .all(|cell| cell.style().bg == Some(header_bg));
    assert!(
        !all_header,
        "row 2 should render body content when pipeline is empty, not all HEADER_BG"
    );
}

/// T.11.2 — Test 9: Connectors `──▶` in the top border row are styled in BORDER color.
#[test]
fn connectors_use_border_color() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = make_four_card_state();

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let border_color = agentic_tui::theme::BORDER;

    // Find first `──▶` connector in the buffer.
    // `──` is two box-drawing dashes, `▶` is the arrow.
    // Search top border row (row 2).
    let row = row_cells(&buffer, 2, 140);

    let arrow_pos = row.iter().position(|cell| cell.symbol() == "▶")
        .expect("'▶' not found in top border row");

    let arrow_cell = buffer.cell((arrow_pos as u16, 2)).unwrap();
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
