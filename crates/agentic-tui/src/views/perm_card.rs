//! Spec §4.7 — Inline permission card rendered inside the logs pane.
//!
//! The card is 5 rows tall. A RED `┃` left accent appears on the top 3 body
//! rows (command, reason, hotkey) at `area.x`; the card box starts at
//! `area.x + 1`.
//!
//! Column math uses `.chars().count()` for multi-byte safety.

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{PermissionRequest, PermissionRisk};
use crate::theme;

/// Render the permission card for `perm` into `area`.
///
/// `area` must be at least 5 rows tall. Rows beyond `area.height` are
/// silently clipped. The red left-accent `┃` is placed at `area.x`; the box
/// begins at `area.x + 1`.
pub fn render(area: Rect, f: &mut Frame<'_>, perm: &PermissionRequest) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let buf = f.buffer_mut();

    let accent_x = area.x;
    let box_x = area.x + 1;
    // box_width covers from box_x to area.x + area.width (i.e. area.width - 1).
    let box_width = area.width.saturating_sub(1);
    let max_x = area.x + area.width;

    let red_style = Style::default().fg(theme::RED).bg(theme::HEADER_BG);

    // Row 0 — top border (accent + box top).
    render_top_border(buf, area.y, accent_x, box_x, box_width, max_x, perm);

    // Row 1 — command row.
    if area.height >= 2 {
        put_accent(buf, area.y + 1, accent_x, red_style);
        render_command_row(buf, area.y + 1, box_x, box_width, max_x, perm, red_style);
    }

    // Row 2 — reason row.
    if area.height >= 3 {
        put_accent(buf, area.y + 2, accent_x, red_style);
        render_reason_row(buf, area.y + 2, box_x, box_width, max_x, perm, red_style);
    }

    // Row 3 — hotkey row.
    if area.height >= 4 {
        put_accent(buf, area.y + 3, accent_x, red_style);
        render_hotkey_row(buf, area.y + 3, box_x, box_width, max_x, red_style);
    }

    // Row 4 — bottom border.
    if area.height >= 5 {
        put_accent(buf, area.y + 4, accent_x, red_style);
        render_bottom_border(buf, area.y + 4, box_x, box_width, max_x, red_style);
    }
}

// ── Row renderers ─────────────────────────────────────────────────────────────

/// Place the `┃` left accent at (`x`, `y`) in RED.
fn put_accent(buf: &mut Buffer, y: u16, x: u16, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol("┃");
        cell.set_style(style);
    }
}

/// Top border: `┃ ┌─ ⚠ PERM  <agent> requests permission    <RISK> ─┐`
fn render_top_border(
    buf: &mut Buffer,
    y: u16,
    accent_x: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    perm: &PermissionRequest,
) {
    let red_style = Style::default().fg(theme::RED).bg(theme::HEADER_BG);

    // Accent on this row.
    put_accent(buf, y, accent_x, red_style);

    if box_x >= max_x || box_width == 0 {
        return;
    }

    // `┌` at box_x.
    put_char(buf, box_x, y, "┌", red_style);

    let inner_width = box_width.saturating_sub(2) as usize;
    let label = format!("─ ⚠ PERM  {} requests permission", perm.agent);
    let risk_label = risk_str(perm.risk);
    let right = format!("{} ─", risk_label);

    let mut col = box_x + 1;
    let mut written = 0_usize;

    // Write label prefix.
    for ch in label.chars() {
        if written >= inner_width || col >= max_x {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), red_style);
        col += 1;
        written += 1;
    }

    // Dash-fill up to the right label.
    let right_chars: Vec<char> = right.chars().collect();
    let right_start = inner_width.saturating_sub(right_chars.len());
    while written < right_start && written < inner_width && col < max_x {
        put_char(buf, col, y, "─", red_style);
        col += 1;
        written += 1;
    }

    // Write right label.
    for ch in right_chars {
        if written >= inner_width || col >= max_x {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), red_style);
        col += 1;
        written += 1;
    }

    // `┐` at box_x + box_width - 1.
    let tr_x = box_x + box_width - 1;
    if tr_x < max_x
        && let Some(cell) = buf.cell_mut((tr_x, y))
    {
        cell.set_symbol("┐");
        cell.set_style(red_style);
    }
}

/// Command row: `│ $ <command>                        │`
fn render_command_row(
    buf: &mut Buffer,
    y: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    perm: &PermissionRequest,
    red_style: Style,
) {
    put_left_border(buf, y, box_x, red_style);
    if box_width < 2 {
        put_right_border(buf, y, box_x, box_width, max_x, red_style);
        return;
    }

    let inner_start = box_x + 1;
    let inner_end = (box_x + box_width).saturating_sub(1).min(max_x);
    let mut col = inner_start;

    let dim_black = Style::default().fg(theme::DIM).bg(Color::Black);
    let green_black = Style::default().fg(theme::GREEN).bg(Color::Black);
    let blank_black = Style::default().bg(Color::Black);

    // ` $ ` prefix.
    for ch in " $ ".chars() {
        if col >= inner_end {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), dim_black);
        col += 1;
    }

    // Command text in GREEN on Black.
    for ch in perm.command.chars() {
        if col >= inner_end {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), green_black);
        col += 1;
    }

    // Pad.
    while col < inner_end {
        put_char(buf, col, y, " ", blank_black);
        col += 1;
    }

    put_right_border(buf, y, box_x, box_width, max_x, red_style);
}

/// Reason row: `│ <reason> (scope: <scope>)          │`
///
/// Per F2 / hand-off (tui-view.jsx):
///   - `<reason>` in DIM
///   - `" (scope: "` in DIM
///   - `<scope>` in YELLOW
///   - `")"` in DIM
fn render_reason_row(
    buf: &mut Buffer,
    y: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    perm: &PermissionRequest,
    red_style: Style,
) {
    put_left_border(buf, y, box_x, red_style);
    if box_width < 2 {
        put_right_border(buf, y, box_x, box_width, max_x, red_style);
        return;
    }

    let inner_start = box_x + 1;
    let inner_end = (box_x + box_width).saturating_sub(1).min(max_x);
    let mut col = inner_start;

    let dim_style = Style::default().fg(theme::DIM).bg(theme::HEADER_BG);
    let yellow_style = Style::default().fg(theme::YELLOW).bg(theme::HEADER_BG);
    let bg_style = Style::default().bg(theme::HEADER_BG);

    // Leading space + reason in DIM.
    let reason_prefix = format!(" {}", perm.reason);
    for ch in reason_prefix.chars() {
        if col >= inner_end {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), dim_style);
        col += 1;
    }

    // " (scope: " in DIM.
    for ch in " (scope: ".chars() {
        if col >= inner_end {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), dim_style);
        col += 1;
    }

    // Scope value in YELLOW.
    for ch in perm.scope.chars() {
        if col >= inner_end {
            break;
        }
        put_char(buf, col, y, &ch.to_string(), yellow_style);
        col += 1;
    }

    // Closing ")" in DIM.
    if col < inner_end {
        put_char(buf, col, y, ")", dim_style);
        col += 1;
    }

    // Padding.
    while col < inner_end {
        put_char(buf, col, y, " ", bg_style);
        col += 1;
    }

    put_right_border(buf, y, box_x, box_width, max_x, red_style);
}

/// Hotkey row: `│ [y] allow once    [s] session    [n] deny │`
fn render_hotkey_row(
    buf: &mut Buffer,
    y: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    red_style: Style,
) {
    put_left_border(buf, y, box_x, red_style);
    if box_width < 2 {
        put_right_border(buf, y, box_x, box_width, max_x, red_style);
        return;
    }

    let inner_start = box_x + 1;
    let inner_end = (box_x + box_width).saturating_sub(1).min(max_x);
    let mut col = inner_start;

    let green_bold = Style::default()
        .fg(theme::GREEN)
        .bg(theme::HEADER_BG)
        .add_modifier(Modifier::BOLD);
    let red_bold = Style::default()
        .fg(theme::RED)
        .bg(theme::HEADER_BG)
        .add_modifier(Modifier::BOLD);
    let fg_style = Style::default().fg(theme::FG).bg(theme::HEADER_BG);
    let bg_style = Style::default().bg(theme::HEADER_BG);

    // Leading space.
    if col < inner_end {
        put_char(buf, col, y, " ", fg_style);
        col += 1;
    }

    let segments: &[(&str, Style, &str, Style)] = &[
        ("[y]", green_bold, " allow once    ", fg_style),
        ("[s]", green_bold, " session    ", fg_style),
        ("[n]", red_bold, " deny", fg_style),
    ];

    for (bracket, bracket_style, label, label_style) in segments {
        for ch in bracket.chars() {
            if col >= inner_end {
                break;
            }
            put_char(buf, col, y, &ch.to_string(), *bracket_style);
            col += 1;
        }
        for ch in label.chars() {
            if col >= inner_end {
                break;
            }
            put_char(buf, col, y, &ch.to_string(), *label_style);
            col += 1;
        }
    }

    while col < inner_end {
        put_char(buf, col, y, " ", bg_style);
        col += 1;
    }

    put_right_border(buf, y, box_x, box_width, max_x, red_style);
}

/// Bottom border: `└──...──┘`
fn render_bottom_border(
    buf: &mut Buffer,
    y: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    red_style: Style,
) {
    if box_x >= max_x || box_width == 0 {
        return;
    }

    put_char(buf, box_x, y, "└", red_style);

    let inner_width = box_width.saturating_sub(2);
    for (col, _) in ((box_x + 1)..).zip(0..inner_width) {
        if col >= max_x {
            break;
        }
        put_char(buf, col, y, "─", red_style);
    }

    let br_x = box_x + box_width - 1;
    if br_x < max_x
        && let Some(cell) = buf.cell_mut((br_x, y))
    {
        cell.set_symbol("┘");
        cell.set_style(red_style);
    }
}

// ── Micro-helpers ─────────────────────────────────────────────────────────────

fn put_char(buf: &mut Buffer, x: u16, y: u16, symbol: &str, style: Style) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(symbol);
        cell.set_style(style);
    }
}

fn put_left_border(buf: &mut Buffer, y: u16, box_x: u16, style: Style) {
    put_char(buf, box_x, y, "│", style);
}

fn put_right_border(
    buf: &mut Buffer,
    y: u16,
    box_x: u16,
    box_width: u16,
    max_x: u16,
    style: Style,
) {
    let rb_x = box_x + box_width - 1;
    if rb_x < max_x
        && let Some(cell) = buf.cell_mut((rb_x, y))
    {
        cell.set_symbol("│");
        cell.set_style(style);
    }
}

fn risk_str(risk: PermissionRisk) -> &'static str {
    match risk {
        PermissionRisk::Low => "LOW RISK",
        PermissionRisk::Medium => "MEDIUM RISK",
        PermissionRisk::High => "HIGH RISK",
    }
}
