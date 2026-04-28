-- Closes #71: `findings.id` was a single-column PRIMARY KEY but
-- `Event::Finding.finding_id` is unique only within a run. Re-running the
-- same script (or two ticket runs whose reviewers happen to use the same
-- finding_ids) collided on PK and silently dropped the second run's
-- findings.
--
-- This migration recreates `findings` with PRIMARY KEY (run_id, id) so the
-- invariant lives in the schema. Existing rows are preserved as-is — the
-- workaround scoping (`<run_id>:<finding_id>` ids) is still uniquely keyed
-- under the new composite PK because each scoped id lives under its own
-- run_id. New code inserts plain finding_ids.
--
-- Triggers from migration 0007 are dropped + recreated against the new
-- table (SQLite triggers are bound to a specific table name; rename
-- doesn't carry them over).

DROP TRIGGER IF EXISTS findings_triage_check_insert;
DROP TRIGGER IF EXISTS findings_triage_check_update;

CREATE TABLE findings_new (
    id           TEXT NOT NULL,
    run_id       TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    step_id      TEXT NOT NULL REFERENCES run_steps(id) ON DELETE CASCADE,
    severity     TEXT NOT NULL,
    file_path    TEXT,
    line         INTEGER,
    message      TEXT NOT NULL,
    suggestion   TEXT,
    triage       TEXT,                     -- null | 'fix' | 'tech-debt' | 'ignore'
    triaged_at   INTEGER,
    created_at   INTEGER NOT NULL,
    PRIMARY KEY (run_id, id)
);

INSERT INTO findings_new (
    id, run_id, step_id, severity, file_path, line, message, suggestion,
    triage, triaged_at, created_at
)
SELECT
    id, run_id, step_id, severity, file_path, line, message, suggestion,
    triage, triaged_at, created_at
FROM findings;

DROP TABLE findings;
ALTER TABLE findings_new RENAME TO findings;

-- Recreate the index from migration 0003.
CREATE INDEX idx_findings_run_triage ON findings(run_id, triage);

-- Recreate the triage-value triggers from migration 0007.
CREATE TRIGGER findings_triage_check_insert
BEFORE INSERT ON findings
FOR EACH ROW
WHEN NEW.triage IS NOT NULL
 AND NEW.triage NOT IN ('fix', 'tech-debt', 'ignore')
BEGIN
    SELECT RAISE(ABORT, 'invalid findings.triage value');
END;

CREATE TRIGGER findings_triage_check_update
BEFORE UPDATE OF triage ON findings
FOR EACH ROW
WHEN NEW.triage IS NOT NULL
 AND NEW.triage NOT IN ('fix', 'tech-debt', 'ignore')
BEGIN
    SELECT RAISE(ABORT, 'invalid findings.triage value');
END;
