//! Chat-pane renderer. Step 12.4 only renders the bordered Block plus
//! a command-mode prompt at the bottom (`:plan hello█`). Step 12.5
//! adds chat scrollback + a real text-input field.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{AppState, Pane};
use crate::modes::Mode;

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

    if let Mode::Command { buffer } = &state.mode {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner);
        // Cursor glyph (█) makes it obvious where typed input lands —
        // ratatui's TestBackend surfaces this character verbatim.
        let prompt = Paragraph::new(format!(":{buffer}█"));
        frame.render_widget(prompt, chunks[1]);
    }
}
