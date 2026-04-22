use r2d2_sqlite::SqliteConnectionManager;

use crate::Result;
use crate::db::Db;
use crate::events::RunStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub id: String,
    pub workspace_id: String,
    pub pipeline_name: String,
    pub status: RunStatus,
    pub ticket_type: Option<String>,
    pub ticket_ref: Option<String>,
    pub ticket_title: Option<String>,
    pub ticket_body: Option<String>,
    pub backend: String,
    pub model: String,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub token_usage: Option<String>,
    pub cost_usd: Option<f64>,
    pub summary: Option<String>,
    pub subprocess_pid: Option<i64>,
}

#[derive(Clone)]
pub struct RunRepo {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl RunRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    pub fn insert(&self, run: Run) -> Result<Run> {
        unimplemented!()
    }

    pub fn get(&self, id: &str) -> Result<Option<Run>> {
        unimplemented!()
    }

    pub fn list_by_workspace(&self, workspace_id: &str, limit: usize) -> Result<Vec<Run>> {
        unimplemented!()
    }

    pub fn transition(&self, id: &str, to: RunStatus) -> Result<()> {
        unimplemented!()
    }
}

fn is_legal_run_transition(from: RunStatus, to: RunStatus) -> bool {
    use RunStatus::*;
    matches!(
        (from, to),
        (Pending, Running)
            | (Pending, Cancelled)
            | (Running, Completed)
            | (Running, CompletedWithTechDebt)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Running, Crashed)
    )
}
