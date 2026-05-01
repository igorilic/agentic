//! Spec §4.6 — Chat pane message blocks.
//!
//! Renders chat messages per spec:
//!   - System: centered `── <text> ──` divider in DIM on HEADER_BG.
//!   - User: label `you` in ACCENT bold; body indented 2 cols in FG.
//!   - Agent: label = agent name in GREEN bold; body indented 2 cols in FG.
//!
//! Slash commands (`/word`) and `@mentions` in body text are highlighted
//! with a dark amber tint (SLASH_TINT) approximating the hand-off
//! `rgba(253,230,138,0.1)` on HEADER_BG.
//!
//! IMPORTANT: Use `.chars().count()` for all column math (multi-byte safety).

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{AppState, ChatMessage};
use crate::theme;
use crate::views::diff;

/// Dark amber tint approximating rgba(253,230,138,0.1) composited on HEADER_BG.
/// Computed: 0.9 * HEADER_BG + 0.1 * (253,230,138).
///   R: 0.9*0x16 + 0.1*253 ≈ 0x2d
///   G: 0.9*0x17 + 0.1*230 ≈ 0x2c
///   B: 0.9*0x1b + 0.1*138 ≈ 0x26
/// Same pattern as T.11.2 ACTIVE_TINT in pipeline_bar.rs.
const SLASH_TINT: Color = Color::Rgb(0x2d, 0x2c, 0x26);

/// Render the chat pane into `area`.
///
/// When `state.current_diff` is `Some`, the entire area is given to the diff
/// renderer (restoring the pre-T.12.2 behavior from `views::chat`). When it
/// is `None`, message blocks are rendered instead.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // Diff view takes over the entire chat pane area when active.
    if let Some(diff_text) = &state.current_diff {
        diff::render(area, diff_text, state.diff_scroll_offset, f);
        return;
    }

    let buf = f.buffer_mut();

    // Fill entire area with HEADER_BG for continuity.
    for dy in 0..area.height {
        for dx in 0..area.width {
            if let Some(cell) = buf.cell_mut((area.x + dx, area.y + dy)) {
                cell.set_symbol(" ");
                cell.set_style(Style::default().bg(theme::HEADER_BG));
            }
        }
    }

    // Row cursor — tracks the next available row within the area.
    let mut row_cursor: u16 = 0;

    for msg in &state.chat {
        if row_cursor >= area.height {
            break;
        }

        match msg {
            ChatMessage::System(text) => {
                render_system(buf, area, row_cursor, text);
                row_cursor += 1;
            }
            ChatMessage::User(body) => {
                let body_lines = body_line_count(body);
                // Label row.
                render_label(buf, area, row_cursor, "you", theme::ACCENT);
                row_cursor += 1;
                // Body rows.
                let body_rows = render_body(buf, area, row_cursor, body, area.height);
                row_cursor += body_rows.min(body_lines as u16);
            }
            ChatMessage::Agent { agent, text } => {
                let body_lines = body_line_count(text);
                // Label row.
                render_label(buf, area, row_cursor, agent, theme::GREEN);
                row_cursor += 1;
                // Body rows.
                let body_rows = render_body(buf, area, row_cursor, text, area.height);
                row_cursor += body_rows.min(body_lines as u16);
            }
        }
    }
}

/// Count the number of visual lines a body string will occupy.
fn body_line_count(body: &str) -> usize {
    if body.is_empty() {
        1
    } else {
        body.lines().count().max(1)
    }
}

/// Render a system divider: `── <text> ──` centered in DIM on HEADER_BG.
fn render_system(buf: &mut Buffer, area: Rect, row_offset: u16, text: &str) {
    let row = area.y + row_offset;
    let max_x = area.x + area.width;

    // Build the full divider string: `── <text> ──`
    let divider = format!("── {text} ──");
    let divider_len = divider.chars().count() as u16;

    // Center it within the pane width.
    let leading = if area.width > divider_len {
        (area.width - divider_len) / 2
    } else {
        0
    };

    let style = Style::default().fg(theme::DIM).bg(theme::HEADER_BG);

    for (col, ch) in (area.x + leading..).zip(divider.chars()) {
        if col >= max_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((col, row)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
    }
}

/// Render a label line (agent name or "you") in `color` bold on HEADER_BG.
fn render_label(buf: &mut Buffer, area: Rect, row_offset: u16, label: &str, color: Color) {
    let row = area.y + row_offset;
    let max_x = area.x + area.width;

    let style = Style::default()
        .fg(color)
        .bg(theme::HEADER_BG)
        .add_modifier(Modifier::BOLD);

    for (col, ch) in (area.x..).zip(label.chars()) {
        if col >= max_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((col, row)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
    }
}

/// Render body lines indented 2 cols. Returns the number of rows written.
/// Scans each line for slash commands and @mentions and highlights them
/// with SLASH_TINT. Stops writing when `row_cursor + lines_written >= max_height`.
fn render_body(buf: &mut Buffer, area: Rect, row_offset: u16, body: &str, max_height: u16) -> u16 {
    let indent: u16 = 2;
    let mut rows_written: u16 = 0;

    for line in body.lines() {
        if row_offset + rows_written >= max_height {
            break;
        }
        let row = area.y + row_offset + rows_written;
        let max_x = area.x + area.width;
        let mut col = area.x + indent;

        // Tokenise the line into (text, is_highlighted) fragments.
        let fragments = tokenise_line(line);

        for (fragment, highlighted) in fragments {
            for ch in fragment.chars() {
                if col >= max_x {
                    break;
                }
                let style = if highlighted {
                    Style::default().fg(theme::YELLOW).bg(SLASH_TINT)
                } else {
                    Style::default().fg(theme::FG).bg(theme::HEADER_BG)
                };
                if let Some(cell) = buf.cell_mut((col, row)) {
                    cell.set_symbol(&ch.to_string());
                    cell.set_style(style);
                }
                col += 1;
            }
        }

        rows_written += 1;
    }

    // If body was empty string (no lines()), still count as 1 row consumed.
    if rows_written == 0 {
        rows_written = 1;
    }

    rows_written
}

/// Tokenise a line into `(fragment_str, is_highlighted)` pairs.
///
/// A token starts with `/` followed by `[a-zA-Z]` or `@` followed by
/// `[a-zA-Z]`. The token continues while chars are `[a-zA-Z0-9_-]`.
/// All other text is non-highlighted.
fn tokenise_line(line: &str) -> Vec<(String, bool)> {
    let mut result: Vec<(String, bool)> = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut current = String::new();

    while i < len {
        let ch = chars[i];
        let starts_token =
            (ch == '/' || ch == '@') && i + 1 < len && chars[i + 1].is_ascii_alphabetic();

        if starts_token {
            // Flush any accumulated plain text first.
            if !current.is_empty() {
                result.push((current.clone(), false));
                current.clear();
            }
            // Collect the token: starts with / or @ then [a-zA-Z][a-zA-Z0-9_-]*
            let mut token = String::new();
            token.push(ch);
            i += 1;
            while i < len && is_token_char(chars[i]) {
                token.push(chars[i]);
                i += 1;
            }
            result.push((token, true));
        } else {
            current.push(ch);
            i += 1;
        }
    }

    if !current.is_empty() {
        result.push((current, false));
    }

    result
}

/// Returns `true` if `ch` is a valid token continuation character.
#[inline]
fn is_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
