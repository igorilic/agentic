//! Findings repository — review-time findings produced by reviewer agents.
//!
//! Each row records a single finding (severity, message, optional file/line,
//! suggestion) and its triage state. Triage values are constrained to the
//! set documented in migration `0003_artifacts.sql`:
//! `null | 'fix' | 'tech-debt' | 'ignore'`.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingRow {
    pub id: String,
    pub run_id: String,
    pub step_id: String,
    pub severity: String,
    pub file_path: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub suggestion: Option<String>,
    pub triage: Option<String>,
    pub triaged_at: Option<i64>,
    pub created_at: i64,
}

/// Allowed `triage` values. Mirrors the comment in migration 0003 and the
/// trigger added in migration 0007.
pub const ALLOWED_TRIAGE: &[&str] = &["fix", "tech-debt", "ignore"];

/// Allowed `severity` values. Mirrors `events::Severity` as serialised
/// (`error | warning | info`). Note: a stray `"warn"` would deserialise into
/// the wrong colour in the UI, so we reject anything outside the canonical
/// set at write time.
pub const ALLOWED_SEVERITY: &[&str] = &["error", "warning", "info"];

pub struct FindingsRepo {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl FindingsRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    /// Insert a finding row. Caller supplies the id (typically the
    /// `finding_id` from the corresponding `Event::Finding` envelope).
    /// Returns `Err` if `severity` is not in [`ALLOWED_SEVERITY`].
    pub fn insert(&self, row: &FindingRow) -> Result<()> {
        if !ALLOWED_SEVERITY.contains(&row.severity.as_str()) {
            return Err(CoreError::Parse(format!(
                "invalid severity value: {:?} (allowed: {ALLOWED_SEVERITY:?})",
                row.severity
            )));
        }
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO findings \
             (id, run_id, step_id, severity, file_path, line, message, suggestion, \
              triage, triaged_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                row.id,
                row.run_id,
                row.step_id,
                row.severity,
                row.file_path,
                row.line,
                row.message,
                row.suggestion,
                row.triage,
                row.triaged_at,
                row.created_at,
            ],
        )?;
        Ok(())
    }

    /// List findings for a run in insertion order (by `created_at`).
    pub fn list_by_run(&self, run_id: &str) -> Result<Vec<FindingRow>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, run_id, step_id, severity, file_path, line, message, suggestion, \
                    triage, triaged_at, created_at \
             FROM findings \
             WHERE run_id = ?1 \
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![run_id], |r| {
                Ok(FindingRow {
                    id: r.get(0)?,
                    run_id: r.get(1)?,
                    step_id: r.get(2)?,
                    severity: r.get(3)?,
                    file_path: r.get(4)?,
                    line: r.get(5)?,
                    message: r.get(6)?,
                    suggestion: r.get(7)?,
                    triage: r.get(8)?,
                    triaged_at: r.get(9)?,
                    created_at: r.get(10)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Update the triage state for a finding identified by composite
    /// `(run_id, finding_id)` (matches the table's PRIMARY KEY since
    /// migration 0008). Returns `Ok(true)` if a row was updated,
    /// `Ok(false)` if no matching row exists.
    /// Returns `Err` if `triage` (after trimming) is not one of
    /// [`ALLOWED_TRIAGE`].
    ///
    /// `triage` is trimmed before validation so the server contract mirrors
    /// the frontend's whitespace tolerance — the same symmetry rule we apply
    /// to mention bodies.
    pub fn update_triage(
        &self,
        run_id: &str,
        finding_id: &str,
        triage: &str,
        triaged_at: i64,
    ) -> Result<bool> {
        let triage = triage.trim();
        if !ALLOWED_TRIAGE.contains(&triage) {
            return Err(CoreError::Parse(format!(
                "invalid triage value: {triage:?} (allowed: {ALLOWED_TRIAGE:?})"
            )));
        }
        let conn = self.pool.get()?;
        let updated = conn.execute(
            "UPDATE findings SET triage = ?1, triaged_at = ?2 \
             WHERE run_id = ?3 AND id = ?4",
            params![triage, triaged_at, run_id, finding_id],
        )?;
        Ok(updated > 0)
    }
}
