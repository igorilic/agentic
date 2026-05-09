//! Auth-account repository.
//!
//! Persists *metadata* about auth accounts (id, provider, host, username,
//! timestamps) to the `auth_accounts` table. **Tokens are NEVER stored in
//! this table** — they live in the OS keychain via
//! [`crate::auth::SecretStore`], keyed by the account id. This keeps
//! secrets off-disk while letting the UI list accounts without unlocking
//! the keychain.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthAccount {
    /// Composite identifier — convention: `<provider>:<host>`. Examples:
    /// `github:github.com`, `gitlab:gitlab.com`, `github:ghe.example.com`.
    pub id: String,
    /// One of [`ALLOWED_PROVIDERS`].
    pub provider: String,
    pub host: String,
    pub username: Option<String>,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

/// Allowed `provider` values. Mirrors the comment in migration 0006.
pub const ALLOWED_PROVIDERS: &[&str] = &["github", "gitlab", "jira", "claude", "copilot"];

pub struct AuthRepo {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl AuthRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    /// Insert an account row. Returns the inserted row on success. Returns
    /// `Err` if `provider` is not one of [`ALLOWED_PROVIDERS`].
    pub fn insert(&self, acc: &AuthAccount) -> Result<AuthAccount> {
        if !ALLOWED_PROVIDERS.contains(&acc.provider.as_str()) {
            return Err(CoreError::Parse(format!(
                "invalid auth_accounts.provider value: {:?} (allowed: {ALLOWED_PROVIDERS:?})",
                acc.provider
            )));
        }
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO auth_accounts \
             (id, provider, host, username, created_at, last_used_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                acc.id,
                acc.provider,
                acc.host,
                acc.username,
                acc.created_at,
                acc.last_used_at,
            ],
        )?;
        Ok(acc.clone())
    }

    /// List all accounts in `created_at` ascending order.
    pub fn list(&self) -> Result<Vec<AuthAccount>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, provider, host, username, \
                    created_at, last_used_at \
             FROM auth_accounts \
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map([], row_to_account)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn get(&self, id: &str) -> Result<Option<AuthAccount>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, provider, host, username, \
                    created_at, last_used_at \
             FROM auth_accounts \
             WHERE id = ?1",
        )?;
        let mut iter = stmt.query_map(params![id], row_to_account)?;
        match iter.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    /// Delete an account row. Returns `true` if a row was deleted, `false`
    /// for missing id (so re-deletes are idempotent for the caller).
    /// Callers SHOULD also delete the keychain entry keyed by this id.
    pub fn delete(&self, id: &str) -> Result<bool> {
        let conn = self.pool.get()?;
        let n = conn.execute("DELETE FROM auth_accounts WHERE id = ?1", params![id])?;
        Ok(n > 0)
    }

    /// Update `last_used_at` for an account. Returns `true` if a row was
    /// updated, `false` if the id doesn't exist.
    pub fn touch_last_used(&self, id: &str, ts_ms: i64) -> Result<bool> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "UPDATE auth_accounts SET last_used_at = ?1 WHERE id = ?2",
            params![ts_ms, id],
        )?;
        Ok(n > 0)
    }
}

fn row_to_account(r: &rusqlite::Row<'_>) -> rusqlite::Result<AuthAccount> {
    Ok(AuthAccount {
        id: r.get(0)?,
        provider: r.get(1)?,
        host: r.get(2)?,
        username: r.get(3)?,
        created_at: r.get(4)?,
        last_used_at: r.get(5)?,
    })
}
