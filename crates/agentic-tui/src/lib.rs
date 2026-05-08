//! Agentic TUI shell — Phase 12.
//!
//! Step 12.2 wired the two-pane layout, focus, and resize. Step 12.3
//! adds the cockpit stepper. Step 12.4 adds command mode (`:plan`,
//! `:status`, `:q`). The render and state logic live here so
//! integration tests can drive them against
//! `ratatui::backend::TestBackend` without spawning a real terminal.

#![deny(unsafe_code)]

pub mod app;
pub mod modes;
pub mod run;
pub mod theme;
pub mod views;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::{AppState, Pane};

/// Render one frame of the application — title bar at top, issue header
/// (full-width) next, optional pipeline bar, then the tab bar, then the body,
/// and finally the status line at the very bottom.
///
/// Row order matches spec §4 (TUI layout):
///   0 — title bar (1 row)
///   1 — issue header (1 row, full width)
///   2 — pipeline bar (5 rows, only when `state.pipeline` is non-empty; else 0)
///   3 — tab bar (2 rows, spec §4.5)
///   4 — two-pane body (cockpit | chat), shrunk by 1 to make room for status
///   5 — status line (1 row, spec §4.8)
pub fn draw_app(f: &mut Frame<'_>, state: &AppState) {
    let total = f.area();
    let pipeline_height: u16 = if state.pipeline.is_empty() { 0 } else { 5 };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),               // row 0: title bar
            Constraint::Length(1),               // row 1: issue header
            Constraint::Length(pipeline_height), // row 2: pipeline bar (0 or 5)
            Constraint::Length(2),               // row 3: tab bar
            Constraint::Min(0),                  // row 4: body panes
            Constraint::Length(1),               // row 5: status line
        ])
        .split(total);

    views::title_bar::render(rows[0], f);
    views::issue_header::render(rows[1], f, state);
    views::pipeline_bar::render(rows[2], f, state);
    views::tab_bar::render(rows[3], f, state);

    let body_area = rows[4];
    match state.focus {
        Pane::Logs => views::logs_pane::render(body_area, f, state),
        Pane::Chat => views::chat_pane::render(body_area, f, state),
        Pane::Issue => views::issue_pane::render(body_area, f, state),
    }

    views::status_line::render(rows[5], f, state);

    // Help overlay renders LAST so it sits on top of all other widgets.
    views::help_overlay::render(f.area(), f, state);
}
