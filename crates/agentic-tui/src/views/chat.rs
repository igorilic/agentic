//! Chat-pane renderer. Step 12.4 only renders the bordered Block plus
//! a command-mode prompt at the bottom (`:plan hello█`). Step 12.5
//! adds chat scrollback + a real text-input field.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{AppState, Pane};
use crate::modes::Mode;
use crate::views::diff;

const HINT: &str = "j/k findings · f/t/i triage · : commands";

pub fn render(area: Rect, state: &AppState, frame: &mut Frame<'_>) {
    let title = if state.focus == Pane::Chat {
        "Chat *"
    } else {
        "Chat"
    };
    let title_style = if state.focus == Pane::Chat {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .title_style(title_style)
        .borders(Borders::ALL);

    // Inner area is the bordered Block minus its 1-cell frame; carve out
    // the bottom row for the command prompt when active. Tests assert
    // the prompt string lands somewhere in the buffer, so the exact
    // line position is incidental — putting it at the bottom matches
    // vim's `:` line.
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // The bottom row is the user-feedback line (one of: command
    // prompt while typing, last_status in red, or the static hint).
    // The body above it shows the diff view when `current_diff` is
    // Some; otherwise it's empty (chat scrollback is a future step).
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);
    let body_area = chunks[0];
    let footer_area = chunks[1];

    if let Some(diff_text) = &state.current_diff {
        diff::render(body_area, diff_text, frame);
    }

    let footer: Paragraph<'_> = match &state.mode {
        // Cursor glyph (█) makes it obvious where typed input lands —
        // ratatui's TestBackend surfaces this character verbatim.
        Mode::Command { buffer } => Paragraph::new(format!(":{buffer}█")),
        Mode::Normal => match &state.last_status {
            Some(msg) => Paragraph::new(Span::styled(
                msg.clone(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            None => Paragraph::new(Span::styled(HINT, Style::default().fg(Color::DarkGray))),
        },
    };
    frame.render_widget(footer, footer_area);
}
