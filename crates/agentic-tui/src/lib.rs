//! Agentic TUI shell — Phase 12.
//!
//! At Step 12.1 this is just a "Hello Agentic" alt-screen with a
//! `q`-to-quit loop. Subsequent steps add a two-pane layout
//! (cockpit + chat), event subscription, and chat input.
//!
//! The render function lives in `lib.rs` so integration tests can drive
//! it against `ratatui::backend::TestBackend` without spawning a real
//! terminal.

#![deny(unsafe_code)]

use ratatui::Frame;
use ratatui::layout::Alignment;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Render the application's first (and currently only) frame. Called once
/// per redraw by the main loop and by integration tests with a
/// `TestBackend`.
pub fn draw_first_frame(f: &mut Frame<'_>) {
    let area = f.area();
    let block = Block::default().title("Agentic").borders(Borders::ALL);
    let body = Paragraph::new("Hello Agentic — press q to quit")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(body, area);
}
