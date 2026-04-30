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

use crate::app::AppState;
use crate::theme;

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
            let pill_width = pill_text.len() as u16;

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
                Span::styled(format!(" {title}"), Style::default().fg(theme::DIM)),
            ];

            // Measure left content width (in chars; all ASCII-safe spans).
            let left_width: usize = left_spans.iter().map(|s| s.content.chars().count()).sum();

            // Pad between left and right to right-align the pill within area.width.
            let total_width = area.width as usize;
            let pad_width = total_width
                .saturating_sub(left_width)
                .saturating_sub(pill_width as usize);

            let mut spans = left_spans;
            if pad_width > 0 {
                spans.push(Span::raw(" ".repeat(pad_width)));
            }
            spans.push(Span::styled(pill_text, Style::default().fg(theme::BLUE)));

            let line = Line::from(spans);
            f.render_widget(Paragraph::new(line), area);
        }
        _ => {
            // No active run — render a blank row (background only).
            f.render_widget(Paragraph::new(""), area);
        }
    }
}
