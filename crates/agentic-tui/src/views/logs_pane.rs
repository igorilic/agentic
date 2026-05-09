//! Spec §4.6 — Logs pane.
//!
//! Renders column-aligned rows per spec:
//!   `HH:MM:SS  agent           LEVEL    message`
//!
//! Column widths (8 / 16 / 8 / rest) with 2-space gaps:
//!   - Time:    col 0..8   (8 chars)  — DIM
//!   - Gap:     col 8..10  (2 spaces)
//!   - Agent:   col 10..26 (16 chars) — agent accent colour
//!   - Gap:     col 26..28 (2 spaces)
//!   - Level:   col 28..36 (8 chars)  — level colour
//!   - Gap:     col 36..38 (2 spaces)
//!   - Message: col 38..end           — FG (or styled tool-call fragments)
//!
//! Agent accent colours (from the JSX hand-off `TUILogs.accentFor`):
//!   architect → BLUE, developer → GREEN, qa → PURPLE, reviewer → YELLOW,
//!   unknown   → DIM
//!
//! Tool calls render as `name("arg") → result` with name=BLUE, result=DIM.
//!
//! IMPORTANT: Use `.chars().count()` for all column math (multi-byte safety).

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::app::{AppState, LogEntry, LogLevel, Pane};
use crate::theme;
use crate::views::perm_card;

// Column layout constants (relative to the pane left edge).
const TIME_COL: u16 = 0;
const TIME_WIDTH: u16 = 8;
const AGENT_COL: u16 = TIME_COL + TIME_WIDTH + 2; // 10
const AGENT_WIDTH: u16 = 16;
const LEVEL_COL: u16 = AGENT_COL + AGENT_WIDTH + 2; // 28
const LEVEL_WIDTH: u16 = 8;
const MSG_COL: u16 = LEVEL_COL + LEVEL_WIDTH + 2; // 38

/// Render the logs pane into `area`.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // ── Sticky-tail clamping (GH #100) ───────────────────────────────────────
    // When log_scroll > 0, one body row is consumed by the "+N earlier"
    // indicator, leaving (visible_height - 1) rows for log entries.
    //
    // sticky tail: pin so the last entry is always in the last body row.
    //   - If log fits without scroll: effective_scroll = 0, no indicator,
    //     all visible_height rows available.
    //   - If log overflows AND effective_scroll would be 0: same — no indicator.
    //   - If log overflows AND effective_scroll > 0: indicator takes 1 row, so
    //     capacity = visible_height - 1. We need len - effective_scroll <=
    //     visible_height - 1, i.e. effective_scroll >= len - (visible_height - 1).
    //
    // Two-phase resolution:
    //   1. Compute tentative effective_scroll assuming no indicator.
    //   2. If it's > 0, the indicator appears → capacity shrinks by 1 →
    //      recompute with capacity = visible_height - 1.
    let visible_height = area.height as usize;

    // Record this frame's height so the j/Down handler can compute max_scroll
    // without needing a mutable borrow of AppState. (GH #100 fix-loop 1)
    state.last_known_log_height.set(visible_height);

    let effective_scroll = if state.log_sticky_tail {
        let len = state.log.len();
        if len <= visible_height {
            // Fits without scrolling.
            0
        } else {
            // Indicator will appear (scroll > 0), so capacity = visible_height - 1.
            len.saturating_sub(visible_height.saturating_sub(1))
        }
    } else {
        state.log_scroll
    };

    // When the indicator row is needed, we lose one visible slot for log rows.
    let has_indicator = effective_scroll > 0;
    let log_rows_capacity = if has_indicator {
        visible_height.saturating_sub(1)
    } else {
        visible_height
    };

    // Use a block scope so the mutable borrow of `f` via `buf` ends before
    // we call `perm_card::render(... f)` below.
    let log_rows: u16 = {
        let buf = f.buffer_mut();

        // Fill the entire area with HEADER_BG for continuity.
        for dy in 0..area.height {
            for dx in 0..area.width {
                if let Some(cell) = buf.cell_mut((area.x + dx, area.y + dy)) {
                    cell.set_symbol(" ");
                    cell.set_style(Style::default().bg(theme::HEADER_BG));
                }
            }
        }

        let mut rows_used: u16 = 0;

        // Render the "+N earlier" indicator on the first row if scrolled. (GH #100)
        if has_indicator {
            let indicator = format!("+{} earlier", effective_scroll);
            write_text(
                buf,
                area.x,
                area.y,
                &indicator,
                area.width,
                theme::DIM,
                area.x + area.width,
            );
            rows_used += 1;
        }

        // Render the visible window of log entries.
        let visible_entries = state
            .log
            .iter()
            .skip(effective_scroll)
            .take(log_rows_capacity);

        for entry in visible_entries {
            let row = area.y + rows_used;
            if row >= area.y + area.height {
                break;
            }
            render_entry(buf, area, row, entry);
            rows_used += 1;
        }

        rows_used
    };

    // Remaining area after log rows.
    let after_logs_y = area.y + log_rows.min(area.height);
    let after_logs_height = area.height - log_rows.min(area.height);

    // Reserve space for the permission card (5 rows) if one is pending.
    let perm = if state.focus == Pane::Logs {
        state.pending_perms.first()
    } else {
        None
    };

    // Render perm card immediately after the log rows.
    // Card takes up to 5 rows.
    const CARD_HEIGHT: u16 = 5;
    if let Some(p) = perm
        && after_logs_height > 0
    {
        let card_area = Rect {
            x: area.x,
            y: after_logs_y,
            width: area.width,
            height: after_logs_height.min(CARD_HEIGHT),
        };
        perm_card::render(card_area, f, p);
    }
}

/// Render a single `LogEntry` into buffer row `row`.
fn render_entry(buf: &mut Buffer, area: Rect, row: u16, entry: &LogEntry) {
    let base_x = area.x;
    let max_x = area.x + area.width;

    // Time column — DIM, capped at TIME_WIDTH chars.
    write_text(
        buf,
        base_x + TIME_COL,
        row,
        &entry.timestamp,
        TIME_WIDTH,
        theme::DIM,
        max_x,
    );

    // Agent column — accent colour, capped at AGENT_WIDTH chars.
    let agent_color = agent_color(&entry.agent);
    write_text(
        buf,
        base_x + AGENT_COL,
        row,
        &entry.agent,
        AGENT_WIDTH,
        agent_color,
        max_x,
    );

    // Level column — level colour, capped at LEVEL_WIDTH chars.
    let (level_text, level_color) = level_label(&entry.level);
    write_text(
        buf,
        base_x + LEVEL_COL,
        row,
        level_text,
        LEVEL_WIDTH,
        level_color,
        max_x,
    );

    // Message column — FG (plain) or structured tool-call fragments.
    let msg_x = base_x + MSG_COL;
    if msg_x >= max_x {
        return;
    }
    let msg_width = max_x.saturating_sub(msg_x);

    match &entry.level {
        LogLevel::Tool { name, arg, result } => {
            render_tool_call(buf, msg_x, row, name, arg, result, max_x);
        }
        _ => {
            write_text(buf, msg_x, row, &entry.message, msg_width, theme::FG, max_x);
        }
    }
}

/// Write up to `max_chars` characters of `text` starting at (x, y) in `color`.
/// Stops at `max_x` (absolute column limit).
fn write_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    text: &str,
    max_chars: u16,
    color: Color,
    max_x: u16,
) {
    let end_x = (x + max_chars).min(max_x);
    for (col, ch) in (x..end_x).zip(text.chars()) {
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(Style::default().fg(color).bg(theme::HEADER_BG));
        }
    }
}

/// Render `tool_name("arg") → result` with:
///   - `tool_name` in BLUE
///   - `(` in DIM, `"arg"` (both quote chars + body) in FG, `)` in DIM
///   - ` → result` in DIM
///
/// NOTE: tool names and args are conventionally ASCII (edit_file, bash,
/// read_file, etc.). Non-ASCII args (e.g. Unicode file paths) may misalign
/// visually because col advances by 1 per char rather than by display width.
/// Track as tech-debt if real-world misalignment is observed.
fn render_tool_call(
    buf: &mut Buffer,
    start_x: u16,
    y: u16,
    name: &str,
    arg: &str,
    result: &str,
    max_x: u16,
) {
    let mut col = start_x;

    // tool_name in BLUE
    for ch in name.chars() {
        if col >= max_x {
            return;
        }
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(Style::default().fg(theme::BLUE).bg(theme::HEADER_BG));
        }
        col += 1;
    }

    // `(` in DIM — only the parenthesis, not the quote char
    if col >= max_x {
        return;
    }
    if let Some(cell) = buf.cell_mut((col, y)) {
        cell.set_symbol("(");
        cell.set_style(Style::default().fg(theme::DIM).bg(theme::HEADER_BG));
    }
    col += 1;

    // `"arg"` in FG — both quote chars wrap the arg body, all in FG
    for ch in std::iter::once('"')
        .chain(arg.chars())
        .chain(std::iter::once('"'))
    {
        if col >= max_x {
            return;
        }
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(Style::default().fg(theme::FG).bg(theme::HEADER_BG));
        }
        col += 1;
    }

    // `)` in DIM — only the parenthesis
    if col >= max_x {
        return;
    }
    if let Some(cell) = buf.cell_mut((col, y)) {
        cell.set_symbol(")");
        cell.set_style(Style::default().fg(theme::DIM).bg(theme::HEADER_BG));
    }
    col += 1;

    // ` → ` separator in DIM
    for ch in " → ".chars() {
        if col >= max_x {
            return;
        }
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(Style::default().fg(theme::DIM).bg(theme::HEADER_BG));
        }
        col += 1;
    }

    // result in DIM
    for ch in result.chars() {
        if col >= max_x {
            return;
        }
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(Style::default().fg(theme::DIM).bg(theme::HEADER_BG));
        }
        col += 1;
    }
}

/// Map agent name to its accent colour per the JSX hand-off `TUILogs.accentFor`.
fn agent_color(agent: &str) -> Color {
    match agent {
        "architect" => theme::BLUE,
        "developer" | "tdd-developer" => theme::GREEN,
        "qa" => theme::PURPLE,
        "reviewer" => theme::YELLOW,
        _ => theme::DIM,
    }
}

/// Return the (label_text, colour) for a `LogLevel`.
fn level_label(level: &LogLevel) -> (&'static str, Color) {
    match level {
        LogLevel::Info => ("INFO", theme::DIM),
        LogLevel::Tool { .. } => ("TOOL", theme::BLUE),
        LogLevel::Warn => ("WARN", theme::YELLOW),
        LogLevel::Error => ("ERROR", theme::RED),
    }
}
