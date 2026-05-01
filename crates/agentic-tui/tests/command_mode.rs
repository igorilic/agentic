//! Step 12.4: command mode (`:plan`, `:status`, `:q`).
//!
//! `:` enters command mode; typed characters build a buffer; Enter
//! parses the buffer into an `AppCommand`; Esc cancels.

use agentic_core::events::{Event, EventEnvelope, StepStatus, TokenUsage};
use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::modes::{AppCommand, Mode};
use agentic_tui::run::{CANONICAL_AGENTS, StepRunStatus};
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn flatten(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn type_str(state: &mut AppState, s: &str) -> Option<AppCommand> {
    let mut last = None;
    for ch in s.chars() {
        last = state.handle_key(KeyCode::Char(ch));
    }
    last
}

// ─── mode transitions ───────────────────────────────────────────────────────

#[test]
fn default_mode_is_normal() {
    let s = AppState::default();
    assert_eq!(s.mode, Mode::Normal);
}

#[test]
fn typing_colon_in_normal_mode_enters_command_mode_with_empty_buffer() {
    let mut s = AppState::default();
    let cmd = s.handle_key(KeyCode::Char(':'));
    assert_eq!(cmd, None);
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: String::new()
        }
    );
}

#[test]
fn typing_chars_in_command_mode_appends_to_buffer() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan hello");
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: "plan hello".to_string()
        }
    );
}

#[test]
fn backspace_removes_last_char_from_buffer() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan");
    s.handle_key(KeyCode::Backspace);
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: "pla".to_string()
        }
    );
}

#[test]
fn backspace_with_empty_buffer_stays_in_command_mode() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    s.handle_key(KeyCode::Backspace);
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: String::new()
        }
    );
}

#[test]
fn esc_in_command_mode_cancels_back_to_normal() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan oops");
    let cmd = s.handle_key(KeyCode::Esc);
    assert_eq!(cmd, None);
    assert_eq!(s.mode, Mode::Normal);
}

// ─── parse on Enter ─────────────────────────────────────────────────────────

#[test]
fn enter_plan_with_ticket_returns_appcommand_plan_and_normal_mode() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan hello");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(
        cmd,
        Some(AppCommand::Plan {
            ticket: "hello".to_string()
        })
    );
    assert_eq!(s.mode, Mode::Normal);
}

#[test]
fn enter_plan_with_multiword_ticket_joins_with_spaces() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan add rate limiting to /chat");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(
        cmd,
        Some(AppCommand::Plan {
            ticket: "add rate limiting to /chat".to_string()
        }),
    );
}

#[test]
fn enter_plan_with_no_ticket_returns_none_and_exits_command_mode() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(cmd, None);
    assert_eq!(s.mode, Mode::Normal);
}

#[test]
fn enter_q_returns_appcommand_quit() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    s.handle_key(KeyCode::Char('q'));
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(cmd, Some(AppCommand::Quit));
    assert_eq!(s.mode, Mode::Normal);
}

#[test]
fn enter_status_returns_appcommand_status() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "status");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(cmd, Some(AppCommand::Status));
}

#[test]
fn enter_unknown_command_returns_none_and_exits_command_mode() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "bogus");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(cmd, None);
    assert_eq!(s.mode, Mode::Normal);
}

// ─── normal-mode keys keep their meaning ────────────────────────────────────

#[test]
fn tab_in_normal_mode_toggles_focus() {
    use agentic_tui::app::Pane;
    let mut s = AppState::default();
    assert_eq!(s.focus, Pane::Logs);
    s.handle_key(KeyCode::Tab);
    assert_eq!(s.focus, Pane::Chat);
}

#[test]
fn brackets_in_normal_mode_resize_cockpit() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(']'));
    assert!((s.cockpit_ratio - 0.60).abs() < f32::EPSILON);
    s.handle_key(KeyCode::Char('['));
    assert!((s.cockpit_ratio - 0.50).abs() < f32::EPSILON);
}

#[test]
fn brackets_in_command_mode_are_appended_to_buffer() {
    // In command mode, `[` and `]` are just text — not resize keys.
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan [a]b");
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: "plan [a]b".to_string()
        }
    );
    // Critically, ratio must NOT have changed.
    assert!((s.cockpit_ratio - 0.50).abs() < f32::EPSILON);
}

#[test]
fn tab_in_command_mode_is_a_noop() {
    use agentic_tui::app::Pane;
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    s.handle_key(KeyCode::Tab);
    // Focus must NOT have toggled.
    assert_eq!(s.focus, Pane::Logs);
    // Tab must NOT have appended to the buffer.
    assert_eq!(
        s.mode,
        Mode::Command {
            buffer: String::new()
        }
    );
}

// ─── render ─────────────────────────────────────────────────────────────────

#[test]
fn command_mode_renders_prompt_line_with_buffer() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan hi");
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    assert!(
        content.contains(":plan hi"),
        "expected ':plan hi' prompt in frame; got: {content:?}"
    );
}

#[test]
fn normal_mode_does_not_render_a_command_prompt() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    // The cursor glyph `█` is unique to the command-mode prompt; the
    // hint line uses ` · ` separators only.
    assert!(
        !content.contains('█'),
        "no command-mode cursor expected in normal mode; got: {content:?}"
    );
}

// ─── status / hint line ─────────────────────────────────────────────────────

#[test]
fn default_last_status_is_none() {
    let s = AppState::default();
    assert_eq!(s.last_status, None);
}

#[test]
fn enter_unknown_command_sets_last_status() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "bogus");
    s.handle_key(KeyCode::Enter);
    let status = s.last_status.expect("expected last_status set");
    assert!(
        status.to_lowercase().contains("bogus") || status.to_lowercase().contains("unknown"),
        "expected error to mention 'bogus' or 'unknown'; got: {status:?}"
    );
}

#[test]
fn enter_plan_with_no_ticket_sets_last_status() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan");
    s.handle_key(KeyCode::Enter);
    let status = s.last_status.expect("expected last_status set");
    assert!(
        status.to_lowercase().contains("ticket") || status.to_lowercase().contains("plan"),
        "expected error to mention ticket/plan; got: {status:?}"
    );
}

#[test]
fn successful_command_clears_last_status() {
    let mut s = AppState {
        last_status: Some("stale error".to_string()),
        ..Default::default()
    };
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "status");
    s.handle_key(KeyCode::Enter);
    assert_eq!(s.last_status, None);
}

#[test]
fn entering_command_mode_does_not_immediately_clear_last_status() {
    // The user typing `:` to retry should still see the previous error
    // until they execute a new command — they may need to refer to it.
    let mut s = AppState {
        last_status: Some("some error".to_string()),
        ..Default::default()
    };
    s.handle_key(KeyCode::Char(':'));
    assert_eq!(s.last_status, Some("some error".to_string()));
}

#[test]
fn normal_mode_renders_hint_line() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    // Hint text mentions some keys.
    assert!(
        content.contains("j/k") || content.contains("triage") || content.contains("commands"),
        "expected hint text in chat pane footer; got: {content:?}"
    );
}

#[test]
fn normal_mode_with_last_status_renders_status_instead_of_hint() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState {
        last_status: Some("Unknown command: bogus".to_string()),
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    assert!(
        content.contains("Unknown command: bogus"),
        "expected status text in chat pane footer; got: {content:?}"
    );
}

// ─── integration: command → simulated scripted events → cockpit transitions ─

#[test]
fn plan_command_followed_by_simulated_bus_events_drives_cockpit() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char(':'));
    type_str(&mut s, "plan hello");
    let cmd = s.handle_key(KeyCode::Enter);
    assert_eq!(
        cmd,
        Some(AppCommand::Plan {
            ticket: "hello".to_string()
        })
    );

    // The binary's job is to spawn a backend and forward bus envelopes.
    // In tests we feed envelopes directly to assert the integration.
    for agent in CANONICAL_AGENTS {
        s.apply_envelope(&EventEnvelope {
            schema_version: 1,
            event_id: format!("e-{agent}-start"),
            run_id: "run1".to_string(),
            step_id: Some(format!("run1-step-{agent}")),
            timestamp_ms: 0,
            event: Event::StepStarted {
                agent: agent.to_string(),
                model: agentic_core::backends::ModelId("m".to_string()),
            },
        });
        s.apply_envelope(&EventEnvelope {
            schema_version: 1,
            event_id: format!("e-{agent}-done"),
            run_id: "run1".to_string(),
            step_id: Some(format!("run1-step-{agent}")),
            timestamp_ms: 0,
            event: Event::StepComplete {
                status: StepStatus::Passed,
                summary: "ok".to_string(),
                token_usage: TokenUsage::default(),
                cost_usd: None,
                duration_ms: 1,
            },
        });
    }
    for row in &s.run.steps {
        assert_eq!(row.status, StepRunStatus::Passed);
    }
}
