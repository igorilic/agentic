use agentic_core::db::chat::{ChatMessage, ChatRepo};
use agentic_core::Db;

fn setup_in_memory() -> (Db, ChatRepo) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    seed_workspace(&db, "ws1");
    let repo = ChatRepo::new(&db);
    (db, repo)
}

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        rusqlite::params![id],
    )
    .unwrap();
}

fn sample_message(
    id: &str,
    session_id: &str,
    role: &str,
    content: &str,
    created_at: i64,
) -> ChatMessage {
    ChatMessage {
        id: id.to_string(),
        session_id: session_id.to_string(),
        run_id: None,
        role: role.to_string(),
        content: content.to_string(),
        metadata: None,
        created_at,
    }
}

#[test]
fn insert_message_returns_message_and_persists() {
    let (_db, repo) = setup_in_memory();
    repo.create_session("sess1", "ws1", 100)
        .expect("create_session");

    let msg = sample_message("msg1", "sess1", "user", "hello", 200);
    let returned = repo.insert_message(msg.clone()).expect("insert_message");

    assert_eq!(returned.id, "msg1");
    assert_eq!(returned.session_id, "sess1");
    assert_eq!(returned.role, "user");
    assert_eq!(returned.content, "hello");
    assert_eq!(returned.created_at, 200);

    // Verify it was actually persisted by listing.
    let list = repo.list_by_session("sess1").expect("list_by_session");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "msg1");
}

#[test]
fn list_by_session_returns_messages_in_chronological_order() {
    let (_db, repo) = setup_in_memory();
    repo.create_session("sess2", "ws1", 100)
        .expect("create_session");

    // Insert out of order by timestamp.
    repo.insert_message(sample_message("msg3", "sess2", "assistant", "third", 300))
        .unwrap();
    repo.insert_message(sample_message("msg1", "sess2", "user", "first", 100))
        .unwrap();
    repo.insert_message(sample_message("msg2", "sess2", "assistant", "second", 200))
        .unwrap();

    let list = repo.list_by_session("sess2").expect("list_by_session");
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].id, "msg1");
    assert_eq!(list[1].id, "msg2");
    assert_eq!(list[2].id, "msg3");
}

#[test]
fn list_by_session_returns_empty_for_unknown_session() {
    let (_db, repo) = setup_in_memory();

    let list = repo
        .list_by_session("no-such-session")
        .expect("list_by_session");
    assert!(list.is_empty(), "expected empty vec for unknown session");
}
