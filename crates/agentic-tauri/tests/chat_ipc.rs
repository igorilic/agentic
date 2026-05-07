#![cfg(test)]

use agentic_core::Db;
use agentic_tauri::commands::chat::{ChatState, chat_list_messages, chat_record_system_message, chat_send_message};
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

fn seed_workspace(db: &Db, id: &str) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'github', 100, 100)",
        [id],
    )
    .unwrap();
}

fn build_app() -> tauri::App<tauri::test::MockRuntime> {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    seed_workspace(&db, "default");
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::chat::chat_send_message,
            agentic_tauri::commands::chat::chat_list_messages,
            agentic_tauri::commands::chat::chat_record_system_message,
        ])
        .manage(ChatState::new(&db))
        .build(mock_context(noop_assets()))
        .expect("build mock app")
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_send_message_persists_user_and_reply() {
    let app = build_app();
    let state = app.state::<ChatState>();

    let result = chat_send_message(state, None, "default".to_string(), "hello".to_string())
        .await
        .expect("chat_send_message");

    assert_eq!(result.user_message.role, "user");
    assert_eq!(result.user_message.content, "hello");
    assert_eq!(result.reply.role, "assistant");
    assert!(result.reply.content.contains("hello"));

    // Verify both messages were persisted.
    let session_id = result.user_message.session_id.clone();
    let state2 = app.state::<ChatState>();
    let messages = chat_list_messages(state2, session_id)
        .await
        .expect("chat_list_messages");
    assert_eq!(messages.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_send_message_creates_session_when_none_provided() {
    let app = build_app();
    let state = app.state::<ChatState>();

    let result = chat_send_message(
        state,
        None,
        "default".to_string(),
        "test message".to_string(),
    )
    .await
    .expect("chat_send_message");

    // The user message should have a valid session_id.
    assert!(!result.user_message.session_id.is_empty());
    // The reply must belong to the same session.
    assert_eq!(result.user_message.session_id, result.reply.session_id);
}

#[tokio::test(flavor = "multi_thread")]
async fn chat_send_message_rejects_empty_content() {
    let app = build_app();
    let state = app.state::<ChatState>();

    let result = chat_send_message(state, None, "default".to_string(), "   ".to_string()).await;

    assert!(result.is_err(), "expected Err for empty content");
    let err = result.unwrap_err();
    assert!(
        err.contains("empty"),
        "error message should mention empty: {err}"
    );
}

// ---------------------------------------------------------------------------
// SR1 — chat_record_system_message persists a role="system" row
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn chat_record_system_message_persists_system_role_row() {
    let app = build_app();

    // First create a session via chat_send_message so we have a valid session_id.
    let state = app.state::<ChatState>();
    let send_result = chat_send_message(state, None, "default".to_string(), "seed".to_string())
        .await
        .expect("chat_send_message seed");
    let session_id = send_result.user_message.session_id.clone();

    // Record a system message into that session.
    let state2 = app.state::<ChatState>();
    let sys_msg = chat_record_system_message(
        state2,
        Some(session_id.clone()),
        "default".to_string(),
        "pre-flight: `claude` not found on PATH".to_string(),
    )
    .await
    .expect("chat_record_system_message");

    assert_eq!(sys_msg.role, "system");
    assert_eq!(sys_msg.content, "pre-flight: `claude` not found on PATH");
    assert_eq!(sys_msg.session_id, session_id);

    // Verify it was persisted: list_messages should contain it.
    let state3 = app.state::<ChatState>();
    let messages = chat_list_messages(state3, session_id)
        .await
        .expect("chat_list_messages");
    assert!(
        messages.iter().any(|m| m.role == "system" && m.content.contains("pre-flight")),
        "expected a system-role message in the persisted list"
    );
}

// ---------------------------------------------------------------------------
// SR2 — chat_record_system_message rejects empty content
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn chat_record_system_message_rejects_empty_content() {
    let app = build_app();
    let state = app.state::<ChatState>();

    let result = chat_record_system_message(
        state,
        None,
        "default".to_string(),
        "   ".to_string(),
    )
    .await;

    assert!(result.is_err(), "expected Err for empty content");
    let err = result.unwrap_err();
    assert_eq!(err, "content is empty", "error must be 'content is empty': {err}");
}

// ---------------------------------------------------------------------------
// SR3 — chat_record_system_message creates a new session when session_id is None
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn chat_record_system_message_creates_session_when_none() {
    let app = build_app();
    let state = app.state::<ChatState>();

    let sys_msg = chat_record_system_message(
        state,
        None,
        "default".to_string(),
        "slash command audit".to_string(),
    )
    .await
    .expect("chat_record_system_message with None session");

    // A fresh, non-empty session_id must be assigned.
    assert!(!sys_msg.session_id.is_empty(), "session_id must be non-empty");

    // The session row must exist so list_messages can find the persisted message.
    let state2 = app.state::<ChatState>();
    let messages = chat_list_messages(state2, sys_msg.session_id.clone())
        .await
        .expect("chat_list_messages for new session");
    assert_eq!(messages.len(), 1, "new session should have exactly one system message");
    assert_eq!(messages[0].role, "system");
}
