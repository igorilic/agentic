#![cfg(test)]

use std::sync::Arc;

use agentic_core::Db;
use agentic_tauri::commands::chat::{ChatState, chat_list_messages, chat_send_message};
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
    let db = Arc::new(Db::open_in_memory().expect("Db::open_in_memory"));
    seed_workspace(&db, "default");
    mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::chat::chat_send_message,
            agentic_tauri::commands::chat::chat_list_messages,
        ])
        .manage(ChatState::new(db))
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
