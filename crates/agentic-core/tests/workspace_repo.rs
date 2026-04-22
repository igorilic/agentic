use agentic_core::{Db, Paths, Workspace, WorkspaceRepo};

fn setup() -> (tempfile::TempDir, WorkspaceRepo) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let repo = WorkspaceRepo::new(&db);
    (tmp, repo)
}

fn sample(id: &str, name: &str, last_opened: i64) -> Workspace {
    Workspace {
        id: id.to_string(),
        name: name.to_string(),
        root_path: format!("/tmp/{name}"),
        remote_url: Some(format!("https://example.com/{name}.git")),
        profile: "github".to_string(),
        created_at: 100,
        last_opened,
    }
}

#[test]
fn insert_returns_the_inserted_workspace() {
    let (_tmp, repo) = setup();
    let ws = sample("ws-a", "alpha", 1000);
    let inserted = repo.insert(ws.clone()).expect("insert");
    assert_eq!(inserted, ws);
    let fetched = repo.get("ws-a").expect("get").expect("some");
    assert_eq!(fetched, ws);
}

#[test]
fn get_by_unknown_id_returns_none() {
    let (_tmp, repo) = setup();
    let result = repo.get("nonexistent").expect("get");
    assert!(result.is_none(), "expected None for unknown id, got {result:?}");
}

#[test]
fn list_recent_ordered_by_last_opened_desc() {
    let (_tmp, repo) = setup();
    repo.insert(sample("ws-a", "alpha", 100)).unwrap();
    repo.insert(sample("ws-b", "beta", 300)).unwrap();
    repo.insert(sample("ws-c", "charlie", 200)).unwrap();
    let recent = repo.list_recent(10).expect("list");
    let ids: Vec<String> = recent.into_iter().map(|w| w.id).collect();
    assert_eq!(ids, vec!["ws-b".to_string(), "ws-c".to_string(), "ws-a".to_string()]);
}

#[test]
fn touch_updates_last_opened() {
    let (_tmp, repo) = setup();
    repo.insert(sample("ws-a", "alpha", 100)).unwrap();
    let before = repo.get("ws-a").unwrap().unwrap();
    assert_eq!(before.last_opened, 100);
    // Sleep briefly so now_ms > 100. 2ms is enough on any modern platform.
    std::thread::sleep(std::time::Duration::from_millis(2));
    repo.touch("ws-a").expect("touch");
    let after = repo.get("ws-a").unwrap().unwrap();
    assert!(
        after.last_opened > before.last_opened,
        "expected last_opened to increase after touch: {} -> {}",
        before.last_opened, after.last_opened
    );
}

#[test]
fn compute_id_is_deterministic_and_32_hex_chars() {
    let a = Workspace::compute_id(Some("https://example.com/a.git"), "/Users/x/repos/a");
    let b = Workspace::compute_id(Some("https://example.com/a.git"), "/Users/x/repos/a");
    assert_eq!(a, b, "same inputs must produce same id");
    assert_eq!(a.len(), 32, "id must be 32 hex chars (16 bytes)");
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()), "id must be hex");
    let c = Workspace::compute_id(None, "/Users/x/repos/a");
    assert_ne!(a, c, "remote_url absence must change the id");
}
