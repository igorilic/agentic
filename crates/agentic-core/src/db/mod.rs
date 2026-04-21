use crate::{Paths, Result};

pub mod migrations;

#[derive(Clone)]
pub struct Db {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl Db {
    /// Open the on-disk database at `paths.db_file()`.
    /// Caller is responsible for ensuring the parent directory exists
    /// (typically via `paths.ensure_dirs()` first).
    pub fn open(paths: &Paths) -> Result<Db> {
        let manager =
            r2d2_sqlite::SqliteConnectionManager::file(paths.db_file()).with_init(apply_pragmas);
        let pool = r2d2::Pool::builder().build(manager)?;
        let db = Db { pool };
        migrations::Migrator::run(&db)?;
        Ok(db)
    }

    /// In-memory database for tests that don't need on-disk persistence
    /// or WAL semantics. Pool max size is 1 because each r2d2_sqlite
    /// in-memory connection is independent by default.
    pub fn open_in_memory() -> Result<Db> {
        let manager = r2d2_sqlite::SqliteConnectionManager::memory().with_init(apply_pragmas);
        let pool = r2d2::Pool::builder().max_size(1).build(manager)?;
        let db = Db { pool };
        migrations::Migrator::run(&db)?;
        Ok(db)
    }

    /// Borrow a pooled connection. Returns `CoreError::Db` if the pool
    /// is exhausted or a connection can't be established.
    pub fn conn(&self) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>> {
        Ok(self.pool.get()?)
    }
}

/// Apply the pragmas every pooled connection needs. WAL persists in
/// the db file after the first setting; foreign_keys is per-connection
/// and must be re-applied for every connection the pool hands out.
fn apply_pragmas(conn: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "wal")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    Ok(())
}
