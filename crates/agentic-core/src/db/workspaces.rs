use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{OptionalExtension, params};

use crate::Result;
use crate::db::Db;
use crate::time::now_ms;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub remote_url: Option<String>,
    pub profile: String,
    pub created_at: i64,
    pub last_opened: i64,
}

impl Workspace {
    /// Compute the deterministic workspace id per spec §9.1:
    /// blake3(remote_url || canonical_path)[..16] hex-encoded.
    ///
    /// If `remote_url` is None (new repo, no remotes), falls back to hashing
    /// the canonical path alone. Callers should supply the canonicalized form
    /// (symlinks resolved) so two checkouts of the same path produce the
    /// same id regardless of prior mount state.
    pub fn compute_id(remote_url: Option<&str>, canonical_path: &str) -> String {
        let mut hasher = blake3::Hasher::new();
        if let Some(url) = remote_url {
            hasher.update(url.as_bytes());
        }
        hasher.update(canonical_path.as_bytes());
        let hash = hasher.finalize();
        let hex = hash.to_hex(); // ArrayString<64> — 32 bytes hex-encoded
        hex[..32].to_string() // first 16 bytes = 32 hex chars
    }
}

#[derive(Clone)]
pub struct WorkspaceRepo {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl WorkspaceRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    /// Insert a workspace. Returns the input unchanged on success (the id,
    /// timestamps, and other fields are expected to be fully set by the
    /// caller).
    pub fn insert(&self, ws: Workspace) -> Result<Workspace> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO workspaces (id, name, root_path, remote_url, profile, created_at, last_opened) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ws.id, ws.name, ws.root_path, ws.remote_url, ws.profile,
                ws.created_at, ws.last_opened,
            ],
        )?;
        Ok(ws)
    }

    /// Insert a workspace only if no row with the same `id` already exists
    /// (INSERT OR IGNORE semantics). Returns the input unchanged on success.
    /// Silently succeeds (no-op) when the id is already present.
    pub fn insert_if_absent(&self, ws: Workspace) -> Result<Workspace> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO workspaces \
             (id, name, root_path, remote_url, profile, created_at, last_opened) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ws.id,
                ws.name,
                ws.root_path,
                ws.remote_url,
                ws.profile,
                ws.created_at,
                ws.last_opened,
            ],
        )?;
        Ok(ws)
    }

    /// Return the workspace with the given id, or `None` if absent.
    pub fn get(&self, id: &str) -> Result<Option<Workspace>> {
        let conn = self.pool.get()?;
        let row = conn
            .query_row(
                "SELECT id, name, root_path, remote_url, profile, created_at, last_opened \
                 FROM workspaces WHERE id = ?1",
                params![id],
                |r| {
                    Ok(Workspace {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        root_path: r.get(2)?,
                        remote_url: r.get(3)?,
                        profile: r.get(4)?,
                        created_at: r.get(5)?,
                        last_opened: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Most-recently-opened workspaces first. `limit` caps the returned rows.
    pub fn list_recent(&self, limit: usize) -> Result<Vec<Workspace>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, root_path, remote_url, profile, created_at, last_opened \
             FROM workspaces ORDER BY last_opened DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(Workspace {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    root_path: r.get(2)?,
                    remote_url: r.get(3)?,
                    profile: r.get(4)?,
                    created_at: r.get(5)?,
                    last_opened: r.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Set `last_opened` on the given workspace to `now_ms()`. No-op if the
    /// id doesn't exist. Returns Err only on DB failure.
    pub fn touch(&self, id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE workspaces SET last_opened = ?1 WHERE id = ?2",
            params![now_ms(), id],
        )?;
        Ok(())
    }
}
