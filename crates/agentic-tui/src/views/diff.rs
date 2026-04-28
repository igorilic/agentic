//! Step 13.1: unified-diff renderer.
//!
//! Consumes the same `--- a/foo / +++ b/foo / @@ ... @@ / -old / +new`
//! format `agentic-core`'s `build_unified_diff` produces (using the
//! `similar` crate). Lines are classified into a small enum and
//! rendered with +/- colour coding. Syntax highlighting via `syntect`
//! is logged as future work — the primary contract per spec §7.2 /
//! §12.3 is "+/- coloring" + "scrollable", which a flat colour map
//! satisfies.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// `--- a/path` or `+++ b/path` — rendered in cyan/bold.
    FileHeader(String),
    /// `@@ -1,3 +1,3 @@ optional context` — rendered in magenta.
    Hunk(String),
    /// `+added` — green; the inner string omits the leading `+`.
    Add(String),
    /// `-removed` — red; the inner string omits the leading `-`.
    Remove(String),
    /// ` unchanged` (or any other line) — neutral; inner string keeps
    /// its leading space if present.
    Context(String),
}

pub fn parse_unified(text: &str) -> Vec<DiffLine> {
    if text.is_empty() {
        return Vec::new();
    }
    text.lines().map(classify).collect()
}

fn classify(line: &str) -> DiffLine {
    // Order matters: triple-dash and triple-plus are headers, not
    // remove/add lines. Two chars of look-ahead suffice.
    if let Some(rest) = line.strip_prefix("--- ") {
        return DiffLine::FileHeader(format!("--- {rest}"));
    }
    if let Some(rest) = line.strip_prefix("+++ ") {
        return DiffLine::FileHeader(format!("+++ {rest}"));
    }
    if line.starts_with("@@") {
        return DiffLine::Hunk(line.to_string());
    }
    if let Some(rest) = line.strip_prefix('+') {
        return DiffLine::Add(rest.to_string());
    }
    if let Some(rest) = line.strip_prefix('-') {
        return DiffLine::Remove(rest.to_string());
    }
    DiffLine::Context(line.to_string())
}

pub fn render(area: Rect, diff_text: &str, frame: &mut Frame<'_>) {
    let lines: Vec<Line<'_>> = parse_unified(diff_text)
        .into_iter()
        .map(render_line)
        .collect();
    let body = Paragraph::new(lines);
    frame.render_widget(body, area);
}

fn render_line(line: DiffLine) -> Line<'static> {
    match line {
        DiffLine::FileHeader(text) => Line::styled(
            text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        DiffLine::Hunk(text) => Line::styled(text, Style::default().fg(Color::Magenta)),
        // For Add / Remove we re-attach the marker as a styled prefix,
        // so the cell at column 0 still carries the colour. That's
        // what the cell-colour test in tests/diff.rs reads back.
        DiffLine::Add(text) => {
            let style = Style::default().fg(Color::Green);
            Line::from(vec![
                Span::styled("+".to_string(), style),
                Span::styled(text, style),
            ])
        }
        DiffLine::Remove(text) => {
            let style = Style::default().fg(Color::Red);
            Line::from(vec![
                Span::styled("-".to_string(), style),
                Span::styled(text, style),
            ])
        }
        DiffLine::Context(text) => Line::raw(text),
    }
}
