//! Agentic TUI shell — Phase 12.
//!
//! Step 12.2: two-pane (cockpit | chat) layout with Tab focus and
//! `[`/`]` resize. The render and state logic live here so integration
//! tests can drive them against `ratatui::backend::TestBackend` without
//! spawning a real terminal.

#![deny(unsafe_code)]

pub mod app;
pub mod layout;

use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders};

use crate::app::{AppState, Pane};
use crate::layout::compute_panes;

/// Render one frame of the application — two bordered panes whose
/// titles read "Cockpit" / "Chat", with " *" appended to the focused
/// pane's title.
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let (cockpit_area, chat_area) = compute_panes(f.area(), state);

    let cockpit_title = if state.focus == Pane::Cockpit {
        "Cockpit *"
    } else {
        "Cockpit"
    };
    let chat_title = if state.focus == Pane::Chat {
        "Chat *"
    } else {
        "Chat"
    };

    let focused_style = Style::default().add_modifier(Modifier::BOLD);

    let cockpit_block = Block::default()
        .title(cockpit_title)
        .borders(Borders::ALL)
        .title_style(if state.focus == Pane::Cockpit {
            focused_style
        } else {
            Style::default()
        });
    let chat_block = Block::default()
        .title(chat_title)
        .borders(Borders::ALL)
        .title_style(if state.focus == Pane::Chat {
            focused_style
        } else {
            Style::default()
        });

    f.render_widget(cockpit_block, cockpit_area);
    f.render_widget(chat_block, chat_area);
}
