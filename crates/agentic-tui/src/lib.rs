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

/// Render one frame of the application — title bar at top, issue header
/// (full-width) next, optional pipeline bar, then cockpit (stepper) on the
/// left and chat (bordered + optional command prompt) on the right.
///
/// Row order matches spec §4 (TUI layout):
///   0 — title bar (1 row)
///   1 — issue header (1 row, full width)
///   2 — pipeline bar (5 rows, only when `state.pipeline` is non-empty; else 0)
///   3 — two-pane body (cockpit | chat)
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let total = f.area();
    let pipeline_height: u16 = if state.pipeline.is_empty() { 0 } else { 5 };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),               // row 0: title bar
            Constraint::Length(1),               // row 1: issue header
            Constraint::Length(pipeline_height), // row 2: pipeline bar (0 or 5)
            Constraint::Min(0),                  // row 3: body panes
        ])
        .split(total);

    views::title_bar::render(rows[0], f);
    views::issue_header::render(rows[1], f, state);
    views::pipeline_bar::render(rows[2], f, state);

    let (cockpit_area, chat_area) = compute_panes(rows[3], state);
    views::cockpit::render(cockpit_area, state, f);
    views::chat::render(chat_area, state, f);
}
