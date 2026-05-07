//! Step 12.5 findings list. Rendered below the stepper inside the
//! cockpit pane. The selected row is prefixed with `>` so the cursor
//! is visible without colour (snapshot tests assert on plain text).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::AppState;
use crate::findings::Finding;

pub fn render(area: Rect, state: &AppState, frame: &mut Frame<'_>) {
    if state.findings.items.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "No findings yet.",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ));
        frame.render_widget(empty, area);
        return;
    }

    let lines: Vec<Line<'_>> = state
        .findings
        .items
        .iter()
        .enumerate()
        .map(|(idx, f)| render_row(idx == state.findings.cursor, f))
        .collect();

    let body = Paragraph::new(lines);
    frame.render_widget(body, area);
}

fn render_row(selected: bool, f: &Finding) -> Line<'_> {
    let cursor = if selected { "> " } else { "  " };
    let triage_badge = f
        .triage
        .map(|t| format!("  [{}]", t.label()))
        .unwrap_or_default();
    let location = match (&f.file, f.line) {
        (Some(file), Some(line)) => format!(" ({file}:{line})"),
        (Some(file), None) => format!(" ({file})"),
        _ => String::new(),
    };

    Line::from(vec![
        Span::styled(cursor.to_string(), cursor_style(selected)),
        Span::styled(severity_glyph(f).to_string(), severity_style(f)),
        Span::raw(format!(" {}{}{}", f.message, location, triage_badge)),
    ])
}

fn cursor_style(selected: bool) -> Style {
    if selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn severity_glyph(f: &Finding) -> char {
    use agentic_core::events::Severity;
    match f.severity {
        Severity::Error => '!',
        Severity::Warning => '⚠',
        Severity::Info => 'i',
    }
}

fn severity_style(f: &Finding) -> Style {
    use agentic_core::events::Severity;
    match f.severity {
        Severity::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Severity::Warning => Style::default().fg(Color::Yellow),
        Severity::Info => Style::default().fg(Color::Blue),
    }
}
