//! Tauri IPC for browsing past runs.

use agentic_core::Db;
use agentic_core::db::runs::{Run, RunRepo};
use agentic_core::events::RunStatus;
use serde::Serialize;
use tauri::State;

fn run_status_to_str(s: RunStatus) -> &'static str {
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

/// Read-only summary returned by `list_runs`. Mirrors the fields the
/// cockpit's PastRunsPane actually renders — keeps the IPC payload small
/// and avoids leaking infrequently-used columns.
#[derive(Debug, Serialize)]
pub struct RunSummary {
    pub id: String,
    pub workspace_id: String,
    pub status: String,
    pub backend: String,
    pub model: String,
    /// ticket_body when ticket_type is "free-text"; otherwise the
    /// `ticket_ref`. Either way, a single human-readable label.
    pub ticket_label: Option<String>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
}

impl RunSummary {
    fn from_run(r: Run) -> Self {
        let ticket_label = r.ticket_body.clone().or(r.ticket_ref.clone());
        Self {
            id: r.id,
            workspace_id: r.workspace_id,
            status: run_status_to_str(r.status).to_string(),
            backend: r.backend,
            model: r.model,
            ticket_label,
            started_at: r.started_at,
            completed_at: r.completed_at,
            duration_ms: r.duration_ms,
        }
    }
}

/// Tauri command. Returns the most recent runs across all workspaces,
/// ordered by `started_at` DESC. `limit` defaults to 50 — small enough
/// to render without virtualisation, large enough to cover a day of
/// /plan iterations.
#[tauri::command]
pub async fn list_runs(
    db_state: State<'_, Db>,
    limit: Option<u32>,
) -> Result<Vec<RunSummary>, String> {
    let limit = limit.unwrap_or(50).max(1) as usize;
    let runs = RunRepo::new(&db_state)
        .list_recent(limit)
        .map_err(|e| e.to_string())?;
    Ok(runs.into_iter().map(RunSummary::from_run).collect())
}
