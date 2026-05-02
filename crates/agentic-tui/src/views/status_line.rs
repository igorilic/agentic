//! Spec §4.8 — Status / command line (bottom row).
//!
//! Single row, HEADER_BG background throughout.
//!
//! Mode rendering:
//! - NORMAL:  left = DIM hint text; right = `NORMAL` in DIM.
//! - COMMAND: left = `:` (ACCENT bold) + buffer, or DIM placeholder if empty;
//!   right = `COMMAND` in YELLOW.
//! - INSERT:  left = DIM hint (same as NORMAL; compose UI is T.13.6);
//!   right = `INSERT` in GREEN.
//!
//! The flash-message override (spec §4.8 / T.13.4) is NOT wired here yet;
//! that lands in the T.13.4 step.
//!
//! IMPORTANT: use `.chars().count()` for all column math — the `·` separator
//! in the hint string is a multi-byte UTF-8 character.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::AppState;
use crate::modes::Mode;
use crate::theme;

/// DIM hint shown in NORMAL (and INSERT for T.13.3) mode on the left side.
const NORMAL_HINT: &str =
    "Press : for command · ? for help · 1/2/3 to switch panes · y/s/n on permission";

/// Placeholder shown after `:` in COMMAND mode when the buffer is empty.
const CMD_PLACEHOLDER: &str = "add <agent>  ·  rm <agent>  ·  help  ·  q";

/// Render the status line into `area`. `area.height` should be 1.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let buf = f.buffer_mut();

    // Fill entire row with HEADER_BG first.
    fill_bg(buf, area);

    // Compute the right-side mode label and its color.
    let (label, label_fg) = match &state.mode {
        Mode::Normal => ("NORMAL", theme::DIM),
        Mode::Command { .. } => ("COMMAND", theme::YELLOW),
        Mode::Insert => ("INSERT", theme::GREEN),
    };

    // Right-align the label with 1 cell of padding on the right edge.
    let label_chars = label.chars().count() as u16;
    let padding: u16 = 1;
    let label_start = area.width.saturating_sub(label_chars + padding);

    write_str(
        buf,
        area,
        label_start,
        label,
        Style::default()
            .fg(label_fg)
            .bg(theme::HEADER_BG)
            .add_modifier(Modifier::BOLD),
    );

    // Compute the left-side content based on mode.
    // Flash override (spec §4.8 / T.13.4): when set, flash text replaces the
    // hint or command buffer for ~1.6 s. The mode label on the right is unaffected.
    // Clip left-side text at label_start so it never overwrites the right-aligned
    // mode label (F-1: hint overwrites label at width <= 84 cols).
    if let Some(flash) = &state.flash {
        write_str_clipped(
            buf,
            area,
            0,
            label_start,
            &flash.text,
            Style::default().fg(theme::ACCENT).bg(theme::HEADER_BG),
        );
    } else {
        match &state.mode {
            Mode::Normal | Mode::Insert => {
                write_str_clipped(
                    buf,
                    area,
                    0,
                    label_start,
                    NORMAL_HINT,
                    Style::default().fg(theme::DIM).bg(theme::HEADER_BG),
                );
            }
            Mode::Command { buffer } => {
                // Render ':' in ACCENT bold.
                write_str_clipped(
                    buf,
                    area,
                    0,
                    label_start,
                    ":",
                    Style::default()
                        .fg(theme::ACCENT)
                        .bg(theme::HEADER_BG)
                        .add_modifier(Modifier::BOLD),
                );

                if buffer.is_empty() {
                    // Placeholder hint in DIM.
                    write_str_clipped(
                        buf,
                        area,
                        1,
                        label_start,
                        CMD_PLACEHOLDER,
                        Style::default().fg(theme::DIM).bg(theme::HEADER_BG),
                    );
                } else {
                    // Buffer text in FG.
                    write_str_clipped(
                        buf,
                        area,
                        1,
                        label_start,
                        buffer,
                        Style::default().fg(theme::FG).bg(theme::HEADER_BG),
                    );
                    // T.13.3: cursor positioning via f.set_cursor_position is deferred
                    // to keep this function signature buffer-only; T.13.x will add it
                    // if needed. The buffer text renders correctly without it.
                }
            }
        }
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Fill an entire single-row area with HEADER_BG spaces.
fn fill_bg(buf: &mut Buffer, area: Rect) {
    let y = area.y;
    for x in area.x..area.x + area.width {
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(" ");
            cell.set_style(Style::default().bg(theme::HEADER_BG));
        }
    }
}

/// Write `text` starting at column offset `col_offset` within `area`,
/// clipping at the right edge. Uses `.chars()` iteration for multi-byte safety.
fn write_str(buf: &mut Buffer, area: Rect, col_offset: u16, text: &str, style: Style) {
    write_str_clipped(buf, area, col_offset, area.width, text, style);
}

/// Like [`write_str`] but clips at `clip_cols` (measured from `area.x`) rather
/// than the full `area.width`. Use this for left-side content to prevent it from
/// overwriting the right-aligned mode label (F-1).
fn write_str_clipped(
    buf: &mut Buffer,
    area: Rect,
    col_offset: u16,
    clip_cols: u16,
    text: &str,
    style: Style,
) {
    let y = area.y;
    let max_x = area.x + clip_cols.min(area.width);

    for (i, ch) in text.chars().enumerate() {
        let x = area.x + col_offset + i as u16;
        if x >= max_x {
            break;
        }
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(&ch.to_string());
            cell.set_style(style);
        }
    }
}
