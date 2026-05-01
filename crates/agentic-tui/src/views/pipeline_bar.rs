//! Spec §4.4 — ASCII pipeline bar (4 rows).
//!
//! Renders per-agent status cards joined by `──▶` connectors:
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │ ✓ 01 Plan   │──▶ │ ● 02 Dev    │──▶ │ ○ 03 QA     │
//! │ DONE        │    │ ACTIVE      │    │ QUEUED      │
//! └─────────────┘    └─────────────┘    └─────────────┘
//! ```
//!
//! Color mapping (all existing palette constants — no new constants):
//! - Done glyph `✓` + DONE word → GREEN
//! - Active glyph `●` + ACTIVE word + card border → YELLOW
//! - Queued glyph `○` + QUEUED word → DIM
//! - Failed glyph `✗` + FAILED word → RED
//! - Connectors `──▶` → BORDER
//! - Default card borders → BORDER
//! - Background of the entire strip → HEADER_BG

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::app::{AgentInstance, AgentRunStatus, AppState};
use crate::theme;

/// Minimum inner card width per spec §4.4 (13 inner cols = 15 total).
const INNER_WIDTH_MIN: u16 = 13;
/// Gap between card right-border and next card left-border: `──▶ ` = 4 cols.
const GAP_WIDTH: u16 = 4;

/// Compute the inner card width for a set of agents.
/// Inner = max(INNER_WIDTH_MIN, longest_content_needed).
/// Content row layout: ` G label ` = 1 + 1 + label_chars + 1 (trailing space before `│`).
/// So inner_needed = label_chars + 3 (space + glyph + space).
fn compute_inner_width(agents: &[AgentInstance]) -> u16 {
    let max_label = agents
        .iter()
        .map(|a| a.label.chars().count() as u16)
        .max()
        .unwrap_or(0);
    // Also account for status words (QUEUED=6, ACTIVE=6, DONE=4, FAILED=6).
    let max_status: u16 = 6;
    // inner = space + glyph + space + label = 3 + max_label
    // Also must fit status word: space + status_word = 1 + max_status
    let content_needed = (max_label + 3).max(max_status + 1);
    content_needed.max(INNER_WIDTH_MIN)
}

/// Returns (glyph string, glyph color, status word, status color, border color)
/// for the given agent status.
fn status_parts(status: AgentRunStatus) -> (&'static str, Color, &'static str, Color, Color) {
    match status {
        AgentRunStatus::Done => ("✓", theme::GREEN, "DONE", theme::GREEN, theme::BORDER),
        AgentRunStatus::Active => ("●", theme::YELLOW, "ACTIVE", theme::YELLOW, theme::YELLOW),
        AgentRunStatus::Queued => ("○", theme::DIM, "QUEUED", theme::DIM, theme::BORDER),
        AgentRunStatus::Failed => ("✗", theme::RED, "FAILED", theme::RED, theme::BORDER),
    }
}

/// Write a single character cell into the buffer.
fn set_cell(buf: &mut Buffer, x: u16, y: u16, sym: &str, fg: Color, bg: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(sym);
        cell.set_style(Style::default().fg(fg).bg(bg));
    }
}

/// Write an ASCII string cell-by-cell (each char is 1 column).
fn set_str(buf: &mut Buffer, x: u16, y: u16, s: &str, fg: Color, bg: Color) {
    for (i, ch) in s.chars().enumerate() {
        set_cell(buf, x + i as u16, y, &ch.to_string(), fg, bg);
    }
}

/// Render a single agent card at position (`x`, `y`). `y` is the top row.
/// `inner_width` is the number of columns between the two `│` borders.
fn render_card(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    agent: &AgentInstance,
    inner_width: u16,
    bg: Color,
) {
    let (glyph, glyph_color, status_word, status_color, border_color) = status_parts(agent.status);
    let card_width = inner_width + 2; // +2 for the two `│` borders

    // Row 0: top border  ┌────...────┐
    set_cell(buf, x, y, "┌", border_color, bg);
    for i in 1..card_width - 1 {
        set_cell(buf, x + i, y, "─", border_color, bg);
    }
    set_cell(buf, x + card_width - 1, y, "┐", border_color, bg);

    // Row 1: content  │ <glyph> <label_padded> │
    // Layout: │(0) ·(1) G(2) ·(3) label(4..card_width-2) │(card_width-1)
    set_cell(buf, x, y + 1, "│", border_color, bg);
    set_cell(buf, x + 1, y + 1, " ", theme::FG, bg);
    set_cell(buf, x + 2, y + 1, glyph, glyph_color, bg);
    set_cell(buf, x + 3, y + 1, " ", theme::FG, bg);
    // label fills from offset 4 to card_width-2 inclusive = inner_width - 3 cols
    let label_width = (inner_width - 3) as usize;
    let label: String = agent.label.chars().take(label_width).collect();
    let padded = format!("{:<width$}", label, width = label_width);
    set_str(buf, x + 4, y + 1, &padded, theme::FG, bg);
    set_cell(buf, x + card_width - 1, y + 1, "│", border_color, bg);

    // Row 2: status word  │ <STATUS_padded> │
    set_cell(buf, x, y + 2, "│", border_color, bg);
    set_cell(buf, x + 1, y + 2, " ", theme::FG, bg);
    let sw_width = (inner_width - 1) as usize; // cols after leading space
    let sw: String = status_word.chars().take(sw_width).collect();
    let sw_padded = format!("{:<width$}", sw, width = sw_width);
    set_str(buf, x + 2, y + 2, &sw_padded, status_color, bg);
    set_cell(buf, x + card_width - 1, y + 2, "│", border_color, bg);

    // Row 3: bottom border  └────...────┘
    set_cell(buf, x, y + 3, "└", border_color, bg);
    for i in 1..card_width - 1 {
        set_cell(buf, x + i, y + 3, "─", border_color, bg);
    }
    set_cell(buf, x + card_width - 1, y + 3, "┘", border_color, bg);
}

/// Render the connector `──▶ ` between two cards at the given top-border row.
/// The connector spans the gap between `card_right_x` (exclusive) and the
/// next card's left border (exclusive), drawn on the content row (y+1).
fn render_connector(buf: &mut Buffer, gap_x: u16, y: u16, bg: Color) {
    // Connector occupies GAP_WIDTH=4 cols: `─`, `─`, `▶`, ` `
    set_cell(buf, gap_x, y, "─", theme::BORDER, bg);
    set_cell(buf, gap_x + 1, y, "─", theme::BORDER, bg);
    set_cell(buf, gap_x + 2, y, "▶", theme::BORDER, bg);
    set_cell(buf, gap_x + 3, y, " ", theme::FG, bg);
}

/// Render the pipeline bar into `area`. `area.height` must be >= 4.
/// When `state.pipeline` is empty, nothing is drawn (caller must ensure
/// area has height 0 by passing a zero-height `Rect`).
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if state.pipeline.is_empty() || area.height < 4 || area.width == 0 {
        return;
    }

    let bg = theme::HEADER_BG;
    let inner_width = compute_inner_width(&state.pipeline);
    let card_width = inner_width + 2;
    let buf = f.buffer_mut();

    // Fill background
    for dy in 0..4u16 {
        for dx in 0..area.width {
            set_cell(buf, area.x + dx, area.y + dy, " ", theme::FG, bg);
        }
    }

    let y = area.y;
    let mut x = area.x;
    let n = state.pipeline.len();

    for (i, agent) in state.pipeline.iter().enumerate() {
        render_card(buf, x, y, agent, inner_width, bg);
        x += card_width;

        if i + 1 < n {
            // Connector `──▶ ` on the top border row between cards.
            render_connector(buf, x, y, bg);

            // Fill rows 1-3 of the gap with spaces.
            for dy in 1..4u16 {
                for dx in 0..GAP_WIDTH {
                    set_cell(buf, x + dx, y + dy, " ", theme::FG, bg);
                }
            }
            x += GAP_WIDTH;
        }
    }
}
