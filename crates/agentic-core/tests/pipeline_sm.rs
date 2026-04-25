use agentic_core::{
    BackendId, Event, ModelId, PipelineConfig, PipelineSm, ProfileId, RunStatus, SmInput,
    StepStatus, TicketKind, TicketRef,
};
use proptest::prelude::*;

fn sample_ticket() -> TicketRef {
    TicketRef {
        kind: TicketKind::GithubIssue,
        reference: "#42".to_string(),
        title: Some("test".to_string()),
    }
}

fn start_input() -> SmInput {
    SmInput::Start {
        ticket: sample_ticket(),
        profile: ProfileId("github".into()),
        backend: BackendId("claude-code".into()),
        model: ModelId("claude-opus-4-7".into()),
    }
}

fn default_sm() -> PipelineSm {
    let config = PipelineConfig::builtin_default();
    let pipeline = config.default_pipeline().clone();
    PipelineSm::new("run-1".to_string(), pipeline)
}

#[test]
fn happy_path_pending_to_completed() {
    let mut sm = default_sm();
    assert_eq!(sm.state(), RunStatus::Pending);

    let events = sm.handle(start_input()).expect("start");
    assert_eq!(sm.state(), RunStatus::Running);
    // Expect RunStarted + StepStarted(architect)
    assert!(events.iter().any(|e| matches!(e, Event::RunStarted { .. })));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StepStarted { agent, .. } if agent == "architect"))
    );

    // Advance through: architect → tdd-developer → qa → reviewer
    for expected_next_agent in &["tdd-developer", "qa", "reviewer"] {
        let events = sm.handle(SmInput::StepPassed).expect("pass");
        assert!(
            events.iter().any(
                |e| matches!(e, Event::StepStarted { agent, .. } if agent == expected_next_agent)
            ),
            "expected StepStarted for {expected_next_agent}"
        );
    }

    // Reviewer passes — run completes
    let events = sm.handle(SmInput::StepPassed).expect("pass reviewer");
    assert_eq!(sm.state(), RunStatus::Completed);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::RunComplete {
            status: RunStatus::Completed,
            ..
        }
    )));
}

#[test]
fn qa_fails_three_times_then_tech_debt_and_reviewer_completes_with_tech_debt() {
    let mut sm = default_sm();
    sm.handle(start_input()).expect("start");
    sm.handle(SmInput::StepPassed).expect("architect passed"); // → tdd-developer
    sm.handle(SmInput::StepPassed)
        .expect("tdd-developer passed 1st time"); // → qa

    // QA fails 3 times, bouncing back to tdd-developer each time.
    for retry in 1..=3 {
        let events = sm.handle(SmInput::StepFailed).expect("qa failed");
        assert!(
            events.iter().any(
                |e| matches!(e, Event::RetryStarted { attempt, .. } if *attempt == retry as u32)
            ),
            "expected RetryStarted(attempt={retry})"
        );
        // Now current step is tdd-developer again
        sm.handle(SmInput::StepPassed)
            .expect("tdd-developer retry passed"); // → qa
    }

    // 4th qa failure: moves to tech-debt, advances to reviewer
    sm.handle(SmInput::StepFailed).expect("qa failed 4th");
    assert_eq!(sm.state(), RunStatus::Running); // still running reviewer
    // Reviewer passes → CompletedWithTechDebt
    let events = sm.handle(SmInput::StepPassed).expect("reviewer passed");
    assert_eq!(sm.state(), RunStatus::CompletedWithTechDebt);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::RunComplete {
            status: RunStatus::CompletedWithTechDebt,
            ..
        }
    )));
}

#[test]
fn cancel_during_any_running_step_yields_cancelled() {
    let mut sm = default_sm();
    sm.handle(start_input()).expect("start");
    let events = sm.handle(SmInput::Cancel).expect("cancel");
    assert_eq!(sm.state(), RunStatus::Cancelled);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::RunComplete {
            status: RunStatus::Cancelled,
            ..
        }
    )));

    // Subsequent inputs must error
    let result = sm.handle(SmInput::StepPassed);
    assert!(result.is_err(), "terminal state must reject further input");
}

// #16 — Crash input transitions to Crashed and emits RunComplete{Crashed}
#[test]
fn crash_during_running_yields_crashed() {
    let mut sm = default_sm();
    sm.handle(start_input()).expect("start");
    assert_eq!(sm.state(), RunStatus::Running);

    let events = sm.handle(SmInput::Crash).expect("crash");
    assert_eq!(sm.state(), RunStatus::Crashed);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::RunComplete {
                status: RunStatus::Crashed,
                ..
            }
        )),
        "expected RunComplete{{Crashed}} event"
    );
    // Terminal: further input must error.
    assert!(sm.handle(SmInput::StepPassed).is_err());
}

// #16 — StepSkipped advances to next step without failing
#[test]
fn step_skipped_advances_to_next_step() {
    let mut sm = default_sm();
    sm.handle(start_input()).expect("start"); // architect running

    let events = sm.handle(SmInput::StepSkipped).expect("skip architect");
    // Should advance to tdd-developer
    assert_eq!(sm.state(), RunStatus::Running);
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::StepComplete {
                status: StepStatus::Skipped,
                ..
            }
        )),
        "expected StepComplete{{Skipped}}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StepStarted { agent, .. } if agent == "tdd-developer")),
        "expected StepStarted for tdd-developer after skip"
    );
}

// #16 — non-qa step with stop_on_failure=true fails → RunStatus::Failed
#[test]
fn non_qa_step_stop_on_failure_yields_failed() {
    // In the default pipeline, architect has stop_on_failure=true.
    let mut sm = default_sm();
    sm.handle(start_input()).expect("start"); // architect running

    let events = sm.handle(SmInput::StepFailed).expect("architect failed");
    assert_eq!(
        sm.state(),
        RunStatus::Failed,
        "stop_on_failure=true step failure should set run to Failed"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::RunComplete {
                status: RunStatus::Failed,
                ..
            }
        )),
        "expected RunComplete{{Failed}}"
    );
}

proptest! {
    #[test]
    fn sm_invariants_hold_over_random_input_sequences(
        inputs in proptest::collection::vec(arb_sm_input(), 0..30)
    ) {
        let mut sm = default_sm();
        let mut terminal_reached = false;

        for input in inputs {
            let prior_state = sm.state();
            let prior_sm = sm.clone();
            let result = sm.handle(input);

            if result.is_err() {
                // F5a: Err must not mutate state.
                prop_assert_eq!(
                    sm.clone(),
                    prior_sm,
                    "handle() returned Err but SM state changed"
                );
            }

            // Invariant 1: once terminal, all subsequent handle() calls error.
            if terminal_reached {
                prop_assert!(result.is_err(), "terminal state must reject further input");
            }

            // Invariant 2: state == Running ⟺ exactly one step is Running.
            let running_count = sm
                .step_statuses()
                .iter()
                .filter(|s| **s == StepStatus::Running)
                .count();
            if sm.state() == RunStatus::Running {
                prop_assert!(
                    running_count == 1,
                    "Running run must have exactly 1 running step, had {}; prior state: {:?}",
                    running_count,
                    prior_state
                );
            } else {
                prop_assert!(
                    running_count == 0,
                    "Non-running run must have 0 running steps, had {}; state: {:?}",
                    running_count,
                    sm.state()
                );
            }

            if is_terminal_pub(sm.state()) {
                terminal_reached = true;
            }
        }
    }
}

// Helper for proptest (public clone of sm.rs's private is_terminal)
fn is_terminal_pub(s: RunStatus) -> bool {
    matches!(
        s,
        RunStatus::Completed
            | RunStatus::CompletedWithTechDebt
            | RunStatus::Failed
            | RunStatus::Cancelled
            | RunStatus::Crashed
    )
}

fn arb_sm_input() -> impl Strategy<Value = SmInput> {
    prop_oneof![
        Just(start_input()),
        Just(SmInput::StepPassed),
        Just(SmInput::StepFailed),
        Just(SmInput::StepNeedsTriage),
        Just(SmInput::StepSkipped),
        Just(SmInput::Cancel),
        Just(SmInput::Crash),
    ]
}
