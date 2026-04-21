use agentic_core::db::Db;
use agentic_core::Paths;

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
