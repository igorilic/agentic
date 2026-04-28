#![cfg(test)]

use std::sync::Arc;

use agentic_core::Db;
use agentic_core::events::EventBus;
use agentic_tauri::commands::events::EventBusState;
use agentic_tauri::commands::ticket::start_ticket_run;
use tauri::Manager;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tokio::sync::Mutex as AsyncMutex;

/// Serialise tests that read or mutate `AGENTIC_WORKSPACE_ROOT`. Cargo runs
/// tests in parallel by default; env vars are process-global, so without
/// this lock test A could see test B's mid-run env mutation and assert
/// against the wrong workspace path. Uses `tokio::sync::Mutex` because the
/// guard crosses `.await` points inside the test bodies.
static ENV_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

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
    let _g = ENV_LOCK.lock().await;
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
    let _g = ENV_LOCK.lock().await;
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
    let _g = ENV_LOCK.lock().await;
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
async fn start_ticket_run_honors_agentic_workspace_root_env_var() {
    let _g = ENV_LOCK.lock().await;
    // Regression: when launched via `cargo tauri dev`, cwd is the tauri
    // crate dir — not the user's target repo. The env var override lets
    // users point the IPC at the right workspace without changing cwd.
    let tmp = tempfile::tempdir().unwrap();
    let target_root = tmp.path();

    // SAFETY: tests in this module run on a multi_thread runtime, but env
    // mutation is process-global. set_var is `unsafe` on Rust 2024.
    unsafe {
        std::env::set_var("AGENTIC_WORKSPACE_ROOT", target_root);
    }

    let (app, db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "do the thing".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;

    unsafe {
        std::env::remove_var("AGENTIC_WORKSPACE_ROOT");
    }

    let run_id = result.expect("start_ticket_run with env override");

    // Workspace row should reference the env-var path, not cwd.
    let conn = db.conn().unwrap();
    let (workspace_id, root_path): (String, String) = conn
        .query_row(
            "SELECT w.id, w.root_path FROM runs r \
             JOIN workspaces w ON w.id = r.workspace_id \
             WHERE r.id = ?1",
            [&run_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert!(
        workspace_id.starts_with("ws-"),
        "stable_workspace_id should produce ws- prefix"
    );
    assert_eq!(
        std::path::PathBuf::from(&root_path).canonicalize().unwrap(),
        target_root.canonicalize().unwrap(),
        "workspace root_path should reflect AGENTIC_WORKSPACE_ROOT, not cwd"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_rejects_workspace_root_that_is_not_a_directory() {
    let _g = ENV_LOCK.lock().await;
    unsafe {
        std::env::set_var(
            "AGENTIC_WORKSPACE_ROOT",
            "/definitely/does/not/exist/anywhere",
        );
    }

    let (app, _db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "x".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;

    unsafe {
        std::env::remove_var("AGENTIC_WORKSPACE_ROOT");
    }

    assert!(
        result.is_err(),
        "should reject a non-directory workspace root"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_passes_through_the_model_override() {
    let _g = ENV_LOCK.lock().await;
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
