//! Pure layout maths for the cockpit + chat panes.
//!
//! Separated from rendering so tests can assert on rect geometry without
//! pulling in the TestBackend round-trip.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::AppState;

/// Compute the cockpit and chat rects given the available area and the
/// current ratio. Cockpit is always on the left.
pub fn compute_panes(area: Rect, state: &AppState) -> (Rect, Rect) {
    // ratatui's percentage Constraint takes a u16, so quantise here.
    let cockpit_pct = (state.cockpit_ratio * 100.0).round().clamp(0.0, 100.0) as u16;
    let chat_pct = 100 - cockpit_pct;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(cockpit_pct),
            Constraint::Percentage(chat_pct),
        ])
        .split(area);
    (chunks[0], chunks[1])
}
