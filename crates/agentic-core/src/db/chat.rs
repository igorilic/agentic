//! Chat sessions + chat messages repository.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;
use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub run_id: Option<String>,
    pub role: String,
    pub content: String,
    pub metadata: Option<String>,
    pub created_at: i64,
}

pub struct ChatRepo {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl ChatRepo {
    pub fn new(db: &Db) -> Self {
        Self { pool: db.pool() }
    }

    /// Create a chat session row.
    pub fn create_session(&self, id: &str, workspace_id: &str, created_at: i64) -> Result<()> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO chat_sessions (id, workspace_id, title, created_at, last_message_at) \
             VALUES (?1, ?2, NULL, ?3, NULL)",
            params![id, workspace_id, created_at],
        )?;
        Ok(())
    }

    /// Insert a message. Returns it.
    pub fn insert_message(&self, msg: ChatMessage) -> Result<ChatMessage> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO chat_messages \
             (id, session_id, run_id, role, content, metadata, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                msg.id,
                msg.session_id,
                msg.run_id,
                msg.role,
                msg.content,
                msg.metadata,
                msg.created_at,
            ],
        )?;
        Ok(msg)
    }

    /// List messages for a session in chronological order.
    pub fn list_by_session(&self, session_id: &str) -> Result<Vec<ChatMessage>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, session_id, run_id, role, content, metadata, created_at \
             FROM chat_messages \
             WHERE session_id = ?1 \
             ORDER BY created_at ASC",
        )?;
        let rows = stmt
            .query_map(params![session_id], |row| {
                Ok(ChatMessage {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    run_id: row.get(2)?,
                    role: row.get(3)?,
                    content: row.get(4)?,
                    metadata: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}
