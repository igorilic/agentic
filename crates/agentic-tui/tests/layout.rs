//! T.12.3: Layout tests — updated to remove the now-deleted compute_panes /
//! cockpit_ratio / WidenCockpit / NarrowCockpit surface (single-pane body
//! restructure). The resize-key tests are deleted. The focus cycle and render
//! smoke tests are kept.

use agentic_tui::app::{AppEvent, AppState, Pane};
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ─── focus state machine ─────────────────────────────────────────────────────

#[test]
fn tab_cycles_focus_through_logs_chat_issue() {
    let mut s = AppState::default();
    assert_eq!(s.focus, Pane::Logs);
    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Chat);
    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Issue);
    s.handle(AppEvent::ToggleFocus);
    assert_eq!(s.focus, Pane::Logs);
}

// ─── render smoke tests ───────────────────────────────────────────────────────

#[test]
fn first_frame_renders_without_panic() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}

#[test]
fn render_in_each_focus_does_not_panic() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default(); // focus = Logs
    terminal.draw(|f| draw_app(f, &s)).unwrap();

    s.handle(AppEvent::ToggleFocus); // focus = Chat
    terminal.draw(|f| draw_app(f, &s)).unwrap();

    s.handle(AppEvent::ToggleFocus); // focus = Issue
    terminal.draw(|f| draw_app(f, &s)).unwrap();
}
