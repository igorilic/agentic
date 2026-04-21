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
    // Count _migrations rows — must be 1 (only version 1 applied, not duplicated)
    let conn = db.conn().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM _migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 1,
        "_migrations should have exactly 1 row, not {count}"
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
    assert_eq!(versions, vec![1], "expected exactly version 1 applied");
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
