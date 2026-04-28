//! Agentic TUI shell — Phase 12.
//!
//! Step 12.2 wired the two-pane layout, focus, and resize. Step 12.3
//! adds the cockpit stepper. Step 12.4 adds command mode (`:plan`,
//! `:status`, `:q`). The render and state logic live here so
//! integration tests can drive them against
//! `ratatui::backend::TestBackend` without spawning a real terminal.

#![deny(unsafe_code)]

pub mod app;
pub mod layout;
pub mod modes;
pub mod run;
pub mod views;

use ratatui::Frame;

use crate::app::AppState;
use crate::layout::compute_panes;

/// Render one frame of the application — cockpit (stepper) on the left,
/// chat (bordered + optional command prompt) on the right.
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let (cockpit_area, chat_area) = compute_panes(f.area(), state);
    views::cockpit::render(cockpit_area, state, f);
    views::chat::render(chat_area, state, f);
}
