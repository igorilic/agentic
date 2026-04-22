CREATE TABLE runs (
    id             TEXT PRIMARY KEY,       -- ulid
    workspace_id   TEXT NOT NULL REFERENCES workspaces(id),
    pipeline_name  TEXT NOT NULL DEFAULT 'default',
    status         TEXT NOT NULL,          -- pending | running | completed | completed_with_tech_debt | failed | cancelled | crashed
    ticket_type    TEXT,                   -- 'github-issue' | 'gitlab-issue' | 'jira' | 'free-text'
    ticket_ref     TEXT,                   -- #42, PROJ-123, or free-text hash
    ticket_title   TEXT,
    ticket_body    TEXT,                   -- snapshotted at run start
    backend        TEXT NOT NULL,          -- 'claude-code' | 'copilot-cli'
    model          TEXT NOT NULL,
    started_at     INTEGER NOT NULL,
    completed_at   INTEGER,
    duration_ms    INTEGER,
    token_usage    TEXT,                   -- json
    cost_usd       REAL,
    summary        TEXT,
    subprocess_pid INTEGER                 -- for crash detection
);

CREATE INDEX idx_runs_workspace_status ON runs(workspace_id, status);
CREATE INDEX idx_runs_started_at       ON runs(started_at DESC);

CREATE TABLE run_steps (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    seq          INTEGER NOT NULL,
    agent_name   TEXT NOT NULL,
    status       TEXT NOT NULL,           -- pending | running | passed | failed | needs_triage | skipped
    started_at   INTEGER,
    completed_at INTEGER,
    duration_ms  INTEGER,
    token_usage  TEXT,                    -- json
    cost_usd     REAL,
    summary      TEXT,
    retry_count  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_run_steps_run_seq ON run_steps(run_id, seq);
