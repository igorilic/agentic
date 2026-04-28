//! Step 12.3: cockpit-side run state.
//!
//! Mirrors what `apps/web-ui/src/types/run.ts` carries on the Tauri side:
//! a fixed-order list of pipeline-step rows, each with an icon-friendly
//! status. Updates flow from `EventEnvelope`s — typically forwarded from
//! the core bus by the binary, but in tests we feed envelopes directly.

use agentic_core::events::{Event, EventEnvelope, StepStatus};

/// Canonical pipeline-step order. Matches `pipeline::config` and the
/// React `Stepper.tsx` component, which iterates the same agent names.
pub const CANONICAL_AGENTS: [&str; 4] = ["architect", "tdd-developer", "qa", "reviewer"];

/// One step's run-time state. We keep our own enum (rather than reusing
/// `agentic_core::events::StepStatus` directly) so a row can sit in
/// `Pending` before any event has fired — the core enum has no `Pending`
/// concept on the wire, only as a DB row state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepRunStatus {
    Pending,
    Running,
    Passed,
    Failed,
    NeedsTriage,
    Skipped,
}

impl StepRunStatus {
    /// Single Unicode glyph rendered in the cockpit pane. Matches the
    /// icons in `apps/web-ui/src/components/Stepper.tsx` so users
    /// switching between TUI and Tauri see consistent symbols.
    pub fn icon(self) -> char {
        match self {
            Self::Pending => '○',
            Self::Running => '◐',
            Self::Passed => '✓',
            Self::Failed => '✗',
            Self::NeedsTriage => '⚠',
            Self::Skipped => '⊘',
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepRow {
    pub agent: String,
    pub status: StepRunStatus,
}

#[derive(Debug, Clone)]
pub struct RunState {
    pub steps: Vec<StepRow>,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            steps: CANONICAL_AGENTS
                .iter()
                .map(|a| StepRow {
                    agent: (*a).to_string(),
                    status: StepRunStatus::Pending,
                })
                .collect(),
        }
    }
}

impl RunState {
    /// Apply a bus envelope. Only `StepStarted` and `StepComplete` mutate
    /// row status; everything else (TextDelta, ToolUseStart, …) is
    /// silently ignored — those affect detail panes that arrive in
    /// later phases.
    pub fn apply_envelope(&mut self, envelope: &EventEnvelope) {
        match &envelope.event {
            Event::StepStarted { agent, .. } => {
                if let Some(row) = self.row_mut(agent) {
                    row.status = StepRunStatus::Running;
                }
            }
            Event::StepComplete { status, .. } => {
                // Primary: route via step_id ("<run>-step-<seq>-<agent>").
                // Fallback: the orchestrator runs steps sequentially, so
                // exactly one row should be in Running — apply the
                // status to that row. Covers replayed events and any
                // future Event::StepComplete payload that drops step_id.
                let agent = envelope
                    .step_id
                    .as_deref()
                    .and_then(extract_agent_from_step_id);
                let row = match agent {
                    Some(a) => self.row_mut(a),
                    None => self.running_row_mut(),
                };
                if let Some(row) = row {
                    row.status = map_step_status(*status);
                }
            }
            _ => {}
        }
    }

    fn row_mut(&mut self, agent: &str) -> Option<&mut StepRow> {
        self.steps.iter_mut().find(|r| r.agent == agent)
    }

    fn running_row_mut(&mut self) -> Option<&mut StepRow> {
        self.steps
            .iter_mut()
            .find(|r| r.status == StepRunStatus::Running)
    }
}

/// Pull the trailing agent name out of a step_id like
/// `"run1-step-0-architect"`. We simply take the last hyphen-separated
/// segment that matches a canonical agent — multi-hyphen agents like
/// `tdd-developer` need a small two-segment lookback.
fn extract_agent_from_step_id(step_id: &str) -> Option<&'static str> {
    CANONICAL_AGENTS
        .into_iter()
        .find(|agent| step_id.ends_with(agent))
}

fn map_step_status(s: StepStatus) -> StepRunStatus {
    match s {
        StepStatus::Pending => StepRunStatus::Pending,
        StepStatus::Running => StepRunStatus::Running,
        StepStatus::Passed => StepRunStatus::Passed,
        StepStatus::Failed => StepRunStatus::Failed,
        StepStatus::NeedsTriage => StepRunStatus::NeedsTriage,
        StepStatus::Skipped => StepRunStatus::Skipped,
    }
}
