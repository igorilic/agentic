CREATE TABLE workspaces (
    id            TEXT PRIMARY KEY,        -- workspace_id (blake3 hash prefix)
    name          TEXT NOT NULL,           -- last folder component
    root_path     TEXT NOT NULL,           -- last known canonical path
    remote_url    TEXT,                    -- canonical git remote
    profile       TEXT NOT NULL,           -- 'github' | 'gitlab' | 'custom'
    created_at    INTEGER NOT NULL,
    last_opened   INTEGER NOT NULL
);
