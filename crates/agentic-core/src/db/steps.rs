use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};

use crate::CoreError;
use crate::Result;
use crate::db::Db;
use crate::db::status::{step_status_from_str, step_status_to_str};
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
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO run_steps (id, run_id, seq, agent_name, status, started_at, completed_at, \
             duration_ms, token_usage, cost_usd, summary, retry_count) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                step.id,
                step.run_id,
                step.seq,
                step.agent_name,
                step_status_to_str(step.status),
                step.started_at,
                step.completed_at,
                step.duration_ms,
                step.token_usage,
                step.cost_usd,
                step.summary,
                step.retry_count,
            ],
        )?;
        Ok(step)
    }

    pub fn get(&self, id: &str) -> Result<Option<Step>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT id, run_id, seq, agent_name, status, started_at, completed_at, \
                 duration_ms, token_usage, cost_usd, summary, retry_count \
                 FROM run_steps WHERE id = ?1",
                params![id],
                row_to_step,
            )
            .optional()?;
        Ok(row)
    }

    pub fn list_by_run(&self, run_id: &str) -> Result<Vec<Step>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, seq, agent_name, status, started_at, completed_at, \
             duration_ms, token_usage, cost_usd, summary, retry_count \
             FROM run_steps WHERE run_id = ?1 ORDER BY seq ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id], row_to_step)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn mark_complete(
        &self,
        id: &str,
        to: StepStatus,
        completed_at: i64,
        duration_ms: i64,
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        // IMMEDIATE (not default DEFERRED): this transaction does SELECT-then-UPDATE.
        // Under WAL with concurrent writers, a DEFERRED read→write upgrade can fail
        // with SQLITE_BUSY_SNAPSHOT, which busy_timeout does NOT retry — the
        // transaction must be aborted and retried manually. IMMEDIATE acquires the
        // write lock up front, eliminating the snapshot-mismatch window. This is
        // what CP-5 surfaced: concurrent orch + persister writers dropped events
        // silently before this fix.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current_str: String = tx.query_row(
            "SELECT status FROM run_steps WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let from = step_status_from_str(&current_str)
            .ok_or_else(|| CoreError::Db(format!("unknown step status: {current_str}")))?;
        if !is_legal_step_transition(from, to) {
            return Err(CoreError::InvalidStateTransition {
                from: step_status_to_str(from).to_string(),
                to: step_status_to_str(to).to_string(),
            });
        }
        tx.execute(
            "UPDATE run_steps SET status = ?1, completed_at = ?2, duration_ms = ?3 WHERE id = ?4",
            params![step_status_to_str(to), completed_at, duration_ms, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn transition(&self, id: &str, to: StepStatus) -> Result<()> {
        let mut conn = self.pool.get()?;
        // IMMEDIATE: same SELECT-then-UPDATE pattern; see mark_complete above.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current_str: String = tx.query_row(
            "SELECT status FROM run_steps WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )?;
        let from = step_status_from_str(&current_str)
            .ok_or_else(|| CoreError::Db(format!("unknown status: {current_str}")))?;
        if !is_legal_step_transition(from, to) {
            return Err(CoreError::InvalidStateTransition {
                from: step_status_to_str(from).to_string(),
                to: step_status_to_str(to).to_string(),
            });
        }
        tx.execute(
            "UPDATE run_steps SET status = ?1 WHERE id = ?2",
            params![step_status_to_str(to), id],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn row_to_step(r: &rusqlite::Row<'_>) -> rusqlite::Result<Step> {
    let status_str: String = r.get(4)?;
    let status = step_status_from_str(&status_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(format!(
                "unknown step status: {status_str}"
            ))),
        )
    })?;
    Ok(Step {
        id: r.get(0)?,
        run_id: r.get(1)?,
        seq: r.get(2)?,
        agent_name: r.get(3)?,
        status,
        started_at: r.get(5)?,
        completed_at: r.get(6)?,
        duration_ms: r.get(7)?,
        token_usage: r.get(8)?,
        cost_usd: r.get(9)?,
        summary: r.get(10)?,
        retry_count: r.get(11)?,
    })
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
