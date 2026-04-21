use agentic_core::db::Db;
use agentic_core::{CoreError, Paths};

fn setup_paths() -> (tempfile::TempDir, Paths) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().expect("ensure_dirs");
    (tmp, paths)
}

#[test]
fn open_fresh_db_in_tempdir_succeeds() {
    let (_tmp, paths) = setup_paths();
    let db = Db::open(&paths).expect("open");
    let _conn = db.conn().expect("conn");
    assert!(
        paths.db_file().exists(),
        "db file {:?} should exist after Db::open",
        paths.db_file()
    );
}

#[test]
fn journal_mode_is_wal() {
    let (_tmp, paths) = setup_paths();
    let db = Db::open(&paths).expect("open");
    let conn = db.conn().expect("conn");
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .expect("query journal_mode");
    assert_eq!(mode.to_lowercase(), "wal");
}

#[test]
fn foreign_keys_pragma_is_on() {
    let (_tmp, paths) = setup_paths();
    let db = Db::open(&paths).expect("open");
    let conn = db.conn().expect("conn");
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .expect("query foreign_keys");
    assert_eq!(fk, 1);
}

#[test]
fn two_opens_on_same_path_succeed() {
    let (_tmp, paths) = setup_paths();
    let db1 = Db::open(&paths).expect("first open");
    let db2 = Db::open(&paths).expect("second open");
    let c1 = db1.conn().expect("conn1");
    let c2 = db2.conn().expect("conn2");
    let v1: i64 = c1.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
    let v2: i64 = c2.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
    assert_eq!(v1, 1);
    assert_eq!(v2, 1);
}

#[test]
fn open_fails_when_parent_dir_missing() {
    // Exercises the From<r2d2::Error> impl in error.rs.
    // Db::open's contract is "caller ensures dirs"; this test documents the
    // failure mode when the parent directory does not exist.
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().join("nonexistent");
    let paths = Paths::for_tests(&base);
    // Intentionally skip ensure_dirs so the parent directory is absent.
    let result = Db::open(&paths);
    match result {
        Err(CoreError::Db(msg)) => {
            assert!(!msg.is_empty(), "Db error message should be non-empty");
        }
        Ok(_) => panic!("expected Err(CoreError::Db), got Ok"),
        Err(other) => panic!("expected Err(CoreError::Db), got {other:?}"),
    }
}
