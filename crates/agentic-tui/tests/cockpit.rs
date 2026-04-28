//! Step 12.3: cockpit pane renders a four-row stepper that mirrors the
//! Tauri `Stepper.tsx` component but in ratatui. The state transitions
//! are driven by `EventEnvelope`s (the same ones the bus broadcasts),
//! so the tests don't need a real bus — they just call `apply_envelope`.

use agentic_core::events::{Event, EventEnvelope, StepStatus, TokenUsage};
use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::run::{CANONICAL_AGENTS, StepRunStatus};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// Build an envelope using the same `step_id` format the real
/// orchestrator emits — `<run>-step-<seq>-<agent>` — so the tests
/// exercise the production parsing path, not just `ends_with`.
fn envelope_for(agent: &str, event: Event) -> EventEnvelope {
    let seq = canonical_seq(agent);
    EventEnvelope {
        schema_version: 1,
        event_id: format!("evt-{agent}"),
        run_id: "run1".to_string(),
        step_id: Some(format!("run1-step-{seq}-{agent}")),
        timestamp_ms: 0,
        event,
    }
}

fn canonical_seq(agent: &str) -> usize {
    CANONICAL_AGENTS
        .iter()
        .position(|a| *a == agent)
        .unwrap_or(0)
}

fn flatten(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

// ─── pure run-state state machine ───────────────────────────────────────────

#[test]
fn default_run_state_has_four_pending_steps_in_canonical_order() {
    let s = AppState::default();
    let agents: Vec<&str> = s.run.steps.iter().map(|r| r.agent.as_str()).collect();
    assert_eq!(agents, CANONICAL_AGENTS);
    for row in &s.run.steps {
        assert_eq!(row.status, StepRunStatus::Pending);
    }
}

#[test]
fn step_started_for_architect_marks_first_row_running() {
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "architect",
        Event::StepStarted {
            agent: "architect".to_string(),
            model: agentic_core::backends::ModelId("claude-sonnet-4-6".to_string()),
        },
    ));
    assert_eq!(s.run.steps[0].status, StepRunStatus::Running);
    // The other rows must still be pending.
    assert_eq!(s.run.steps[1].status, StepRunStatus::Pending);
    assert_eq!(s.run.steps[2].status, StepRunStatus::Pending);
    assert_eq!(s.run.steps[3].status, StepRunStatus::Pending);
}

#[test]
fn step_complete_passed_for_architect_marks_row_passed() {
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "architect",
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 100,
        },
    ));
    assert_eq!(s.run.steps[0].status, StepRunStatus::Passed);
}

#[test]
fn full_pipeline_run_drives_all_four_rows_to_passed() {
    let mut s = AppState::default();
    for agent in CANONICAL_AGENTS {
        s.apply_envelope(&envelope_for(
            agent,
            Event::StepStarted {
                agent: agent.to_string(),
                model: agentic_core::backends::ModelId("m".to_string()),
            },
        ));
        s.apply_envelope(&envelope_for(
            agent,
            Event::StepComplete {
                status: StepStatus::Passed,
                summary: "done".to_string(),
                token_usage: TokenUsage::default(),
                cost_usd: None,
                duration_ms: 1,
            },
        ));
    }
    for row in &s.run.steps {
        assert_eq!(row.status, StepRunStatus::Passed);
    }
}

#[test]
fn step_complete_failed_for_qa_marks_row_failed() {
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "qa",
        Event::StepComplete {
            status: StepStatus::Failed,
            summary: "tests failed".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 1,
        },
    ));
    assert_eq!(s.run.steps[2].status, StepRunStatus::Failed);
}

#[test]
fn step_complete_needs_triage_for_reviewer_marks_row_needs_triage() {
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "reviewer",
        Event::StepComplete {
            status: StepStatus::NeedsTriage,
            summary: "review pending".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 1,
        },
    ));
    assert_eq!(s.run.steps[3].status, StepRunStatus::NeedsTriage);
}

#[test]
fn unknown_agent_in_event_does_not_panic_or_mutate_state() {
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "ghost-agent",
        Event::StepStarted {
            agent: "ghost-agent".to_string(),
            model: agentic_core::backends::ModelId("m".to_string()),
        },
    ));
    for row in &s.run.steps {
        assert_eq!(row.status, StepRunStatus::Pending);
    }
}

#[test]
fn step_complete_without_step_id_falls_back_to_running_row() {
    // Real bus envelopes always carry a step_id, but a replayed or
    // history-buffered event might not. The state machine should still
    // mark the step that is currently Running as Passed.
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "tdd-developer",
        Event::StepStarted {
            agent: "tdd-developer".to_string(),
            model: agentic_core::backends::ModelId("m".to_string()),
        },
    ));
    assert_eq!(s.run.steps[1].status, StepRunStatus::Running);

    s.apply_envelope(&EventEnvelope {
        schema_version: 1,
        event_id: "evt-no-stepid".to_string(),
        run_id: "run1".to_string(),
        step_id: None, // ← the fallback path
        timestamp_ms: 0,
        event: Event::StepComplete {
            status: StepStatus::Passed,
            summary: "done".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 1,
        },
    });
    assert_eq!(s.run.steps[1].status, StepRunStatus::Passed);
}

#[test]
fn step_complete_routes_tdd_developer_via_real_orchestrator_step_id_format() {
    // The orchestrator emits step_id="<run>-step-<seq>-<agent>".
    // tdd-developer's hyphen makes the parse easy to break — pin it.
    let mut s = AppState::default();
    s.apply_envelope(&EventEnvelope {
        schema_version: 1,
        event_id: "evt-tdd".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("run1-step-1-tdd-developer".to_string()),
        timestamp_ms: 0,
        event: Event::StepComplete {
            status: StepStatus::Passed,
            summary: "done".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 1,
        },
    });
    assert_eq!(s.run.steps[1].status, StepRunStatus::Passed);
    // Must NOT bleed onto neighbouring rows — `architect` (which is a
    // strict prefix of nothing here, but worth pinning).
    assert_eq!(s.run.steps[0].status, StepRunStatus::Pending);
}

#[test]
fn non_step_events_are_ignored_by_run_state() {
    // TextDelta should not change any row's status.
    let mut s = AppState::default();
    s.apply_envelope(&envelope_for(
        "architect",
        Event::TextDelta {
            content: "thinking…".to_string(),
        },
    ));
    for row in &s.run.steps {
        assert_eq!(row.status, StepRunStatus::Pending);
    }
}

// ─── render integration ─────────────────────────────────────────────────────

#[test]
fn cockpit_pane_renders_all_four_agent_names() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    for agent in CANONICAL_AGENTS {
        assert!(
            content.contains(agent),
            "cockpit must render agent '{agent}'; got: {content:?}"
        );
    }
}

#[test]
fn cockpit_pane_renders_pending_icon_for_each_row_by_default() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    // The pending icon (○) should appear at least four times — once per row.
    let pending_count = content.matches('○').count();
    assert!(
        pending_count >= 4,
        "expected ≥4 pending icons, got {pending_count}; content: {content:?}"
    );
}

#[test]
fn cockpit_pane_renders_running_and_passed_icons_after_apply() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut s = AppState::default();
    // architect → passed; tdd-developer → running.
    s.apply_envelope(&envelope_for(
        "architect",
        Event::StepComplete {
            status: StepStatus::Passed,
            summary: "ok".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 1,
        },
    ));
    s.apply_envelope(&envelope_for(
        "tdd-developer",
        Event::StepStarted {
            agent: "tdd-developer".to_string(),
            model: agentic_core::backends::ModelId("m".to_string()),
        },
    ));
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    assert!(content.contains('✓'), "expected ✓ icon; got: {content:?}");
    assert!(content.contains('◐'), "expected ◐ icon; got: {content:?}");
}
