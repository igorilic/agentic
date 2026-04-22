CREATE TABLE findings (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id      TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    severity     TEXT NOT NULL,
    file_path    TEXT,
    line         INTEGER,
    message      TEXT NOT NULL,
    suggestion   TEXT,
    triage       TEXT,                     -- null | 'fix' | 'tech-debt' | 'ignore'
    triaged_at   INTEGER,
    created_at   INTEGER NOT NULL
);

CREATE INDEX idx_findings_run_triage ON findings(run_id, triage);

CREATE TABLE clarifying_questions (
    id              TEXT PRIMARY KEY,
    run_id          TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id         TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    question        TEXT NOT NULL,
    suggested_answers TEXT,                -- json array
    answer          TEXT,
    answered_at     INTEGER,
    created_at      INTEGER NOT NULL
);

CREATE TABLE file_changes (
    id           TEXT PRIMARY KEY,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id      TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    before_hash  TEXT,
    after_hash   TEXT,
    diff         BLOB,                     -- unified diff patch
    created_at   INTEGER NOT NULL
);
