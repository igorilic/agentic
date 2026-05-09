//! Pipeline-preset repository.
//!
//! Persists named, ordered collections of agent ids to the
//! `pipeline_presets` table. Ordering of `agents` is significant and
//! duplicates are allowed (stored as a JSON array).

use rusqlite::params;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::db::Db;
use crate::error::{CoreError, Result};
use crate::time::now_ms;

/// A named, ordered collection of agent ids.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelinePreset {
    pub id: String,
    pub name: String,
    pub agents: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct PipelinePresetRepo {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl PipelinePresetRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    /// List all presets ordered by name ASC.
    pub fn list(&self) -> Result<Vec<PipelinePreset>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, agents, created_at, updated_at \
             FROM pipeline_presets \
             ORDER BY name ASC",
        )?;
        let rows = stmt
            .query_map([], row_to_preset)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// Create a new preset with a fresh ULID.
    ///
    /// - Trims `name`; errors if empty after trim or longer than 64 chars.
    /// - Errors if `agents` is empty.
    /// - Errors with `CoreError::Db` if `name` is already taken (UNIQUE constraint).
    pub fn create(&self, name: &str, agents: &[String]) -> Result<PipelinePreset> {
        let trimmed = validate(name, agents)?;
        let id = Ulid::new().to_string();
        let now = now_ms();
        let agents_json = serde_json::to_string(agents)
            .map_err(|e| CoreError::Db(format!("failed to encode agents: {e}")))?;

        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO pipeline_presets (id, name, agents, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, trimmed, agents_json, now, now],
        )?;

        Ok(PipelinePreset {
            id,
            name: trimmed,
            agents: agents.to_vec(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update an existing preset by id.
    ///
    /// Same validation as [`create`]. Errors if the id is not found.
    pub fn update(&self, id: &str, name: &str, agents: &[String]) -> Result<PipelinePreset> {
        let trimmed = validate(name, agents)?;
        let now = now_ms();
        let agents_json = serde_json::to_string(agents)
            .map_err(|e| CoreError::Db(format!("failed to encode agents: {e}")))?;

        let conn = self.pool.get()?;
        let n = conn.execute(
            "UPDATE pipeline_presets \
             SET name = ?1, agents = ?2, updated_at = ?3 \
             WHERE id = ?4",
            params![trimmed, agents_json, now, id],
        )?;

        if n == 0 {
            return Err(CoreError::Db(format!("pipeline preset not found: {id}")));
        }

        // Fetch the full row using the same connection so created_at comes from
        // the DB (pool max_size=1 in tests — can't call get_by_id which would
        // try to acquire a second connection from the pool).
        let mut stmt = conn.prepare(
            "SELECT id, name, agents, created_at, updated_at \
             FROM pipeline_presets \
             WHERE id = ?1",
        )?;
        let mut iter = stmt.query_map(params![id], row_to_preset)?;
        iter.next()
            .ok_or_else(|| {
                CoreError::Db(format!("pipeline preset disappeared after update: {id}"))
            })?
            .map_err(CoreError::from)
    }

    /// Delete by id. Errors if the id is not found.
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let n = conn.execute("DELETE FROM pipeline_presets WHERE id = ?1", params![id])?;
        if n == 0 {
            return Err(CoreError::Db(format!("pipeline preset not found: {id}")));
        }
        Ok(())
    }

    /// Return the preset with `id`, or `None` if it doesn't exist.
    pub fn get_by_id(&self, id: &str) -> Result<Option<PipelinePreset>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, agents, created_at, updated_at \
             FROM pipeline_presets \
             WHERE id = ?1",
        )?;
        let mut iter = stmt.query_map(params![id], row_to_preset)?;
        match iter.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Return the preset with `name`, or `None` if it doesn't exist.
    pub fn get_by_name(&self, name: &str) -> Result<Option<PipelinePreset>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, agents, created_at, updated_at \
             FROM pipeline_presets \
             WHERE name = ?1",
        )?;
        let mut iter = stmt.query_map(params![name], row_to_preset)?;
        match iter.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Validate `name` and `agents`, returning the trimmed name on success.
fn validate(name: &str, agents: &[String]) -> Result<String> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(CoreError::Db("preset name is empty after trim".to_string()));
    }
    if trimmed.chars().count() > 64 {
        return Err(CoreError::Db(
            "preset name exceeds 64 characters".to_string(),
        ));
    }
    if agents.is_empty() {
        return Err(CoreError::Db(
            "preset agents list must not be empty".to_string(),
        ));
    }
    Ok(trimmed)
}

fn row_to_preset(r: &rusqlite::Row<'_>) -> rusqlite::Result<PipelinePreset> {
    let agents_json: String = r.get(2)?;
    let agents: Vec<String> = serde_json::from_str(&agents_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(PipelinePreset {
        id: r.get(0)?,
        name: r.get(1)?,
        agents,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
    })
}
