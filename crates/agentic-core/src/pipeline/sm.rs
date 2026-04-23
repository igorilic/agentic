use crate::events::{
    BackendId, Event, ModelId, ProfileId, RunStatus, StepStatus, TicketRef,
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
    /// Current step failed. For qa, may trigger a retry loop.
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
/// orchestrator (Step 3.5) will wrap in envelopes and publish.
#[derive(Debug, Clone)]
pub struct PipelineSm {
    run_id: String,
    pipeline: Pipeline,
    state: RunStatus,
    /// Index into `pipeline.steps` for the currently-active step.
    /// Meaningful only when `state == Running`.
    step_index: usize,
    /// Per-step status, parallel to `pipeline.steps`.
    step_statuses: Vec<StepStatus>,
    /// Number of times the qa fix-loop has been triggered.
    qa_retries: u32,
    /// Cap from `PipelineStep.qa_fix_loop_cap` on tdd-developer, or default 3.
    qa_fix_loop_cap: u32,
    /// Set to true when QA has exhausted retries; affects RunComplete status.
    has_tech_debt: bool,
}

impl PipelineSm {
    pub fn new(run_id: String, pipeline: Pipeline) -> Self {
        let step_count = pipeline.steps.len();
        // Default qa cap comes from tdd-developer's config, fallback 3.
        let qa_fix_loop_cap = pipeline
            .steps
            .iter()
            .find(|s| s.agent == "tdd-developer")
            .and_then(|s| s.qa_fix_loop_cap)
            .unwrap_or(3);
        Self {
            run_id,
            pipeline,
            state: RunStatus::Pending,
            step_index: 0,
            step_statuses: vec![StepStatus::Pending; step_count],
            qa_retries: 0,
            qa_fix_loop_cap,
            has_tech_debt: false,
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
        unimplemented!("pipeline state machine not yet implemented")
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
