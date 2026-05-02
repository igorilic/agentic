//! Spec §4.6 — Issue pane.
//!
//! Renders the issue body in vertical order:
//!
//!   Row 0: `<run_label>` in ACCENT bold (e.g. "AGT-204")
//!   Row 1: `<run_title>` in bold FG
//!   (blank row)
//!   Row N: label chips — each rendered as `▏<label>▕` with DIM borders
//!          and FG label text. Multiple chips on one line, 1-space gap.
//!   (blank row)
//!   Row N+: description paragraphs — each entry in FG; blank line between.
//!   (blank row)
//!   Row M+: acceptance checklist — each item `[ ]` in DIM + text in FG.
//!
//! HEADER_BG is used as the background colour throughout.
//! All column math uses `.chars().count()` for multi-byte safety.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::AppState;
use crate::theme;

/// Render the issue pane into `area`.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let buf = f.buffer_mut();
    let max_x = area.x + area.width;

    // Fill the entire area with HEADER_BG.
    for dy in 0..area.height {
        for dx in 0..area.width {
            if let Some(cell) = buf.cell_mut((area.x + dx, area.y + dy)) {
                cell.set_symbol(" ");
                cell.set_style(Style::default().bg(theme::HEADER_BG));
            }
        }
    }

    // Row cursor tracks the next available row within the area.
    let mut row: u16 = 0;

    // ── Row 0: issue id in ACCENT bold ──────────────────────────────────────
    if let Some(label) = &state.run_label
        && row < area.height
    {
        let y = area.y + row;
        let style = Style::default()
            .fg(theme::ACCENT)
            .bg(theme::HEADER_BG)
            .add_modifier(Modifier::BOLD);
        write_styled(buf, area.x, y, label, max_x, style);
        row += 1;
    }

    // ── Row 1: title in bold FG ──────────────────────────────────────────────
    if let Some(title) = &state.run_title
        && row < area.height
    {
        let y = area.y + row;
        let style = Style::default()
            .fg(theme::FG)
            .bg(theme::HEADER_BG)
            .add_modifier(Modifier::BOLD);
        write_styled(buf, area.x, y, title, max_x, style);
        row += 1;
    }

    // ── Blank line before chips (only if there is content above) ────────────
    if !state.run_labels.is_empty() && row > 0 {
        row += 1; // skip a blank row
    }

    // ── Label chips ──────────────────────────────────────────────────────────
    if !state.run_labels.is_empty() && row < area.height {
        let y = area.y + row;
        let border_style = Style::default().fg(theme::DIM).bg(theme::HEADER_BG);
        let label_style = Style::default().fg(theme::FG).bg(theme::HEADER_BG);
        let mut col = area.x;

        for (i, chip_text) in state.run_labels.iter().enumerate() {
            // 1-space separator between chips (not before the first one).
            if i > 0 && col + 1 < max_x {
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_symbol(" ");
                    cell.set_style(Style::default().bg(theme::HEADER_BG));
                }
                col += 1;
            }

            if col >= max_x {
                break;
            }

            // Left border: ▏
            if let Some(cell) = buf.cell_mut((col, y)) {
                cell.set_symbol("▏");
                cell.set_style(border_style);
            }
            col += 1;

            // Chip label text in FG.
            for ch in chip_text.chars() {
                if col >= max_x {
                    break;
                }
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_symbol(&ch.to_string());
                    cell.set_style(label_style);
                }
                col += 1;
            }

            // Right border: ▕
            if col < max_x {
                if let Some(cell) = buf.cell_mut((col, y)) {
                    cell.set_symbol("▕");
                    cell.set_style(border_style);
                }
                col += 1;
            }
        }
        row += 1;
    }

    // ── Blank line before body ───────────────────────────────────────────────
    if !state.run_body.is_empty() {
        row += 1;
    }

    // ── Description paragraphs ───────────────────────────────────────────────
    let body_style = Style::default().fg(theme::FG).bg(theme::HEADER_BG);
    for (i, paragraph) in state.run_body.iter().enumerate() {
        // Blank line between paragraphs (not before the first one).
        if i > 0 {
            row += 1;
        }
        if row >= area.height {
            break;
        }
        let y = area.y + row;
        write_styled(buf, area.x, y, paragraph, max_x, body_style);
        row += 1;
    }

    // ── Blank line before acceptance ─────────────────────────────────────────
    if !state.run_acceptance.is_empty() {
        row += 1;
    }

    // ── Acceptance checklist ─────────────────────────────────────────────────
    let prefix = "[ ] ";
    let dim_style = Style::default().fg(theme::DIM).bg(theme::HEADER_BG);
    let fg_style = Style::default().fg(theme::FG).bg(theme::HEADER_BG);
    for item in &state.run_acceptance {
        if row >= area.height {
            break;
        }
        let y = area.y + row;
        let mut col = area.x;

        // `[ ] ` prefix in DIM.
        for ch in prefix.chars() {
            if col >= max_x {
                break;
            }
            if let Some(cell) = buf.cell_mut((col, y)) {
                cell.set_symbol(&ch.to_string());
                cell.set_style(dim_style);
            }
            col += 1;
        }

        // Item text in FG.
        for ch in item.chars() {
            if col >= max_x {
                break;
            }
            if let Some(cell) = buf.cell_mut((col, y)) {
                cell.set_symbol(&ch.to_string());
                cell.set_style(fg_style);
            }
            col += 1;
        }

        row += 1;
    }
}

/// Write `text` starting at `(x, y)` with `style`, stopping at `max_x`.
fn write_styled(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    text: &str,
    max_x: u16,
    style: Style,
) {
    for (col, ch) in (x..).zip(text.chars()) {
        if col >= max_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((col, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
    }
}
