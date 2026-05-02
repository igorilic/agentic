//! Spec §4.9 — Help overlay, toggled by `?` in Normal mode.
//!
//! A centered modal with ACCENT border and HEADER_BG fill. The overlay is
//! drawn LAST in `draw_app` so it appears on top of all other widgets.
//! When `state.help_open` is `false` this function returns immediately
//! without touching the buffer.
//!
//! Column math uses `.chars().count()` for multi-byte box-drawing safety.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::AppState;
use crate::theme;

/// Canonical keybinding rows shown in the modal body.
const BINDINGS: &[(&str, &str)] = &[
    ("Tab", "Cycle focus between panes"),
    ("1 / 2 / 3", "Switch to Logs / Chat / Issue pane"),
    (":", "Enter command mode"),
    ("y / s / n", "Allow once / session / deny permission"),
    ("?", "Toggle this help"),
    ("Esc", "Close help / exit command mode"),
];

/// Modal dimensions (border + content). Width chosen so the longest row fits.
const MODAL_WIDTH: u16 = 52;
/// 1 top border + 1 blank + N binding rows + 1 blank + 1 bottom border.
const MODAL_HEIGHT: u16 = 2 + 6 + 2; // = 10

/// Render the help overlay into the full terminal area.
///
/// Early-returns when `state.help_open == false`.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if !state.help_open {
        return;
    }

    let modal_w = MODAL_WIDTH.min(area.width);
    let modal_h = MODAL_HEIGHT.min(area.height);

    let modal_x = area.x + area.width.saturating_sub(modal_w) / 2;
    let modal_y = area.y + area.height.saturating_sub(modal_h) / 2;

    let modal = Rect {
        x: modal_x,
        y: modal_y,
        width: modal_w,
        height: modal_h,
    };

    let buf = f.buffer_mut();
    render_modal(buf, modal);
}

// ── Internal renderers ────────────────────────────────────────────────────────

fn render_modal(buf: &mut Buffer, modal: Rect) {
    if modal.width == 0 || modal.height == 0 {
        return;
    }

    let accent = Style::default().fg(theme::ACCENT).bg(theme::HEADER_BG);
    let bg = Style::default().bg(theme::HEADER_BG);
    let fg = Style::default().fg(theme::FG).bg(theme::HEADER_BG);
    let key_style = Style::default()
        .fg(theme::ACCENT)
        .bg(theme::HEADER_BG)
        .add_modifier(Modifier::BOLD);

    let max_x = modal.x + modal.width;
    let max_y = modal.y + modal.height;

    // ── Row 0: top border ── KEYBINDINGS ──
    if modal.y < max_y {
        render_top_border(buf, modal.x, modal.y, modal.width, max_x, accent);
    }

    // ── Row 1: blank interior row ──
    if modal.height >= 2 && modal.y + 1 < max_y {
        render_blank_row(buf, modal.x, modal.y + 1, modal.width, max_x, accent, bg);
    }

    // ── Rows 2..(2+N): binding rows ──
    let row_styles = RowStyles {
        border: accent,
        key: key_style,
        desc: fg,
        bg,
    };
    for (i, (key, desc)) in BINDINGS.iter().enumerate() {
        let row_y = modal.y + 2 + i as u16;
        if row_y >= max_y {
            break;
        }
        let row_rect = Rect {
            x: modal.x,
            y: row_y,
            width: modal.width,
            height: 1,
        };
        render_binding_row(buf, row_rect, max_x, &row_styles, key, desc);
    }

    // ── Second-to-last row: blank ──
    let blank2_y = modal.y + 2 + BINDINGS.len() as u16;
    if blank2_y < max_y {
        render_blank_row(buf, modal.x, blank2_y, modal.width, max_x, accent, bg);
    }

    // ── Last row: bottom border ──
    let bot_y = modal.y + modal.height - 1;
    if bot_y < max_y && bot_y >= modal.y {
        render_bottom_border(buf, modal.x, bot_y, modal.width, max_x, accent);
    }
}

/// Top border with inset title: `┌── KEYBINDINGS ──────...──┐`
fn render_top_border(buf: &mut Buffer, x: u16, y: u16, width: u16, max_x: u16, style: Style) {
    if width == 0 {
        return;
    }
    put(buf, x, y, "┌", style);

    let title = "── KEYBINDINGS ──";
    let inner_width = width.saturating_sub(2) as usize;
    let title_chars: Vec<char> = title.chars().collect();
    let mut col = x + 1;
    let mut written = 0usize;

    for ch in &title_chars {
        if written >= inner_width || col >= max_x {
            break;
        }
        put(buf, col, y, &ch.to_string(), style);
        col += 1;
        written += 1;
    }

    // Dash-fill the rest.
    while written < inner_width && col < max_x {
        put(buf, col, y, "─", style);
        col += 1;
        written += 1;
    }

    let tr_x = x + width - 1;
    if tr_x < max_x {
        put(buf, tr_x, y, "┐", style);
    }
}

/// Bottom border: `└──...──┘`
fn render_bottom_border(buf: &mut Buffer, x: u16, y: u16, width: u16, max_x: u16, style: Style) {
    if width == 0 {
        return;
    }
    put(buf, x, y, "└", style);

    let inner_width = width.saturating_sub(2);
    for i in 0..inner_width {
        let cx = x + 1 + i;
        if cx >= max_x {
            break;
        }
        put(buf, cx, y, "─", style);
    }

    let br_x = x + width - 1;
    if br_x < max_x {
        put(buf, br_x, y, "┘", style);
    }
}

/// A row with `│` borders and background-filled interior.
fn render_blank_row(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    max_x: u16,
    border: Style,
    bg: Style,
) {
    if width == 0 {
        return;
    }
    put(buf, x, y, "│", border);
    for cx in (x + 1)..(x + width.saturating_sub(1)).min(max_x) {
        put(buf, cx, y, " ", bg);
    }
    let rb = x + width - 1;
    if rb < max_x && rb > x {
        put(buf, rb, y, "│", border);
    }
}

/// Styles bundle for a binding row — avoids the 11-argument clippy limit.
struct RowStyles {
    border: Style,
    key: Style,
    desc: Style,
    bg: Style,
}

/// A row like `│  <key>     <description>  │`
fn render_binding_row(
    buf: &mut Buffer,
    row: Rect,
    max_x: u16,
    styles: &RowStyles,
    key: &str,
    desc: &str,
) {
    let (x, y, width) = (row.x, row.y, row.width);
    let border = styles.border;
    let key_style = styles.key;
    let desc_style = styles.desc;
    let bg = styles.bg;
    if width < 4 {
        return;
    }
    put(buf, x, y, "│", border);

    let inner_start = x + 1;
    let inner_end = (x + width - 1).min(max_x);
    let mut col = inner_start;

    // Two leading spaces.
    for _ in 0..2 {
        if col < inner_end {
            put(buf, col, y, " ", bg);
            col += 1;
        }
    }

    // Key column — 12 chars wide (pad with spaces).
    let key_col_width = 12usize;
    for ch in key.chars() {
        if col >= inner_end {
            break;
        }
        put(buf, col, y, &ch.to_string(), key_style);
        col += 1;
    }
    // Pad key column.
    let key_len = key.chars().count();
    for _ in key_len..key_col_width {
        if col >= inner_end {
            break;
        }
        put(buf, col, y, " ", bg);
        col += 1;
    }

    // Description.
    for ch in desc.chars() {
        if col >= inner_end {
            break;
        }
        put(buf, col, y, &ch.to_string(), desc_style);
        col += 1;
    }

    // Fill remainder + right border.
    while col < inner_end {
        put(buf, col, y, " ", bg);
        col += 1;
    }

    let rb = x + width - 1;
    if rb < max_x && rb > x {
        put(buf, rb, y, "│", border);
    }
}

// ── Micro-helper ─────────────────────────────────────────────────────────────

fn put(buf: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}
