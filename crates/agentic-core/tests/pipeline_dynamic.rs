/// Tests for the dynamic pipeline: arbitrary agent lists, no qa-fix-loop.
///
/// These tests verify the new linear N-step contract introduced in I.3.
use agentic_core::{
    BackendId, Event, ModelId, Pipeline, PipelineConfig, PipelineSm, ProfileId, RunStatus,
    SmInput, StepStatus, TicketKind, TicketRef,
};

fn sample_ticket() -> TicketRef {
    TicketRef {
        kind: TicketKind::GithubIssue,
        reference: "#1".to_string(),
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

// Helper: build a Pipeline from agent names with stop_on_failure = true, default fields.
fn pipeline_from_agents(agents: &[&str]) -> Pipeline {
    Pipeline {
        steps: agents
            .iter()
            .map(|&name| agentic_core::PipelineStep {
                agent: name.to_string(),
                stop_on_failure: true,
                allowed_questions: None,
            })
            .collect(),
    }
}

// Helper for a step with stop_on_failure = false
fn pipeline_from_agents_no_stop(agents: &[&str]) -> Pipeline {
    Pipeline {
        steps: agents
            .iter()
            .map(|&name| agentic_core::PipelineStep {
                agent: name.to_string(),
                stop_on_failure: false,
                allowed_questions: None,
            })
            .collect(),
    }
}

// --- Test 1: 1-agent pipeline completes in one StepPassed cycle ---

#[test]
fn single_agent_pipeline_completes_after_one_step_passed() {
    let pipeline = pipeline_from_agents(&["reviewer"]);
    let mut sm = PipelineSm::new("run-1".to_string(), pipeline);
    assert_eq!(sm.state(), RunStatus::Pending);

    sm.handle(start_input()).expect("start");
    assert_eq!(sm.state(), RunStatus::Running);
    assert_eq!(sm.current_step_index(), Some(0));

    let events = sm.handle(SmInput::StepPassed).expect("step passed");
    assert_eq!(
        sm.state(),
        RunStatus::Completed,
        "1-agent pipeline must Complete after one StepPassed"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::RunComplete {
                status: RunStatus::Completed,
                ..
            }
        )),
        "expected RunComplete{{Completed}}"
    );
}

// --- Test 2: 5-agent pipeline with duplicate tdd-developer ---

#[test]
fn five_agent_pipeline_with_duplicate_name_advances_in_order() {
    let agents = ["architect", "tdd-developer", "qa", "tdd-developer", "reviewer"];
    let pipeline = pipeline_from_agents(&agents);
    let mut sm = PipelineSm::new("run-2".to_string(), pipeline);

    sm.handle(start_input()).expect("start");

    // Step 0 → 1 → 2 → 3 → 4, then Complete
    let expected_next = ["tdd-developer", "qa", "tdd-developer", "reviewer"];
    for &next_agent in &expected_next {
        let events = sm.handle(SmInput::StepPassed).expect("step passed");
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::StepStarted { agent, .. } if agent == next_agent)),
            "expected StepStarted for {next_agent}"
        );
        assert_eq!(sm.state(), RunStatus::Running);
    }

    // Pass the last step
    let events = sm.handle(SmInput::StepPassed).expect("last step passed");
    assert_eq!(
        sm.state(),
        RunStatus::Completed,
        "5-agent pipeline must Complete after 5 StepPassed calls"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::RunComplete {
                status: RunStatus::Completed,
                ..
            }
        ))
    );
}

// --- Test 3: qa failure with stop_on_failure: false advances to next step (no retry) ---

#[test]
fn qa_step_failure_with_stop_false_advances_to_reviewer_without_retry() {
    // Build: architect(stop=true), tdd-developer(stop=true), qa(stop=false), reviewer(stop=true)
    let steps = vec![
        agentic_core::PipelineStep {
            agent: "architect".to_string(),
            stop_on_failure: true,
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "tdd-developer".to_string(),
            stop_on_failure: true,
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "qa".to_string(),
            stop_on_failure: false,
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "reviewer".to_string(),
            stop_on_failure: false,
            allowed_questions: None,
        },
    ];
    let pipeline = Pipeline { steps };
    let mut sm = PipelineSm::new("run-3".to_string(), pipeline);

    sm.handle(start_input()).expect("start");
    sm.handle(SmInput::StepPassed).expect("architect passed");   // → tdd-developer
    sm.handle(SmInput::StepPassed).expect("tdd-developer passed"); // → qa

    // qa fails — must advance to reviewer, NOT roll back to tdd-developer
    let events = sm.handle(SmInput::StepFailed).expect("qa failed");
    assert_eq!(
        sm.state(),
        RunStatus::Running,
        "run must still be Running after qa fails with stop_on_failure=false"
    );
    // No RetryStarted event must be emitted (that was the old behavior)
    assert!(
        !events.iter().any(|e| matches!(e, Event::RetryStarted { .. })),
        "qa failure must NOT emit RetryStarted under the new contract"
    );
    // Must start reviewer next
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StepStarted { agent, .. } if agent == "reviewer")),
        "expected StepStarted for reviewer after qa failure"
    );
    // step_index must now be at reviewer (3), NOT rolled back to tdd-developer (1)
    assert_eq!(
        sm.current_step_index(),
        Some(3),
        "step_index must advance to reviewer (3), not roll back to tdd-developer (1)"
    );
}

// --- Test 4: qa failure with stop_on_failure: true terminates with Failed ---

#[test]
fn qa_step_failure_with_stop_true_terminates_with_failed() {
    let steps = vec![
        agentic_core::PipelineStep {
            agent: "architect".to_string(),
            stop_on_failure: true,
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "tdd-developer".to_string(),
            stop_on_failure: true,
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "qa".to_string(),
            stop_on_failure: true, // stop on failure this time
            allowed_questions: None,
        },
        agentic_core::PipelineStep {
            agent: "reviewer".to_string(),
            stop_on_failure: false,
            allowed_questions: None,
        },
    ];
    let pipeline = Pipeline { steps };
    let mut sm = PipelineSm::new("run-4".to_string(), pipeline);

    sm.handle(start_input()).expect("start");
    sm.handle(SmInput::StepPassed).expect("architect passed");
    sm.handle(SmInput::StepPassed).expect("tdd-developer passed");

    let events = sm.handle(SmInput::StepFailed).expect("qa failed");
    assert_eq!(
        sm.state(),
        RunStatus::Failed,
        "qa failure with stop_on_failure=true must terminate with Failed"
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

// --- Test 5: PipelineConfig::from_agents constructs a valid Pipeline ---

#[test]
fn from_agents_builds_pipeline_with_correct_steps() {
    let agents = vec!["architect".to_string(), "tdd-developer".to_string()];
    let pipeline = PipelineConfig::from_agents(&agents).expect("from_agents succeeds");
    assert_eq!(pipeline.steps.len(), 2);
    assert_eq!(pipeline.steps[0].agent, "architect");
    assert_eq!(pipeline.steps[1].agent, "tdd-developer");
}

#[test]
fn from_agents_single_agent_works() {
    let agents = vec!["only-one".to_string()];
    let pipeline = PipelineConfig::from_agents(&agents).expect("from_agents single");
    assert_eq!(pipeline.steps.len(), 1);
    assert_eq!(pipeline.steps[0].agent, "only-one");
}

#[test]
fn from_agents_empty_list_errors() {
    let result = PipelineConfig::from_agents(&[]);
    assert!(result.is_err(), "from_agents([]) must return an error");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.to_lowercase().contains("empty") || msg.to_lowercase().contains("at least"),
        "error message should mention empty list, got: {msg}"
    );
}

#[test]
fn from_agents_non_canonical_names_run_successfully() {
    let agents = vec!["alpha".to_string(), "beta".to_string()];
    let pipeline = PipelineConfig::from_agents(&agents).expect("from_agents non-canonical");
    let mut sm = PipelineSm::new("run-nc".to_string(), pipeline);

    sm.handle(start_input()).expect("start");
    sm.handle(SmInput::StepPassed).expect("alpha passed");
    let events = sm.handle(SmInput::StepPassed).expect("beta passed");
    assert_eq!(
        sm.state(),
        RunStatus::Completed,
        "non-canonical names must run to Completed"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, Event::RunComplete { status: RunStatus::Completed, .. })));
}
