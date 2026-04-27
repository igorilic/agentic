use agentic_core::Db;
use agentic_core::db::findings::{FindingRow, FindingsRepo};

fn setup_in_memory() -> (Db, FindingsRepo) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    seed_run_and_step(&db);
    let repo = FindingsRepo::new(&db);
    (db, repo)
}

fn seed_run_and_step(db: &Db) {
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES ('ws1', 'test', '/tmp/test', 'github', 100, 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO runs (id, workspace_id, status, backend, model, started_at) \
         VALUES ('run1', 'ws1', 'running', 'claude-code', 'sonnet', 100)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO run_steps (id, run_id, seq, agent_name, status) \
         VALUES ('step1', 'run1', 0, 'reviewer', 'passed')",
        [],
    )
    .unwrap();
}

fn sample_finding(id: &str, message: &str, severity: &str) -> FindingRow {
    FindingRow {
        id: id.to_string(),
        run_id: "run1".to_string(),
        step_id: "step1".to_string(),
        severity: severity.to_string(),
        file_path: Some("src/main.rs".to_string()),
        line: Some(42),
        message: message.to_string(),
        suggestion: None,
        triage: None,
        triaged_at: None,
        created_at: 200,
    }
}

#[test]
fn insert_finding_persists_and_lists_by_run() {
    let (_db, repo) = setup_in_memory();

    repo.insert(&sample_finding("f1", "missing-error-handling", "warning"))
        .expect("insert f1");
    repo.insert(&sample_finding("f2", "unused-import", "info"))
        .expect("insert f2");

    let list = repo.list_by_run("run1").expect("list_by_run");
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|f| f.id == "f1"));
    assert!(list.iter().any(|f| f.id == "f2"));
}

#[test]
fn update_triage_sets_triage_value_and_returns_true() {
    let (_db, repo) = setup_in_memory();
    repo.insert(&sample_finding("f1", "msg", "warning"))
        .unwrap();

    let updated = repo.update_triage("f1", "tech-debt", 300).expect("update");
    assert!(
        updated,
        "expected update_triage to return true for existing row"
    );

    let list = repo.list_by_run("run1").unwrap();
    let row = list.iter().find(|f| f.id == "f1").unwrap();
    assert_eq!(row.triage.as_deref(), Some("tech-debt"));
    assert_eq!(row.triaged_at, Some(300));
}

#[test]
fn update_triage_returns_false_for_unknown_finding() {
    let (_db, repo) = setup_in_memory();

    let updated = repo
        .update_triage("nonexistent", "fix", 300)
        .expect("update_triage");
    assert!(
        !updated,
        "expected update_triage to return false for missing row"
    );
}

#[test]
fn update_triage_rejects_invalid_triage_value() {
    let (_db, repo) = setup_in_memory();
    repo.insert(&sample_finding("f1", "msg", "warning"))
        .unwrap();

    let result = repo.update_triage("f1", "lol-not-a-real-triage", 300);
    assert!(result.is_err(), "expected error for invalid triage value");
}

#[test]
fn update_triage_trims_whitespace_around_value() {
    let (_db, repo) = setup_in_memory();
    repo.insert(&sample_finding("f1", "msg", "warning"))
        .unwrap();

    let updated = repo
        .update_triage("f1", "  tech-debt  ", 300)
        .expect("update");
    assert!(updated, "trimmed triage should match ALLOWED_TRIAGE");

    let list = repo.list_by_run("run1").unwrap();
    let row = list.iter().find(|f| f.id == "f1").unwrap();
    assert_eq!(row.triage.as_deref(), Some("tech-debt"));
}

#[test]
fn insert_rejects_unknown_severity() {
    let (_db, repo) = setup_in_memory();

    let row = sample_finding("f1", "msg", "warn"); // wrong: should be 'warning'
    let result = repo.insert(&row);
    assert!(result.is_err(), "expected severity rejection for 'warn'");
}

#[test]
fn trigger_rejects_invalid_triage_inserted_via_raw_sql() {
    // Defense-in-depth: even if a future repo bypasses ALLOWED_TRIAGE, the
    // BEFORE INSERT trigger from migration 0007 must abort the write.
    let (db, repo) = setup_in_memory();
    repo.insert(&sample_finding("f1", "msg", "warning"))
        .unwrap();

    let conn = db.conn().unwrap();
    let result = conn.execute(
        "INSERT INTO findings \
         (id, run_id, step_id, severity, message, triage, created_at) \
         VALUES ('f2', 'run1', 'step1', 'warning', 'msg', 'not-a-real-triage', 200)",
        [],
    );
    assert!(result.is_err(), "trigger should reject invalid triage");
}

#[test]
fn trigger_rejects_invalid_triage_updated_via_raw_sql() {
    let (db, repo) = setup_in_memory();
    repo.insert(&sample_finding("f1", "msg", "warning"))
        .unwrap();

    let conn = db.conn().unwrap();
    let result = conn.execute(
        "UPDATE findings SET triage = 'not-a-real-triage' WHERE id = 'f1'",
        [],
    );
    assert!(
        result.is_err(),
        "trigger should reject invalid triage on update"
    );
}

#[test]
fn list_by_run_returns_empty_for_unknown_run() {
    let (_db, repo) = setup_in_memory();

    let list = repo.list_by_run("no-such-run").expect("list_by_run");
    assert!(list.is_empty());
}
