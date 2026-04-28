//! Agentic TUI shell — Phase 12.
//!
//! Step 12.2 wired the two-pane layout, focus, and resize. Step 12.3
//! adds the cockpit stepper that mirrors the Tauri `Stepper.tsx`. The
//! render and state logic live here so integration tests can drive
//! them against `ratatui::backend::TestBackend` without spawning a
//! real terminal.

#![deny(unsafe_code)]

pub mod app;
pub mod layout;
pub mod run;
pub mod views;

use ratatui::Frame;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders};

use crate::app::{AppState, Pane};
use crate::layout::compute_panes;

/// Render one frame of the application — the cockpit pane (with its
/// stepper) on the left, an empty bordered chat pane on the right.
/// Chat content is Step 12.4+.
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let (cockpit_area, chat_area) = compute_panes(f.area(), state);

    views::cockpit::render(cockpit_area, state, f);

    let chat_title = if state.focus == Pane::Chat {
        "Chat *"
    } else {
        "Chat"
    };
    let chat_title_style = if state.focus == Pane::Chat {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let chat_block = Block::default()
        .title(chat_title)
        .title_style(chat_title_style)
        .borders(Borders::ALL);
    f.render_widget(chat_block, chat_area);
}
