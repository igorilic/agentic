#![cfg(test)]

use agentic_core::Db;
use agentic_core::db::findings::{FindingRow, FindingsRepo};
use agentic_tauri::commands::findings::{FindingsState, triage_finding};
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

fn seed_run_step_and_finding(db: &Db) {
    {
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
             VALUES ('default', 'test', '/tmp/test', 'github', 100, 100)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO runs (id, workspace_id, status, backend, model, started_at) \
             VALUES ('run1', 'default', 'running', 'claude-code', 'sonnet', 100)",
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
    // Conn dropped here so the in-memory pool (max_size=1) can hand it out
    // again to the repo below.
    let repo = FindingsRepo::new(db);
    repo.insert(&FindingRow {
        id: "f1".to_string(),
        run_id: "run1".to_string(),
        step_id: "step1".to_string(),
        severity: "warn".to_string(),
        file_path: None,
        line: None,
        message: "stub".to_string(),
        suggestion: None,
        triage: None,
        triaged_at: None,
        created_at: 200,
    })
    .unwrap();
}

fn build_app() -> (tauri::App<tauri::test::MockRuntime>, Db) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    seed_run_step_and_finding(&db);
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::findings::triage_finding,
        ])
        .manage(FindingsState::new(&db))
        .build(mock_context(noop_assets()))
        .expect("build mock app");
    (app, db)
}

#[tokio::test(flavor = "multi_thread")]
async fn triage_finding_updates_the_row() {
    let (app, db) = build_app();
    let state = app.state::<FindingsState>();

    triage_finding(state, "f1".to_string(), "tech-debt".to_string())
        .await
        .expect("triage_finding");

    let repo = FindingsRepo::new(&db);
    let row = repo
        .list_by_run("run1")
        .unwrap()
        .into_iter()
        .find(|f| f.id == "f1")
        .expect("finding f1");
    assert_eq!(row.triage.as_deref(), Some("tech-debt"));
    assert!(row.triaged_at.is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn triage_finding_returns_err_for_unknown_id() {
    let (app, _db) = build_app();
    let state = app.state::<FindingsState>();

    let result = triage_finding(state, "nope".to_string(), "fix".to_string()).await;

    assert!(result.is_err(), "expected Err for unknown finding id");
}

#[tokio::test(flavor = "multi_thread")]
async fn triage_finding_rejects_invalid_triage_value() {
    let (app, _db) = build_app();
    let state = app.state::<FindingsState>();

    let result = triage_finding(state, "f1".to_string(), "wat".to_string()).await;

    assert!(result.is_err(), "expected Err for invalid triage");
}
