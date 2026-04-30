//! Agentic TUI shell — Phase 12.
//!
//! Step 12.2 wired the two-pane layout, focus, and resize. Step 12.3
//! adds the cockpit stepper. Step 12.4 adds command mode (`:plan`,
//! `:status`, `:q`). The render and state logic live here so
//! integration tests can drive them against
//! `ratatui::backend::TestBackend` without spawning a real terminal.

#![deny(unsafe_code)]

pub mod app;
pub mod findings;
pub mod layout;
pub mod modes;
pub mod run;
pub mod theme;
pub mod views;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::AppState;
use crate::layout::compute_panes;

/// Render one frame of the application — title bar at top, then cockpit
/// (stepper) on the left and chat (bordered + optional command prompt)
/// on the right.
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let total = f.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(total);

    views::title_bar::render(rows[0], f);

    let (cockpit_area, chat_area) = compute_panes(rows[1], state);
    views::cockpit::render(cockpit_area, state, f);
    views::chat::render(chat_area, state, f);
}
