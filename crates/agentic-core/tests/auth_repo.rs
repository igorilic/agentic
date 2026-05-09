use agentic_core::Db;
use agentic_core::db::auth::{AuthAccount, AuthRepo};

fn setup() -> (Db, AuthRepo) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    let repo = AuthRepo::new(&db);
    (db, repo)
}

fn sample(id: &str, provider: &str, host: &str) -> AuthAccount {
    AuthAccount {
        id: id.to_string(),
        provider: provider.to_string(),
        host: host.to_string(),
        username: Some("octocat".to_string()),
        created_at: 100,
        last_used_at: None,
    }
}

#[test]
fn insert_returns_the_inserted_row_and_persists() {
    let (_db, repo) = setup();

    let inserted = repo
        .insert(&sample("github:github.com", "github", "github.com"))
        .expect("insert");

    assert_eq!(inserted.id, "github:github.com");
    assert_eq!(inserted.provider, "github");

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "github:github.com");
}

#[test]
fn list_returns_accounts_in_creation_order() {
    let (_db, repo) = setup();

    let mut a = sample("a:host", "github", "github.com");
    a.created_at = 100;
    let mut b = sample("b:host", "gitlab", "gitlab.com");
    b.created_at = 200;
    let mut c = sample("c:host", "github", "ghe.example");
    c.created_at = 50; // earliest

    repo.insert(&b).unwrap();
    repo.insert(&a).unwrap();
    repo.insert(&c).unwrap();

    let list = repo.list().expect("list");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, "c:host"); // 50
    assert_eq!(list[1].id, "a:host"); // 100
    assert_eq!(list[2].id, "b:host"); // 200
}

#[test]
fn get_returns_some_for_existing_and_none_for_missing() {
    let (_db, repo) = setup();
    repo.insert(&sample("github:github.com", "github", "github.com"))
        .unwrap();

    let found = repo.get("github:github.com").expect("get");
    assert!(found.is_some());
    assert_eq!(found.unwrap().provider, "github");

    let missing = repo.get("nope:nope").expect("get");
    assert!(missing.is_none());
}

#[test]
fn delete_returns_true_for_existing_and_false_for_missing() {
    let (_db, repo) = setup();
    repo.insert(&sample("github:github.com", "github", "github.com"))
        .unwrap();

    let deleted = repo.delete("github:github.com").expect("delete");
    assert!(deleted, "expected true for existing row");
    assert!(repo.list().unwrap().is_empty());

    let deleted_again = repo.delete("github:github.com").expect("delete");
    assert!(
        !deleted_again,
        "expected false for missing row (idempotent)"
    );
}

#[test]
fn touch_last_used_updates_timestamp_and_returns_true_for_existing() {
    let (_db, repo) = setup();
    repo.insert(&sample("github:github.com", "github", "github.com"))
        .unwrap();

    let touched = repo
        .touch_last_used("github:github.com", 999)
        .expect("touch_last_used");
    assert!(touched);

    let row = repo
        .get("github:github.com")
        .unwrap()
        .expect("row should still exist");
    assert_eq!(row.last_used_at, Some(999));
}

#[test]
fn insert_rejects_unknown_provider() {
    // Defense-in-depth: provider should be one of the documented values.
    // If we let arbitrary strings in, the UI's provider-routing logic
    // (e.g., which OAuth client to use) would break silently.
    let (_db, repo) = setup();
    let mut bad = sample("foo:bar", "made-up-provider", "example.com");
    bad.provider = "made-up-provider".to_string();
    let result = repo.insert(&bad);
    assert!(result.is_err(), "expected error for unknown provider");
}

#[test]
fn migration_0009_drops_client_id_and_token_expires_at_columns() {
    // After migration 0009 the auth_accounts table must no longer have
    // client_id or token_expires_at columns — they were only used by the
    // OAuth flows deleted in Stages 1+2 of the auth-refactor.
    let (db, _repo) = setup();
    let conn = db.conn().expect("conn");
    let mut stmt = conn
        .prepare("PRAGMA table_info(auth_accounts)")
        .expect("prepare pragma");
    let columns: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))
        .expect("query_map")
        .map(|r| r.expect("column name"))
        .collect();

    assert!(
        !columns.contains(&"client_id".to_string()),
        "client_id column should not exist after migration 0009; found columns: {columns:?}"
    );
    assert!(
        !columns.contains(&"token_expires_at".to_string()),
        "token_expires_at column should not exist after migration 0009; found columns: {columns:?}"
    );

    // Verify the expected surviving columns are still present.
    for expected in &[
        "id",
        "provider",
        "host",
        "username",
        "created_at",
        "last_used_at",
    ] {
        assert!(
            columns.contains(&expected.to_string()),
            "expected column {expected} to survive migration 0009; found: {columns:?}"
        );
    }
}
