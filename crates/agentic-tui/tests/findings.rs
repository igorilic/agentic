//! Step 12.5: findings table with keyboard triage (`j`/`k` to move,
//! `f`/`t`/`i` to triage). Tests cover the state machine, the bus
//! ingestion path (`Event::Finding`), and that the rendered cockpit
//! buffer shows the cursor + status badges.

use agentic_core::events::{Event, EventEnvelope, Severity};
use agentic_tui::app::{AppState, Pane};
use agentic_tui::draw_app;
use agentic_tui::findings::{Finding, Triage};
use agentic_tui::modes::Mode;
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

mod common;

fn finding(id: &str, message: &str) -> Finding {
    Finding {
        id: id.to_string(),
        severity: Severity::Warning,
        file: Some("src/main.rs".to_string()),
        line: Some(42),
        message: message.to_string(),
        triage: None,
    }
}

fn finding_envelope(id: &str, message: &str) -> EventEnvelope {
    EventEnvelope {
        schema_version: 1,
        event_id: format!("evt-{id}"),
        run_id: "run1".to_string(),
        step_id: Some("run1-step-reviewer".to_string()),
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: id.to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: message.to_string(),
            suggestion: None,
        },
    }
}

// ─── default state ──────────────────────────────────────────────────────────

#[test]
fn default_findings_state_is_empty_with_cursor_at_zero() {
    let s = AppState::default();
    assert!(s.findings.items.is_empty());
    assert_eq!(s.findings.cursor, 0);
}

// ─── cursor movement ────────────────────────────────────────────────────────

#[test]
fn pressing_j_moves_cursor_down() {
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha"), finding("b", "beta")];
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.findings.cursor, 1);
}

#[test]
fn pressing_k_moves_cursor_up() {
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha"), finding("b", "beta")];
    s.findings.cursor = 1;
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.findings.cursor, 0);
}

#[test]
fn cursor_down_saturates_at_last_index() {
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha"), finding("b", "beta")];
    s.findings.cursor = 1;
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.findings.cursor, 1, "j must clamp at last index, not wrap");
}

#[test]
fn cursor_up_saturates_at_zero() {
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha"), finding("b", "beta")];
    s.findings.cursor = 0;
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.findings.cursor, 0, "k must clamp at 0, not wrap");
}

#[test]
fn cursor_movement_on_empty_list_is_a_noop() {
    let mut s = AppState::default();
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.findings.cursor, 0);
}

// ─── triage keys ────────────────────────────────────────────────────────────

#[test]
fn pressing_t_triages_selected_row_as_tech_debt() {
    // The exact scenario from todo.md §12.5: jj + t → row 3 = tech-debt.
    let mut s = AppState::default();
    s.findings.items = vec![
        finding("a", "alpha"),
        finding("b", "beta"),
        finding("c", "gamma"),
    ];
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('t'));
    assert_eq!(s.findings.items[2].triage, Some(Triage::TechDebt));
    assert_eq!(s.findings.items[0].triage, None);
    assert_eq!(s.findings.items[1].triage, None);
}

#[test]
fn pressing_f_triages_selected_row_as_fix() {
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha")];
    s.handle_key(KeyCode::Char('f'));
    assert_eq!(s.findings.items[0].triage, Some(Triage::Fix));
}

#[test]
fn pressing_i_triages_selected_row_as_ignore() {
    // T.13.6: 'i' triages as Ignore only in Issue pane (not Logs/Chat).
    let mut s = AppState {
        focus: Pane::Issue,
        ..Default::default()
    };
    s.findings.items = vec![finding("a", "alpha")];
    s.handle_key(KeyCode::Char('i'));
    assert_eq!(s.findings.items[0].triage, Some(Triage::Ignore));
}

#[test]
fn triage_keys_on_empty_list_are_noop() {
    // T.13.6: after pane-scoping, 'i' in Logs enters Insert mode, so we
    // test all three triage keys from Pane::Issue where 'i' is the triage
    // action.  With an empty list all three must be pure no-ops — no triage
    // applied and mode stays Normal.
    let mut s = AppState {
        focus: Pane::Issue,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('f'));
    assert_eq!(s.mode, Mode::Normal, "f on empty list must not change mode");
    s.handle_key(KeyCode::Char('t'));
    assert_eq!(s.mode, Mode::Normal, "t on empty list must not change mode");
    s.handle_key(KeyCode::Char('i'));
    assert_eq!(
        s.mode,
        Mode::Normal,
        "i on empty list in Issue pane must not change mode"
    );
    assert!(s.findings.items.is_empty());
}

#[test]
fn re_triaging_a_row_overrides_the_previous_value() {
    // T.13.6: 'i' triages as Ignore only in Issue pane.
    let mut s = AppState {
        focus: Pane::Issue,
        ..Default::default()
    };
    s.findings.items = vec![finding("a", "alpha")];
    s.handle_key(KeyCode::Char('f'));
    s.handle_key(KeyCode::Char('i'));
    assert_eq!(s.findings.items[0].triage, Some(Triage::Ignore));
}

// ─── bus ingestion ──────────────────────────────────────────────────────────

#[test]
fn event_finding_appends_to_the_findings_list() {
    let mut s = AppState::default();
    s.apply_envelope(&finding_envelope("f1", "first"));
    s.apply_envelope(&finding_envelope("f2", "second"));
    assert_eq!(s.findings.items.len(), 2);
    assert_eq!(s.findings.items[0].id, "f1");
    assert_eq!(s.findings.items[0].message, "first");
    assert_eq!(s.findings.items[1].id, "f2");
}

#[test]
fn re_ingesting_same_finding_id_does_not_duplicate_the_row() {
    // The bus can re-emit the same Event::Finding on replay (e.g.
    // history-buffer flush after resume). Ingest must dedupe so the
    // user doesn't have to triage the same issue twice.
    let mut s = AppState::default();
    s.apply_envelope(&finding_envelope("f1", "first"));
    s.apply_envelope(&finding_envelope("f1", "first")); // same id
    assert_eq!(s.findings.items.len(), 1);
}

// ─── command-mode interaction ───────────────────────────────────────────────

#[test]
fn triage_keys_in_command_mode_are_treated_as_text_not_actions() {
    // While the user is typing `:plan f` (e.g. a ticket reference like
    // "fix login"), the `f` must NOT silently triage a finding.
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha")];
    s.handle_key(KeyCode::Char(':'));
    s.handle_key(KeyCode::Char('f'));
    assert_eq!(
        s.findings.items[0].triage, None,
        "f in cmd-mode must not triage"
    );
}

// ─── render ─────────────────────────────────────────────────────────────────

#[test]
fn cockpit_renders_finding_message_when_present() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "missing-error-handling")];
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = common::flatten(&terminal);
    assert!(
        content.contains("missing-error-handling"),
        "expected finding message in cockpit; got: {content:?}"
    );
}

#[test]
fn cockpit_renders_triage_label_after_triaging() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "missing-error-handling")];
    s.handle_key(KeyCode::Char('t'));
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = common::flatten(&terminal);
    assert!(
        content.contains("tech-debt"),
        "expected 'tech-debt' badge in cockpit; got: {content:?}"
    );
}

#[test]
fn cockpit_marks_selected_finding_row_with_a_cursor_glyph() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default();
    s.findings.items = vec![finding("a", "alpha-msg"), finding("b", "beta-msg")];
    s.handle_key(KeyCode::Char('j')); // cursor → 1 (beta-msg row)
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = common::flatten(&terminal);
    // The `>` glyph marks the selected row in `views/findings.rs`.
    // The selected row (beta-msg) must carry the `>` prefix; the
    // unselected row (alpha-msg) must NOT.
    assert!(
        content.contains("> ⚠ beta-msg"),
        "selected row must show '> ⚠ beta-msg'; got: {content:?}"
    );
    assert!(
        !content.contains("> ⚠ alpha-msg"),
        "unselected row must NOT show '> ⚠ alpha-msg'; got: {content:?}"
    );
}
