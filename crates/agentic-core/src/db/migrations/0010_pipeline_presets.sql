-- Pipeline presets: named, ordered collections of agents the user can save/load.
-- Name is unique; the order of `agents` is significant (stored as JSON array).
CREATE TABLE pipeline_presets (
    id         TEXT PRIMARY KEY,             -- ULID
    name       TEXT NOT NULL UNIQUE,
    agents     TEXT NOT NULL,                -- JSON array of agent ids in order
    created_at INTEGER NOT NULL,             -- unix epoch ms
    updated_at INTEGER NOT NULL
);
