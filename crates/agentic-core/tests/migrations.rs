use agentic_core::Paths;
use agentic_core::db::{Db, migrations::Migrator};

fn setup() -> (tempfile::TempDir, Paths, Db) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    (tmp, paths, db)
}

fn has_table(db: &Db, name: &str) -> bool {
    let conn = db.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |r| r.get(0),
        )
        .unwrap();
    count == 1
}

fn has_index(db: &Db, name: &str) -> bool {
    let conn = db.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name=?1",
            [name],
            |r| r.get(0),
        )
        .unwrap();
    count == 1
}

#[test]
fn migrator_creates_migrations_and_workspaces_tables() {
    let (_tmp, _paths, db) = setup();
    // Note: Db::open already runs the migrator in GREEN, so tables should be
    // present immediately. Calling run() again should be a no-op.
    Migrator::run(&db).expect("run migrations");
    assert!(has_table(&db, "_migrations"), "_migrations table missing");
    assert!(has_table(&db, "workspaces"), "workspaces table missing");
}

#[test]
fn migrator_is_idempotent_when_run_twice() {
    let (_tmp, _paths, db) = setup();
    Migrator::run(&db).expect("first run");
    Migrator::run(&db).expect("second run should be a no-op");
    // Count _migrations rows — must be 2 (versions 1 and 2 applied, not duplicated)
    let conn = db.conn().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 2,
        "_migrations should have exactly 2 rows, not {count}"
    );
}

#[test]
fn each_applied_migration_has_a_row_in_migrations_table() {
    let (_tmp, _paths, db) = setup();
    let conn = db.conn().unwrap();
    let versions: Vec<i64> = conn
        .prepare("SELECT version FROM _migrations ORDER BY version")
        .unwrap()
        .query_map([], |r| r.get::<_, i64>(0))
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        versions,
        vec![1, 2],
        "expected exactly versions 1 and 2 applied"
    );
    let applied_at: i64 = conn
        .query_row(
            "SELECT applied_at FROM _migrations WHERE version = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        applied_at > 0,
        "applied_at should be a positive unix timestamp"
    );
}

#[test]
fn workspaces_schema_matches_spec() {
    let (_tmp, _paths, db) = setup();
    let conn = db.conn().unwrap();
    // PRAGMA table_info returns: (cid, name, type, notnull, dflt_value, pk)
    let cols: Vec<(String, String, bool)> = conn
        .prepare("PRAGMA table_info(workspaces)")
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,   // name
                row.get::<_, String>(2)?,   // declared type
                row.get::<_, i64>(3)? == 1, // notnull
            ))
        })
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    // Spec §13.1 order and shape. Note: `id TEXT PRIMARY KEY` without explicit
    // NOT NULL — SQLite quirk allows NULL for non-INTEGER PK, so notnull is 0.
    let expected: Vec<(&str, &str, bool)> = vec![
        ("id", "TEXT", false),
        ("name", "TEXT", true),
        ("root_path", "TEXT", true),
        ("remote_url", "TEXT", false),
        ("profile", "TEXT", true),
        ("created_at", "INTEGER", true),
        ("last_opened", "INTEGER", true),
    ];
    let actual_ref: Vec<(&str, &str, bool)> = cols
        .iter()
        .map(|(n, t, nn)| (n.as_str(), t.as_str(), *nn))
        .collect();
    assert_eq!(actual_ref, expected);
}

#[test]
fn inserting_run_with_missing_workspace_id_fails_fk() {
    let (_tmp, _paths, db) = setup();
    let conn = db.conn().unwrap();
    let result = conn.execute(
        "INSERT INTO runs \
         (id, workspace_id, pipeline_name, status, backend, model, started_at) \
         VALUES ('run1', 'nonexistent-workspace', 'default', 'pending', \
                 'claude-code', 'claude-opus-4-7', 123)",
        [],
    );
    match result {
        Ok(_) => panic!("expected FK violation for missing workspace_id"),
        Err(e) => {
            let msg = e.to_string().to_uppercase();
            assert!(
                msg.contains("FOREIGN KEY") || msg.contains("CONSTRAINT"),
                "expected FK/constraint error, got: {e}"
            );
        }
    }
}

#[test]
fn deleting_run_cascades_to_run_steps() {
    let (_tmp, _paths, db) = setup();
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES ('ws1', 'test', '/tmp/test', 'github', 100, 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO runs \
         (id, workspace_id, pipeline_name, status, backend, model, started_at) \
         VALUES ('run1', 'ws1', 'default', 'pending', 'claude-code', 'claude-opus-4-7', 200)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO run_steps (id, run_id, seq, agent_name, status) \
         VALUES ('step1', 'run1', 1, 'architect', 'pending')",
        [],
    )
    .unwrap();
    let pre: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM run_steps WHERE run_id='run1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pre, 1);
    conn.execute("DELETE FROM runs WHERE id='run1'", [])
        .unwrap();
    let post: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM run_steps WHERE run_id='run1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        post, 0,
        "run_steps should cascade-delete when run is deleted"
    );
}

#[test]
fn runs_indexes_exist() {
    let (_tmp, _paths, db) = setup();
    assert!(
        has_index(&db, "idx_runs_workspace_status"),
        "idx_runs_workspace_status missing"
    );
    assert!(
        has_index(&db, "idx_runs_started_at"),
        "idx_runs_started_at missing"
    );
}

#[test]
fn run_steps_index_exists() {
    let (_tmp, _paths, db) = setup();
    assert!(
        has_index(&db, "idx_run_steps_run_seq"),
        "idx_run_steps_run_seq missing"
    );
}
