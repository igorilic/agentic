#![cfg(test)]

use agentic_core::Db;
use agentic_core::db::runs::{Run, RunRepo};
use agentic_core::events::RunStatus;
use agentic_tauri::commands::runs::list_runs;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

fn build_app() -> (tauri::App<tauri::test::MockRuntime>, Db) {
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    {
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
             VALUES ('ws1', 'test', '/tmp/ws', 'github', 100, 100)",
            [],
        )
        .unwrap();
    }
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::runs::list_runs,
        ])
        .manage(db.clone())
        .build(mock_context(noop_assets()))
        .expect("build mock app");
    (app, db)
}

fn sample_run(id: &str, started_at: i64, ticket_body: Option<&str>) -> Run {
    Run {
        id: id.to_string(),
        workspace_id: "ws1".to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Completed,
        ticket_type: ticket_body.map(|_| "free-text".to_string()),
        ticket_ref: None,
        ticket_title: None,
        ticket_body: ticket_body.map(|s| s.to_string()),
        backend: "claude-code".to_string(),
        model: "sonnet".to_string(),
        started_at,
        completed_at: Some(started_at + 100),
        duration_ms: Some(100),
        token_usage: None,
        cost_usd: None,
        summary: None,
        subprocess_pid: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn list_runs_returns_empty_when_no_runs_exist() {
    let (app, _db) = build_app();
    let rows = list_runs(app.state::<Db>(), None).await.expect("list_runs");
    assert!(rows.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn list_runs_returns_summaries_in_descending_started_at_order() {
    let (app, db) = build_app();
    let repo = RunRepo::new(&db);
    repo.insert(sample_run("r1", 100, Some("first ticket")))
        .unwrap();
    repo.insert(sample_run("r2", 300, Some("third ticket")))
        .unwrap();
    repo.insert(sample_run("r3", 200, Some("second ticket")))
        .unwrap();

    let rows = list_runs(app.state::<Db>(), None).await.expect("list_runs");
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["r2", "r3", "r1"]);

    // Summary fields are populated.
    assert_eq!(rows[0].ticket_label.as_deref(), Some("third ticket"));
    assert_eq!(rows[0].status, "completed");
    assert_eq!(rows[0].backend, "claude-code");
    assert_eq!(rows[0].duration_ms, Some(100));
}

#[tokio::test(flavor = "multi_thread")]
async fn list_runs_respects_limit_argument() {
    let (app, db) = build_app();
    let repo = RunRepo::new(&db);
    for i in 0..5 {
        repo.insert(sample_run(&format!("r{i}"), i64::from(i) * 100, Some("t")))
            .unwrap();
    }

    let rows = list_runs(app.state::<Db>(), Some(2))
        .await
        .expect("list_runs");
    assert_eq!(rows.len(), 2);
}
