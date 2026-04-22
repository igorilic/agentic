CREATE TABLE chat_sessions (
    id            TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces(id),
    title         TEXT,
    created_at    INTEGER NOT NULL,
    last_message_at INTEGER
);

CREATE TABLE chat_messages (
    id           TEXT PRIMARY KEY,
    session_id   TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE,
    run_id       TEXT REFERENCES runs(id),           -- null if pure chat
    role         TEXT NOT NULL,                      -- user | assistant | system | tool
    content      TEXT NOT NULL,                      -- markdown body
    metadata     TEXT,                               -- json (tool calls, citations, etc.)
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_chat_messages_session_ts ON chat_messages(session_id, created_at);
