use crate::events::{RunStatus, StepStatus};

pub(super) fn run_status_to_str(s: RunStatus) -> &'static str {
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

pub(super) fn run_status_from_str(s: &str) -> Option<RunStatus> {
    Some(match s {
        "pending" => RunStatus::Pending,
        "running" => RunStatus::Running,
        "completed" => RunStatus::Completed,
        "completed_with_tech_debt" => RunStatus::CompletedWithTechDebt,
        "failed" => RunStatus::Failed,
        "cancelled" => RunStatus::Cancelled,
        "crashed" => RunStatus::Crashed,
        _ => return None,
    })
}

pub(super) fn step_status_to_str(s: StepStatus) -> &'static str {
    match s {
        StepStatus::Pending => "pending",
        StepStatus::Running => "running",
        StepStatus::Passed => "passed",
        StepStatus::Failed => "failed",
        StepStatus::NeedsTriage => "needs_triage",
        StepStatus::Skipped => "skipped",
    }
}

pub(super) fn step_status_from_str(s: &str) -> Option<StepStatus> {
    Some(match s {
        "pending" => StepStatus::Pending,
        "running" => StepStatus::Running,
        "passed" => StepStatus::Passed,
        "failed" => StepStatus::Failed,
        "needs_triage" => StepStatus::NeedsTriage,
        "skipped" => StepStatus::Skipped,
        _ => return None,
    })
}
