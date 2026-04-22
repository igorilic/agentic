CREATE TABLE auth_accounts (
    id             TEXT PRIMARY KEY,                 -- e.g., 'github:github.com', 'gitlab:gitlab.com', 'jira:mycompany.atlassian.net'
    provider       TEXT NOT NULL,                    -- github | gitlab | jira | claude | copilot
    host           TEXT NOT NULL,
    username       TEXT,
    client_id      TEXT,                             -- for GHES BYO client ID
    -- Secrets: tokens stored in keychain keyed by this id; not in DB.
    token_expires_at INTEGER,
    created_at     INTEGER NOT NULL,
    last_used_at   INTEGER
);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,                           -- json
    scope      TEXT NOT NULL CHECK (scope = 'user' OR scope LIKE 'workspace:_%'),
    updated_at INTEGER NOT NULL
);
