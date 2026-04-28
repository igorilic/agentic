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

/// Create a tempdir with `.claude/agents/` containing the four required
/// agent stubs and a fake `claude` binary at `<tmp>/bin/claude`. Sets
/// `AGENTIC_WORKSPACE_ROOT` to the tempdir and `CLAUDE_CODE_BIN` to the
/// fake binary so the pre-flight check passes in happy-path tests
/// regardless of whether real claude is installed in the CI environment.
/// Caller MUST hold the ENV_LOCK while this lives. Returns the tempdir so
/// it stays alive for the duration of the test.
fn setup_happy_path_workspace() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let agents_dir = tmp.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    for name in ["architect", "tdd-developer", "qa", "reviewer"] {
        std::fs::write(
            agents_dir.join(format!("{name}.md")),
            format!(
                "+++\nname = \"{name}\"\ndescription = \"stub\"\npipeline_role = \"step\"\n+++\nbody"
            ),
        )
        .unwrap();
    }
    let bin_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let fake_claude = bin_dir.join("claude");
    std::fs::write(&fake_claude, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_claude, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    unsafe {
        std::env::set_var("AGENTIC_WORKSPACE_ROOT", tmp.path());
        std::env::set_var("CLAUDE_CODE_BIN", &fake_claude);
    }
    tmp
}

fn teardown_happy_path_workspace() {
    unsafe {
        std::env::remove_var("AGENTIC_WORKSPACE_ROOT");
        std::env::remove_var("CLAUDE_CODE_BIN");
    }
}

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
    let _ws = setup_happy_path_workspace();
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
    teardown_happy_path_workspace();

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
    let tmp = setup_happy_path_workspace();
    let target_root = tmp.path();

    let (app, db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "do the thing".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;
    teardown_happy_path_workspace();

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
async fn start_ticket_run_registers_cancel_token_so_cancel_run_finds_it() {
    let _g = ENV_LOCK.lock().await;
    let _ws = setup_happy_path_workspace();
    let (app, _db) = build_app();

    let run_id = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "fix it".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await
    .expect("start_ticket_run");
    teardown_happy_path_workspace();

    // The existing cancel_run IPC reads from EventBusState's cancellation
    // registry. start_ticket_run must register the run's token there or
    // the user-facing Cancel button is a no-op for chat-driven runs.
    let bus_state = app.state::<EventBusState>();
    let cancelled = bus_state.cancel(&run_id).await;
    assert!(
        cancelled,
        "cancel_run should find the run's token; \
         start_ticket_run must register one in EventBusState"
    );

    // Idempotent: second cancel returns false because token is consumed.
    assert!(!bus_state.cancel(&run_id).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_fails_fast_when_claude_binary_is_missing() {
    let _g = ENV_LOCK.lock().await;

    // Point CLAUDE_CODE_BIN at a path that definitely doesn't exist so the
    // pre-flight `which` resolves to NotFound. Use a tempdir for the
    // workspace so the agent-file check doesn't fail first.
    let ws = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(ws.path().join(".claude").join("agents")).unwrap();
    for name in ["architect", "tdd-developer", "qa", "reviewer"] {
        std::fs::write(
            ws.path()
                .join(".claude")
                .join("agents")
                .join(format!("{name}.md")),
            "+++\nname = \"x\"\ndescription = \"y\"\npipeline_role = \"step\"\n+++\nbody",
        )
        .unwrap();
    }
    unsafe {
        std::env::set_var("AGENTIC_WORKSPACE_ROOT", ws.path());
        std::env::set_var("CLAUDE_CODE_BIN", "/definitely/no/such/binary/claude-xxx");
    }

    let (app, _db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "fix something".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;

    unsafe {
        std::env::remove_var("AGENTIC_WORKSPACE_ROOT");
        std::env::remove_var("CLAUDE_CODE_BIN");
    }

    let err = result.expect_err("missing claude binary must fail pre-flight");
    assert!(
        err.to_lowercase().contains("claude")
            && (err.contains("not found") || err.contains("PATH")),
        "error should call out claude + actionability; got: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_fails_fast_when_agent_files_are_missing() {
    let _g = ENV_LOCK.lock().await;

    // Empty workspace — no .claude/agents/ at all.
    let ws = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("AGENTIC_WORKSPACE_ROOT", ws.path());
    }

    let (app, _db) = build_app();
    let result = start_ticket_run(
        app.state::<EventBusState>(),
        app.state::<Db>(),
        "fix it".to_string(),
        "claude-code".to_string(),
        None,
    )
    .await;

    unsafe {
        std::env::remove_var("AGENTIC_WORKSPACE_ROOT");
    }

    let err = result.expect_err("missing agent files must fail pre-flight");
    assert!(
        err.contains("agent")
            && (err.contains("init") || err.contains("not found") || err.contains("agents/")),
        "error should call out agent files + suggest `init`; got: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn start_ticket_run_passes_through_the_model_override() {
    let _g = ENV_LOCK.lock().await;
    let _ws = setup_happy_path_workspace();
    // Override the copilot binary too — the workspace setup only stubs claude.
    let copilot_path = _ws.path().join("bin").join("copilot");
    std::fs::write(&copilot_path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&copilot_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    unsafe {
        std::env::set_var("COPILOT_CLI_BIN", &copilot_path);
    }

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
    unsafe {
        std::env::remove_var("COPILOT_CLI_BIN");
    }
    teardown_happy_path_workspace();

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
