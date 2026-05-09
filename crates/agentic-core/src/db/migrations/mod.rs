use crate::Result;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "workspaces",
        sql: include_str!("0001_workspaces.sql"),
    },
    Migration {
        version: 2,
        name: "runs_and_steps",
        sql: include_str!("0002_runs_and_steps.sql"),
    },
    Migration {
        version: 3,
        name: "artifacts",
        sql: include_str!("0003_artifacts.sql"),
    },
    Migration {
        version: 4,
        name: "stream_events",
        sql: include_str!("0004_stream_events.sql"),
    },
    Migration {
        version: 5,
        name: "chat",
        sql: include_str!("0005_chat.sql"),
    },
    Migration {
        version: 6,
        name: "auth_and_settings",
        sql: include_str!("0006_auth_and_settings.sql"),
    },
    Migration {
        version: 7,
        name: "findings_triage_check",
        sql: include_str!("0007_findings_triage_check.sql"),
    },
    Migration {
        version: 8,
        name: "findings_composite_pk",
        sql: include_str!("0008_findings_composite_pk.sql"),
    },
    Migration {
        version: 9,
        name: "drop_oauth_columns",
        sql: include_str!("0009_drop_oauth_columns.sql"),
    },
];

pub struct Migrator;

impl Migrator {
    /// Run all pending migrations against `db`. Idempotent: already-applied
    /// migrations (by version) are skipped.
    pub fn run(db: &super::Db) -> Result<()> {
        let mut conn = db.conn()?;
        let tx = conn.transaction()?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migrations (
                version    INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
             );",
        )?;
        let current: i64 = tx.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _migrations",
            [],
            |r| r.get(0),
        )?;
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        for m in MIGRATIONS.iter().filter(|m| m.version > current) {
            tx.execute_batch(m.sql)?;
            tracing::info!(version = m.version, name = m.name, "applied migration");
            tx.execute(
                "INSERT INTO _migrations (version, applied_at) VALUES (?1, ?2)",
                rusqlite::params![m.version, now_secs],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
