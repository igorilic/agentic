use agentic_core::Db;
use agentic_core::db::findings::{FindingRow, FindingsRepo};
use tauri::State;

/// State holding the DB-backed findings repo. Distinct from `EventBusState`
/// and `ChatState` so each command pulls only the resources it needs.
pub struct FindingsState {
    pub repo: FindingsRepo,
}

impl FindingsState {
    pub fn new(db: &Db) -> Self {
        Self {
            repo: FindingsRepo::new(db),
        }
    }
}

/// Tauri command: triage a finding.
///
/// `triage` must be one of `"fix" | "tech-debt" | "ignore"`. Returns
/// `Err` for an unknown finding id or invalid triage value — the frontend
/// surfaces both as a generic "triage failed" toast.
#[tauri::command]
pub async fn triage_finding(
    state: State<'_, FindingsState>,
    finding_id: String,
    triage: String,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let updated = state
        .repo
        .update_triage(&finding_id, &triage, now)
        .map_err(|e| e.to_string())?;

    if !updated {
        return Err(format!("finding not found: {finding_id}"));
    }

    Ok(())
}

/// Tauri command: list findings for a run.
///
/// Used by the cockpit's `useFindings(runId)` hook. Returns an empty vec for
/// unknown run ids — `list_findings` is not authoritative on whether a run
/// exists, only on which findings have been persisted for it so far.
#[tauri::command]
pub async fn list_findings(
    state: State<'_, FindingsState>,
    run_id: String,
) -> Result<Vec<FindingRow>, String> {
    state.repo.list_by_run(&run_id).map_err(|e| e.to_string())
}
