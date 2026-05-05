use crate::events::{
    BackendId, Event, ModelId, ProfileId, RunStatus, StepStatus, TicketRef, TokenUsage,
};
use crate::pipeline::Pipeline;
use crate::{CoreError, Result};

/// Inputs the state machine accepts.
#[derive(Debug, Clone)]
pub enum SmInput {
    /// Start the run. Transitions pending → running and starts the first step.
    Start {
        ticket: TicketRef,
        profile: ProfileId,
        backend: BackendId,
        model: ModelId,
    },
    /// Current step passed.
    StepPassed,
    /// Current step failed. Routes via `stop_on_failure`: true → Failed, false → advance.
    StepFailed,
    /// Reviewer-specific: step produced findings requiring triage.
    StepNeedsTriage,
    /// Skip the current step.
    StepSkipped,
    /// User-initiated cancellation.
    Cancel,
    /// External crash (subprocess killed, etc.).
    Crash,
}

/// Pure state machine. No IO, no backends — produces `Event`s that the
/// orchestrator will wrap in envelopes and publish.
///
/// The SM is a linear N-step advancer. Each step runs once. Failure
/// routing is controlled by `PipelineStep.stop_on_failure` only — no
/// name-based retry logic exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineSm {
    /// Stored for use by the orchestrator; unused in the SM itself.
    #[allow(dead_code)]
    run_id: String,
    pipeline: Pipeline,
    state: RunStatus,
    /// Index into `pipeline.steps` for the currently-active step.
    /// Meaningful only when `state == Running`.
    step_index: usize,
    /// Per-step status, parallel to `pipeline.steps`.
    step_statuses: Vec<StepStatus>,
}

impl PipelineSm {
    pub fn new(run_id: String, pipeline: Pipeline) -> Self {
        let step_count = pipeline.steps.len();
        Self {
            run_id,
            pipeline,
            state: RunStatus::Pending,
            step_index: 0,
            step_statuses: vec![StepStatus::Pending; step_count],
        }
    }

    pub fn state(&self) -> RunStatus {
        self.state
    }

    pub fn step_statuses(&self) -> &[StepStatus] {
        &self.step_statuses
    }

    pub fn current_step_index(&self) -> Option<usize> {
        if self.state == RunStatus::Running {
            Some(self.step_index)
        } else {
            None
        }
    }

    /// Process one input, return the events that should be broadcast.
    /// Returns `Err(CoreError::InvalidStateTransition)` if the input is
    /// not legal in the current state.
    pub fn handle(&mut self, input: SmInput) -> Result<Vec<Event>> {
        if is_terminal(self.state) {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "(attempting input on terminal state)".to_string(),
            });
        }
        match input {
            SmInput::Start {
                ticket,
                profile,
                backend,
                model,
            } => self.handle_start(ticket, profile, backend, model),
            SmInput::StepPassed => self.handle_step_passed(),
            SmInput::StepFailed => self.handle_step_failed(),
            SmInput::StepNeedsTriage => self.handle_step_needs_triage(),
            SmInput::StepSkipped => self.handle_step_skipped(),
            SmInput::Cancel => self.handle_cancel(),
            SmInput::Crash => self.handle_crash(),
        }
    }

    fn handle_start(
        &mut self,
        ticket: TicketRef,
        profile: ProfileId,
        backend: BackendId,
        model: ModelId,
    ) -> Result<Vec<Event>> {
        if self.state != RunStatus::Pending {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "running".to_string(),
            });
        }
        self.state = RunStatus::Running;
        self.step_index = 0;
        self.step_statuses[0] = StepStatus::Running;

        let first_agent = self.pipeline.steps[0].agent.clone();
        Ok(vec![
            Event::RunStarted {
                ticket,
                profile,
                backend,
                model: model.clone(),
            },
            Event::StepStarted {
                agent: first_agent,
                model,
            },
        ])
    }

    fn handle_step_passed(&mut self) -> Result<Vec<Event>> {
        if self.state != RunStatus::Running {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "running".to_string(),
            });
        }
        let idx = self.step_index;
        self.step_statuses[idx] = StepStatus::Passed;

        let mut events = vec![Event::StepComplete {
            status: StepStatus::Passed,
            summary: String::new(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 0,
        }];

        events.extend(self.advance_or_complete());
        Ok(events)
    }

    fn handle_step_failed(&mut self) -> Result<Vec<Event>> {
        if self.state != RunStatus::Running {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "running".to_string(),
            });
        }
        let idx = self.step_index;
        self.step_statuses[idx] = StepStatus::Failed;

        let mut events = vec![Event::StepComplete {
            status: StepStatus::Failed,
            summary: String::new(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 0,
        }];

        let stop_on_failure = self.pipeline.steps[idx].stop_on_failure;

        if stop_on_failure {
            self.state = RunStatus::Failed;
            events.push(Event::RunComplete {
                status: RunStatus::Failed,
                duration_ms: 0,
                summary: String::new(),
            });
        } else {
            events.extend(self.advance_or_complete());
        }

        Ok(events)
    }

    fn handle_step_needs_triage(&mut self) -> Result<Vec<Event>> {
        if self.state != RunStatus::Running {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "running".to_string(),
            });
        }
        let idx = self.step_index;
        self.step_statuses[idx] = StepStatus::NeedsTriage;

        let mut events = vec![Event::StepComplete {
            status: StepStatus::NeedsTriage,
            summary: String::new(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 0,
        }];

        events.extend(self.advance_or_complete());
        Ok(events)
    }

    fn handle_step_skipped(&mut self) -> Result<Vec<Event>> {
        if self.state != RunStatus::Running {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_snake_case(self.state).to_string(),
                to: "running".to_string(),
            });
        }
        let idx = self.step_index;
        self.step_statuses[idx] = StepStatus::Skipped;

        let mut events = vec![Event::StepComplete {
            status: StepStatus::Skipped,
            summary: String::new(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
            duration_ms: 0,
        }];

        events.extend(self.advance_or_complete());
        Ok(events)
    }

    fn handle_cancel(&mut self) -> Result<Vec<Event>> {
        if self.state == RunStatus::Running {
            self.step_statuses[self.step_index] = StepStatus::Failed;
        }
        self.state = RunStatus::Cancelled;
        Ok(vec![Event::RunComplete {
            status: RunStatus::Cancelled,
            duration_ms: 0,
            summary: String::new(),
        }])
    }

    fn handle_crash(&mut self) -> Result<Vec<Event>> {
        if self.state == RunStatus::Running {
            self.step_statuses[self.step_index] = StepStatus::Failed;
        }
        self.state = RunStatus::Crashed;
        Ok(vec![Event::RunComplete {
            status: RunStatus::Crashed,
            duration_ms: 0,
            summary: String::new(),
        }])
    }

    /// Advance to the next step, or complete the run if this was the last step.
    /// Returns the new events to emit.
    fn advance_or_complete(&mut self) -> Vec<Event> {
        let idx = self.step_index;
        if idx + 1 < self.pipeline.steps.len() {
            self.step_index = idx + 1;
            self.step_statuses[idx + 1] = StepStatus::Running;
            let next_agent = self.pipeline.steps[idx + 1].agent.clone();
            vec![Event::StepStarted {
                agent: next_agent,
                model: ModelId(String::new()),
            }]
        } else {
            self.state = RunStatus::Completed;
            vec![Event::RunComplete {
                status: RunStatus::Completed,
                duration_ms: 0,
                summary: String::new(),
            }]
        }
    }
}

fn is_terminal(s: RunStatus) -> bool {
    matches!(
        s,
        RunStatus::Completed
            | RunStatus::CompletedWithTechDebt
            | RunStatus::Failed
            | RunStatus::Cancelled
            | RunStatus::Crashed
    )
}

fn run_status_snake_case(s: RunStatus) -> &'static str {
    match s {
        RunStatus::Pending => "pending",
        RunStatus::Running => "running",
        RunStatus::Completed => "completed",
        RunStatus::CompletedWithTechDebt => "completed_with_tech_debt",
        RunStatus::Failed => "failed",
        RunStatus::Cancelled => "cancelled",
        RunStatus::Crashed => "crashed",
    }
}
