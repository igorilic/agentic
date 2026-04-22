use r2d2_sqlite::SqliteConnectionManager;

use crate::Result;
use crate::db::Db;
use crate::events::StepStatus;

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: String,
    pub run_id: String,
    pub seq: i64,
    pub agent_name: String,
    pub status: StepStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub duration_ms: Option<i64>,
    pub token_usage: Option<String>,
    pub cost_usd: Option<f64>,
    pub summary: Option<String>,
    pub retry_count: i64,
}

#[derive(Clone)]
pub struct StepRepo {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl StepRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    pub fn insert(&self, step: Step) -> Result<Step> {
        unimplemented!()
    }

    pub fn get(&self, id: &str) -> Result<Option<Step>> {
        unimplemented!()
    }

    pub fn list_by_run(&self, run_id: &str) -> Result<Vec<Step>> {
        unimplemented!()
    }

    pub fn transition(&self, id: &str, to: StepStatus) -> Result<()> {
        unimplemented!()
    }
}

fn is_legal_step_transition(from: StepStatus, to: StepStatus) -> bool {
    use StepStatus::*;
    matches!(
        (from, to),
        (Pending, Running)
            | (Pending, Skipped)
            | (Running, Passed)
            | (Running, Failed)
            | (Running, NeedsTriage)
    )
}
