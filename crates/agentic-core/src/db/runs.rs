use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};

use crate::CoreError;
use crate::Result;
use crate::db::Db;
use crate::db::status::{run_status_from_str, run_status_to_str};
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
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO runs (id, workspace_id, pipeline_name, status, ticket_type, ticket_ref, \
             ticket_title, ticket_body, backend, model, started_at, completed_at, duration_ms, \
             token_usage, cost_usd, summary, subprocess_pid) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                run.id,
                run.workspace_id,
                run.pipeline_name,
                run_status_to_str(run.status),
                run.ticket_type,
                run.ticket_ref,
                run.ticket_title,
                run.ticket_body,
                run.backend,
                run.model,
                run.started_at,
                run.completed_at,
                run.duration_ms,
                run.token_usage,
                run.cost_usd,
                run.summary,
                run.subprocess_pid,
            ],
        )?;
        Ok(run)
    }

    pub fn get(&self, id: &str) -> Result<Option<Run>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT id, workspace_id, pipeline_name, status, ticket_type, ticket_ref, \
                 ticket_title, ticket_body, backend, model, started_at, completed_at, duration_ms, \
                 token_usage, cost_usd, summary, subprocess_pid \
                 FROM runs WHERE id = ?1",
                params![id],
                row_to_run,
            )
            .optional()?;
        Ok(row)
    }

    /// List recent runs across ALL workspaces, ordered by `started_at` DESC.
    /// Used by the cockpit's "past runs" pane which doesn't yet scope by
    /// workspace (#TODO: filter when the UI exposes a workspace switcher).
    pub fn list_recent(&self, limit: usize) -> Result<Vec<Run>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, pipeline_name, status, ticket_type, ticket_ref, \
             ticket_title, ticket_body, backend, model, started_at, completed_at, duration_ms, \
             token_usage, cost_usd, summary, subprocess_pid \
             FROM runs ORDER BY started_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], row_to_run)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_by_workspace(&self, workspace_id: &str, limit: usize) -> Result<Vec<Run>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, pipeline_name, status, ticket_type, ticket_ref, \
             ticket_title, ticket_body, backend, model, started_at, completed_at, duration_ms, \
             token_usage, cost_usd, summary, subprocess_pid \
             FROM runs WHERE workspace_id = ?1 ORDER BY started_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![workspace_id, limit as i64], row_to_run)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn mark_complete(
        &self,
        id: &str,
        to: RunStatus,
        completed_at: i64,
        duration_ms: i64,
    ) -> Result<()> {
        let mut conn = self.pool.get()?;
        // IMMEDIATE (not default DEFERRED): this transaction does SELECT-then-UPDATE.
        // See steps.rs::mark_complete for the full SQLITE_BUSY_SNAPSHOT rationale.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current_str: String =
            tx.query_row("SELECT status FROM runs WHERE id = ?1", params![id], |r| {
                r.get(0)
            })?;
        let from = run_status_from_str(&current_str)
            .ok_or_else(|| CoreError::Db(format!("unknown status: {current_str}")))?;
        if !is_legal_run_transition(from, to) {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_to_str(from).to_string(),
                to: run_status_to_str(to).to_string(),
            });
        }
        tx.execute(
            "UPDATE runs SET status = ?1, completed_at = ?2, duration_ms = ?3 WHERE id = ?4",
            params![run_status_to_str(to), completed_at, duration_ms, id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn transition(&self, id: &str, to: RunStatus) -> Result<()> {
        let mut conn = self.pool.get()?;
        // IMMEDIATE: same SELECT-then-UPDATE pattern; see mark_complete above.
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let current_str: String =
            tx.query_row("SELECT status FROM runs WHERE id = ?1", params![id], |r| {
                r.get(0)
            })?;
        let from = run_status_from_str(&current_str)
            .ok_or_else(|| CoreError::Db(format!("unknown status: {current_str}")))?;
        if !is_legal_run_transition(from, to) {
            return Err(CoreError::InvalidStateTransition {
                from: run_status_to_str(from).to_string(),
                to: run_status_to_str(to).to_string(),
            });
        }
        tx.execute(
            "UPDATE runs SET status = ?1 WHERE id = ?2",
            params![run_status_to_str(to), id],
        )?;
        tx.commit()?;
        Ok(())
    }
}

fn row_to_run(r: &rusqlite::Row<'_>) -> rusqlite::Result<Run> {
    let status_str: String = r.get(3)?;
    let status = run_status_from_str(&status_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            3,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::other(format!(
                "unknown run status: {status_str}"
            ))),
        )
    })?;
    Ok(Run {
        id: r.get(0)?,
        workspace_id: r.get(1)?,
        pipeline_name: r.get(2)?,
        status,
        ticket_type: r.get(4)?,
        ticket_ref: r.get(5)?,
        ticket_title: r.get(6)?,
        ticket_body: r.get(7)?,
        backend: r.get(8)?,
        model: r.get(9)?,
        started_at: r.get(10)?,
        completed_at: r.get(11)?,
        duration_ms: r.get(12)?,
        token_usage: r.get(13)?,
        cost_usd: r.get(14)?,
        summary: r.get(15)?,
        subprocess_pid: r.get(16)?,
    })
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
