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
    /// Stored for use by the orchestrator (Step 3.5); unused in the SM itself.
    #[allow(dead_code)]
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
        }];

        let agent = self.pipeline.steps[idx].agent.clone();
        let stop_on_failure = self.pipeline.steps[idx].stop_on_failure;

        if agent == "qa" {
            if self.qa_retries < self.qa_fix_loop_cap {
                // Retry: bounce back to the tdd-developer step (index - 1 before qa)
                self.qa_retries += 1;
                let tdd_idx = self.find_tdd_developer_before_qa(idx);
                self.step_index = tdd_idx;
                self.step_statuses[tdd_idx] = StepStatus::Running;
                let retry_attempt = self.qa_retries;
                let tdd_agent = self.pipeline.steps[tdd_idx].agent.clone();
                events.push(Event::RetryStarted {
                    attempt: retry_attempt,
                    reason: "qa failed".to_string(),
                });
                events.push(Event::StepStarted {
                    agent: tdd_agent,
                    model: ModelId(String::new()),
                });
            } else {
                // Cap exhausted: tech-debt path, advance to reviewer
                self.has_tech_debt = true;
                // Try to advance past qa to next step
                if idx + 1 < self.pipeline.steps.len() {
                    self.step_index = idx + 1;
                    self.step_statuses[idx + 1] = StepStatus::Running;
                    let next_agent = self.pipeline.steps[idx + 1].agent.clone();
                    events.push(Event::StepStarted {
                        agent: next_agent,
                        model: ModelId(String::new()),
                    });
                } else {
                    // qa is the last step; complete with tech debt
                    self.state = RunStatus::CompletedWithTechDebt;
                    events.push(Event::RunComplete {
                        status: RunStatus::CompletedWithTechDebt,
                        duration_ms: 0,
                        summary: String::new(),
                    });
                }
            }
        } else if stop_on_failure {
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
            let final_status = if self.has_tech_debt {
                RunStatus::CompletedWithTechDebt
            } else {
                RunStatus::Completed
            };
            self.state = final_status;
            vec![Event::RunComplete {
                status: final_status,
                duration_ms: 0,
                summary: String::new(),
            }]
        }
    }

    /// Find the tdd-developer step index that precedes the given qa index.
    /// Falls back to idx.saturating_sub(1) if not found by name.
    fn find_tdd_developer_before_qa(&self, qa_idx: usize) -> usize {
        // Search backward from qa_idx for the nearest tdd-developer step
        for i in (0..qa_idx).rev() {
            if self.pipeline.steps[i].agent == "tdd-developer" {
                return i;
            }
        }
        qa_idx.saturating_sub(1)
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
