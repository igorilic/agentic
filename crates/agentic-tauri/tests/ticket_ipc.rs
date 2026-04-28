#![cfg(test)]

use std::sync::Arc;

use agentic_core::Db;
use agentic_core::events::EventBus;
use agentic_tauri::commands::events::EventBusState;
use agentic_tauri::commands::ticket::start_ticket_run;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};

fn build_app() -> (tauri::App<tauri::test::MockRuntime>, Db) {
    let bus = Arc::new(EventBus::new());
    let db = Db::open_in_memory().expect("Db::open_in_memory");
    let app = mock_builder()
        .invoke_handler(tauri::generate_handler![
            agentic_tauri::commands::ticket::start_ticket_run,
        ])
        .manage(EventBusState::new(bus))
        .manage(db.clone())
        .build(mock_context(noop_assets()))
        .expect("build mock app");
    (app, db)
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_rejects_unknown_backend() {
    let (app, _db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "fix the bug".to_string(),
        "made-up-backend".to_string(),
        None,
    )
    .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_lowercase().contains("invalid backend"),
        "error should explain the invalid backend: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_rejects_empty_ticket() {
    let (app, _db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "   ".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_seeds_workspace_and_run_rows_then_returns_run_id() {
    let (app, db) = build_app();

    let run_id = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "fix the auth race".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await
    .expect("start_ticket_run should succeed on the seed-and-spawn path");

    // ULID lowercase: 26 chars
    assert_eq!(
        run_id.len(),
        26,
        "run_id should be a 26-char ULID, got {run_id:?}"
    );

    // Run row was seeded.
    let conn = db.conn().unwrap();
    let (id, ticket_body, backend, status): (String, Option<String>, String, String) = conn
        .query_row(
            "SELECT id, ticket_body, backend, status FROM runs WHERE id = ?1",
            [&run_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .expect("run row must exist");
    assert_eq!(id, run_id);
    assert_eq!(ticket_body.as_deref(), Some("fix the auth race"));
    assert_eq!(backend, "claude-code");
    // Status is Pending right after seeding; the spawned pipeline transitions
    // it to Running/Completed/Failed in the background. We don't await that
    // here — testing the seed contract is enough; full pipeline behaviour is
    // covered by agentic-cli's own tests.
    assert_eq!(status, "pending");
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_passes_through_the_model_override() {
    let (app, db) = build_app();

    let run_id = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "implement export".to_string(),
        "copilot-cli".to_string(),
        Some("claude-sonnet-4-6".to_string()),
    )
    .await
    .expect("start_ticket_run");

    let conn = db.conn().unwrap();
    let (model, backend): (String, String) = conn
        .query_row(
            "SELECT model, backend FROM runs WHERE id = ?1",
            [&run_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(model, "claude-sonnet-4-6");
    assert_eq!(backend, "copilot-cli");
}
