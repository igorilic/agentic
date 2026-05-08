//! Spec §4.3 — issue header strip (1 row).
//!
//! Renders `▰ agentic │ AGT-204 <issue title>` aligned left and a
//! `● running MM:SS` pill aligned right, per the design hand-off.
//!
//! Colour mapping (all from `theme`):
//!   • `▰ agentic` — ACCENT bold
//!   • ` │ ` — DIM
//!   • `AGT-204` — FG
//!   • ` <title>` — DIM
//!   • `● running MM:SS` — BLUE (right-aligned)
//!
//! When `run_label` or `run_title` is `None` the row is rendered blank
//! (background fill only — no panic, no stray text).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthChar;

use crate::app::AppState;
use crate::theme;

/// Truncate `s` so that its display width (in terminal columns) is at most
/// `max_width`, appending `…` when truncation occurs.
///
/// Uses `unicode_width` per-char iteration so that CJK wide characters (2
/// columns each) and other non-ASCII chars are handled correctly.
/// Multi-codepoint ZWJ sequences (complex emoji) are NOT specially handled;
/// each codepoint is measured independently. This is acceptable for the
/// issue-header use case (ticket titles are rarely ZWJ emoji strings).
fn truncate_with_ellipsis(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    let ellipsis = '…';
    let ellipsis_width = ellipsis.width().unwrap_or(1);

    // Measure full display width first.
    let full_width: usize = s.chars().map(|c| c.width().unwrap_or(0)).sum();
    if full_width <= max_width {
        return s.to_owned();
    }

    // Need to truncate. Reserve columns for the ellipsis.
    if max_width < ellipsis_width {
        return String::new();
    }
    let budget = max_width - ellipsis_width;

    let mut result = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > budget {
            break;
        }
        result.push(ch);
        used += w;
    }
    result.push(ellipsis);
    result
}

/// Render the issue header into `area`. `area.height` should be 1.
pub fn render(area: Rect, f: &mut Frame<'_>, state: &AppState) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // Build the right-aligned pill first so we know its rendered width.
    let mm_ss = format!(
        "{:02}:{:02}",
        state.run_elapsed_secs / 60,
        state.run_elapsed_secs % 60
    );

    match (&state.run_label, &state.run_title) {
        (Some(label), Some(title)) => {
            let pill_text = format!("● running {mm_ss}");
            // F-1: count chars, not bytes — '●' is 3 UTF-8 bytes but 1 column.
            let pill_width = pill_text.chars().count() as u16;

            // F-3: pulse dot between BLUE (on) and DIM (off) via frame_parity.
            let dot_color = if state.frame_parity {
                theme::DIM
            } else {
                theme::BLUE
            };

            // Width of fixed left content (without title): "▰ agentic │ " + label + " "
            // "▰ agentic" = 9 chars, " │ " = 3, label, " " = 1 before title.
            let prefix_width: usize =
                "▰ agentic".chars().count() + " │ ".chars().count() + label.chars().count() + 1; // leading space before title

            // Compute available columns for the title text.
            let total_width = area.width as usize;
            let available_for_title = total_width
                .saturating_sub(prefix_width)
                .saturating_sub(pill_width as usize);

            // Truncate title to available_for_title columns (with ellipsis if needed).
            let title_display = truncate_with_ellipsis(title, available_for_title);

            // Left group: "▰ agentic │ AGT-204 <title>"
            // We build a single Line so ratatui handles clipping/fill.
            let left_spans = vec![
                Span::styled(
                    "▰ agentic",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" │ ", Style::default().fg(theme::DIM)),
                Span::styled(label.clone(), Style::default().fg(theme::FG)),
                Span::styled(format!(" {title_display}"), Style::default().fg(theme::DIM)),
            ];

            // Measure left content display width.
            let left_width: usize = "▰ agentic".chars().count()
                + " │ ".chars().count()
                + label.chars().count()
                + 1 // space before title
                + title_display.chars().count();

            // Pad between left and right to right-align the pill within area.width.
            let pad_width = total_width
                .saturating_sub(left_width)
                .saturating_sub(pill_width as usize);

            let mut spans = left_spans;
            if pad_width > 0 {
                spans.push(Span::raw(" ".repeat(pad_width)));
            }
            // Build the pill with a pulsing dot and the rest in BLUE.
            spans.push(Span::styled("●", Style::default().fg(dot_color)));
            spans.push(Span::styled(
                format!(" running {mm_ss}"),
                Style::default().fg(theme::BLUE),
            ));

            let line = Line::from(spans);
            // F-2: paint HEADER_BG so the issue header shares the top-chrome surface.
            f.render_widget(
                Paragraph::new(line).style(Style::default().bg(theme::HEADER_BG)),
                area,
            );
        }
        _ => {
            // No active run — render a blank row (background only).
            // F-2: paint HEADER_BG on blank branch too.
            f.render_widget(
                Paragraph::new("").style(Style::default().bg(theme::HEADER_BG)),
                area,
            );
        }
    }
}
