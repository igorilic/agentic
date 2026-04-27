-- Defense-in-depth CHECK on findings.triage. SQLite ALTER TABLE cannot add
-- CHECK constraints to existing columns, so we install BEFORE INSERT and
-- BEFORE UPDATE triggers that abort the write when triage is set to a value
-- outside the documented set.
--
-- The application layer (FindingsRepo::update_triage in agentic-core) is the
-- primary gatekeeper; this trigger is a backstop for direct SQL access and
-- future repo additions that forget to validate.
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
