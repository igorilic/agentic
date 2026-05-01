//! Spec §4.5 — Tab bar (2-row strip).
//!
//! Row 0: `① logs   ② chat   ③ issue` left-aligned + `? for help` right-aligned in DIM.
//! Row 1: `─` underline in ACCENT directly below the active tab (the spec's
//!         "2 px ACCENT bottom border" approximated as a 1-row underline in a TUI).
//!
//! Colour mapping (all from `theme`, no new constants):
//!   - Active tab label: fg=ACCENT, bg=HEADER_BG, BOLD modifier
//!   - Inactive tab labels: fg=DIM, bg=HEADER_BG
//!   - Help hint `? for help`: fg=DIM, bg=HEADER_BG
//!   - Underline cells (under active tab): symbol=`─`, fg=ACCENT, bg=HEADER_BG
//!   - All other cells: bg=HEADER_BG
//!
//! IMPORTANT: use `.chars().count()` for all column math — the circled digits
//! `①②③` are multi-byte UTF-8 but occupy 1 terminal column each (T.11.1 F-1).

use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{AppState, Pane};
use crate::theme;

/// Tab definitions in display order: (digit, label, pane variant).
const TABS: &[(&str, &str, Pane)] = &[
    ("①", "logs", Pane::Logs),
    ("②", "chat", Pane::Chat),
    ("③", "issue", Pane::Issue),
];
/// Spaces between adjacent tab labels (3 spaces per spec §4.5 visual contract).
const GAP: &str = "   ";
/// Help hint rendered right-aligned.
const HELP_HINT: &str = "? for help";

/// Write a single cell into the buffer with given symbol, fg, and bg.
fn set_cell(buf: &mut Buffer, x: u16, y: u16, sym: &str, fg: Color, bg: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(sym);
        cell.set_style(Style::default().fg(fg).bg(bg));
    }
}

/// Write a single cell with a style that includes BOLD.
fn set_cell_bold(buf: &mut Buffer, x: u16, y: u16, sym: &str, fg: Color, bg: Color) {
    if let Some(cell) = buf.cell_mut((x, y)) {
        cell.set_symbol(sym);
        cell.set_style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD));
    }
}

/// Render the tab bar into `area`. `area.height` should be 2.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height < 2 || area.width == 0 {
        return;
    }

    let bg = theme::HEADER_BG;
    let buf = f.buffer_mut();
    let label_y = area.y;
    let underline_y = area.y + 1;

    // Fill both rows with HEADER_BG spaces.
    for dy in 0..2u16 {
        for dx in 0..area.width {
            set_cell(buf, area.x + dx, area.y + dy, " ", theme::FG, bg);
        }
    }

    // Build and write the tab labels on label_y.
    // Track column positions as we go (chars not bytes).
    let mut col = area.x;

    for (tab_idx, (digit, label, pane)) in TABS.iter().enumerate() {
        let is_active = state.focus == *pane;

        // Each tab = "① logs" (digit + space + label).
        let tab_text = format!("{digit} {label}");
        let tab_chars: Vec<char> = tab_text.chars().collect();

        // Write each character of the tab label.
        for ch in &tab_chars {
            if col >= area.x + area.width {
                break;
            }
            let sym = ch.to_string();
            if is_active {
                set_cell_bold(buf, col, label_y, &sym, theme::ACCENT, bg);
            } else {
                set_cell(buf, col, label_y, &sym, theme::DIM, bg);
            }
            col += 1;
        }

        // Write gap between tabs (but not after the last one).
        if tab_idx + 1 < TABS.len() {
            let gap_chars: Vec<char> = GAP.chars().collect();
            for ch in &gap_chars {
                if col >= area.x + area.width {
                    break;
                }
                set_cell(buf, col, label_y, &ch.to_string(), theme::DIM, bg);
                col += 1;
            }
        }
    }

    // Write help hint right-aligned on label_y.
    let hint_width = HELP_HINT.chars().count() as u16;
    if area.width >= hint_width {
        let hint_start = area.x + area.width - hint_width;
        for (i, ch) in HELP_HINT.chars().enumerate() {
            let x = hint_start + i as u16;
            set_cell(buf, x, label_y, &ch.to_string(), theme::DIM, bg);
        }
    }

    // Write the underline row: `─` in ACCENT under the active tab only.
    // Re-walk the tab positions to know where each tab starts.
    let mut underline_col = area.x;

    for (tab_idx, (digit, label, pane)) in TABS.iter().enumerate() {
        let is_active = state.focus == *pane;
        // tab text is "① logs" = digit + " " + label
        let tab_text = format!("{digit} {label}");
        let tab_char_count = tab_text.chars().count() as u16;

        if is_active {
            // Draw `─` under the entire tab label.
            for i in 0..tab_char_count {
                let x = underline_col + i;
                if x >= area.x + area.width {
                    break;
                }
                set_cell(buf, x, underline_y, "─", theme::ACCENT, bg);
            }
        }

        underline_col += tab_char_count;

        // Advance past the gap.
        if tab_idx + 1 < TABS.len() {
            underline_col += GAP.chars().count() as u16;
        }
    }
}
