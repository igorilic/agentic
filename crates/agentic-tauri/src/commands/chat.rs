use std::sync::Arc;

use agentic_core::db::Db;
use agentic_core::db::chat::{ChatMessage, ChatRepo};
use serde::Serialize;
use tauri::State;
use ulid::Ulid;

/// State holding the DB handle for chat commands. Distinct from EventBusState
/// because chat ops don't need the bus.
pub struct ChatState {
    pub db: Arc<Db>,
    pub repo: ChatRepo,
}

impl ChatState {
    pub fn new(db: Arc<Db>) -> Self {
        let repo = ChatRepo::new(&db);
        Self { db, repo }
    }
}

#[derive(Debug, Serialize)]
pub struct ChatSendResult {
    /// The user's message that was just persisted.
    pub user_message: ChatMessage,
    /// A stub assistant echo (Phase 11 MVP — no LLM yet).
    pub reply: ChatMessage,
}

/// Send a chat message. If `session_id` is None, creates a new session.
/// Persists the user message AND a stub assistant echo so the UI sees a reply.
/// Phase 11+ will replace the stub with a real LLM dispatch.
#[tauri::command]
pub async fn chat_send_message(
    state: State<'_, ChatState>,
    session_id: Option<String>,
    workspace_id: String,
    content: String,
) -> Result<ChatSendResult, String> {
    if content.trim().is_empty() {
        return Err("content is empty".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let session_id = match session_id {
        Some(id) => id,
        None => {
            let new_id = Ulid::new().to_string().to_lowercase();
            state
                .repo
                .create_session(&new_id, &workspace_id, now)
                .map_err(|e| e.to_string())?;
            new_id
        }
    };

    let user_msg = state
        .repo
        .insert_message(ChatMessage {
            id: Ulid::new().to_string().to_lowercase(),
            session_id: session_id.clone(),
            run_id: None,
            role: "user".to_string(),
            content,
            metadata: None,
            created_at: now,
        })
        .map_err(|e| e.to_string())?;

    let reply = state
        .repo
        .insert_message(ChatMessage {
            id: Ulid::new().to_string().to_lowercase(),
            session_id,
            run_id: None,
            role: "assistant".to_string(),
            content: format!("Echo: {}", user_msg.content),
            metadata: None,
            created_at: now + 1,
        })
        .map_err(|e| e.to_string())?;

    Ok(ChatSendResult {
        user_message: user_msg,
        reply,
    })
}

/// List messages for an existing session.
#[tauri::command]
pub async fn chat_list_messages(
    state: State<'_, ChatState>,
    session_id: String,
) -> Result<Vec<ChatMessage>, String> {
    state
        .repo
        .list_by_session(&session_id)
        .map_err(|e| e.to_string())
}
