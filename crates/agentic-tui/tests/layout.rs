//! Step 12.2: layout, focus, resize. Tests run against `TestBackend` so
//! they don't need a real terminal.

use agentic_tui::app::{AppEvent, AppState, Pane};
use agentic_tui::draw_app;
use agentic_tui::layout::compute_panes;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;

// ─── pure layout maths ───────────────────────────────────────────────────────

#[test]
fn default_state_splits_50_50_horizontally() {
    let s = AppState::default();
    let area = Rect::new(0, 0, 100, 20);
    let (cockpit, chat) = compute_panes(area, &s);
    assert_eq!(cockpit.x, 0);
    assert_eq!(cockpit.width, 50);
    assert_eq!(chat.x, 50);
    assert_eq!(chat.width, 50);
    assert_eq!(cockpit.height, 20);
    assert_eq!(chat.height, 20);
}

#[test]
fn pressing_close_bracket_widens_cockpit_to_60_percent() {
    let mut s = AppState::default();
    s.handle(AppEvent::WidenCockpit);
    let area = Rect::new(0, 0, 100, 20);
    let (cockpit, chat) = compute_panes(area, &s);
    assert_eq!(cockpit.width, 60);
    assert_eq!(chat.width, 40);
}

#[test]
fn pressing_open_bracket_narrows_cockpit_to_40_percent() {
    let mut s = AppState::default();
    s.handle(AppEvent::NarrowCockpit);
    let area = Rect::new(0, 0, 100, 20);
    let (cockpit, _chat) = compute_panes(area, &s);
    assert_eq!(cockpit.width, 40);
}

#[test]
fn cockpit_ratio_clamps_between_20_and_80_percent() {
    let mut s = AppState::default();
    // Push way past the upper bound.
    for _ in 0..20 {
        s.handle(AppEvent::WidenCockpit);
    }
    let area = Rect::new(0, 0, 100, 20);
    let (cockpit, _) = compute_panes(area, &s);
    assert!(
        (78..=82).contains(&cockpit.width),
        "cockpit clamped near 80%, got {}",
        cockpit.width
    );

    // And the other way.
    let mut s = AppState::default();
    for _ in 0..20 {
        s.handle(AppEvent::NarrowCockpit);
    }
    let (cockpit, _) = compute_panes(area, &s);
    assert!(
        (18..=22).contains(&cockpit.width),
        "cockpit clamped near 20%, got {}",
        cockpit.width
    );
}

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

// ─── render integration — pane titles appear in the buffer ──────────────────
// NOTE(T.12.2): `views::chat` (bordered Block with "Chat" title) has been
// replaced by `views::chat_pane` (borderless message blocks per spec §4.6).
// The "Chat" border title and "Chat *" focus indicator are no longer rendered.
// Focus state is still tracked in AppState (tab_cycles test above covers that).
// These render tests are updated to just verify no panic.

#[test]
fn first_frame_renders_chat_pane_title() {
    // The old chat.rs Block had a "Chat" border title; chat_pane is borderless.
    // T.12.2 removed that border. Assert: render does not panic.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}

#[test]
fn focus_indicator_renders_in_chat_pane_title_when_focused() {
    // The old chat.rs Block had a "Chat *" focus indicator. chat_pane is
    // borderless. T.12.2 removed that indicator. Assert: render does not panic
    // in either focus state.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default(); // focus = Logs
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic

    s.handle(AppEvent::ToggleFocus); // focus = Chat
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}
