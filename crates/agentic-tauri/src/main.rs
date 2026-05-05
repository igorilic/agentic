// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import the commands module from our own library crate. Avoid `mod commands;`
// here, which would compile a second copy of the module into the binary crate
// and trip rustc's dead-code analyzer for items not used directly by main.rs.
use agentic_tauri::commands;

use std::sync::Arc;

use agentic_core::auth::secrets::KeyringSecretStore;
use agentic_core::db::auth::AuthRepo;
use agentic_core::db::runs::RunRepo;
use agentic_core::db::steps::StepRepo;
use agentic_core::db::workspaces::{Workspace, WorkspaceRepo};
use agentic_core::events::EventBus;
use agentic_core::permissions::gate_async::AsyncGate;
use agentic_core::pipeline::PipelineOrchestrator;
use agentic_core::{Db, Paths, init as init_logging};
use commands::auth::{AuthState, WebbrowserOpener};
use commands::chat::ChatState;
use commands::events::EventBusState;
use commands::findings::FindingsState;
use tauri::Manager;

fn main() {
    // Initialise tracing so backend warnings (e.g., "failed to persist
    // finding; continuing") actually surface in stderr instead of vanishing.
    // Override via `RUST_LOG=agentic_tauri=debug,agentic_core=debug`.
    init_logging(None);

    tauri::Builder::default()
        .setup(|app| {
            let paths = Paths::from_os().expect("resolve OS paths");
            paths.ensure_dirs().expect("ensure data dirs");
            let db = Db::open(&paths).expect("open database");

            // Seed the "default" workspace row so the chat pane can create
            // sessions on a fresh install without hitting an FK violation.
            let ws_repo = WorkspaceRepo::new(&db);
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            ws_repo
                .insert_if_absent(Workspace {
                    id: "default".to_string(),
                    name: "Default".to_string(),
                    root_path: paths.data_dir().to_string_lossy().into_owned(),
                    remote_url: None,
                    profile: "custom".to_string(),
                    created_at: now_ms,
                    last_opened: now_ms,
                })
                .expect("seed default workspace");

            let bus = Arc::new(EventBus::new());

            // Spawn ONE PipelineOrchestrator subscribed to the managed bus
            // for the lifetime of the app. Per-run spawning (the previous
            // approach) caused two orchestrators to race on RunStarted
            // after the second /plan, producing
            // `invalid state transition from "running" to "running"`.
            // The handle is intentionally not stored: the orchestrator
            // exits when the bus is dropped, which happens at app shutdown.
            tauri::async_runtime::block_on(async {
                // Load permissions config from the app data dir. If the
                // file does not exist, write the built-in default to disk
                // so users have a discoverable, editable starting point —
                // and log where we loaded from so confused users can find
                // it via the dev console.
                let permissions_path = paths.data_dir().join("permissions.toml");
                if !permissions_path.exists() {
                    if let Some(parent) = permissions_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(
                        &permissions_path,
                        agentic_core::permissions::builtin_permissions_toml(),
                    ) {
                        tracing::warn!(
                            "Could not write default permissions.toml at {}: {e}",
                            permissions_path.display()
                        );
                    } else {
                        tracing::info!(
                            "Wrote default permissions.toml at {} — edit to customize",
                            permissions_path.display()
                        );
                    }
                } else {
                    tracing::info!(
                        "Loading permissions.toml from {}",
                        permissions_path.display()
                    );
                }
                let permissions_config = agentic_core::PermissionsConfig::load(&permissions_path)
                    .unwrap_or_else(|_| agentic_core::PermissionsConfig::builtin_default());
                let gate = Arc::new(AsyncGate::new(
                    permissions_config,
                    (*bus).clone(),
                    std::time::Duration::from_secs(60),
                    "agentic-tauri".to_string(),
                ));
                // Drop the JoinHandle deliberately — the spawned task runs
                // independently and exits when the bus is dropped at app
                // shutdown.
                let _orchestrator = PipelineOrchestrator::spawn(
                    (*bus).clone(),
                    RunRepo::new(&db),
                    StepRepo::new(&db),
                    gate,
                );
            });

            // EventBusState::new internally handles the missing-runtime
            // case (Tauri 2's setup hook runs outside a tokio context).
            app.manage(EventBusState::new(bus));
            app.manage(ChatState::new(&db));
            app.manage(FindingsState::new(&db));

            // Auth: real OS keyring + production browser opener. The
            // GitHub base URL is the public host; integration tests
            // inject a wiremock server via build_app helpers.
            let auth_state = AuthState {
                repo: AuthRepo::new(&db),
                secrets: Arc::new(KeyringSecretStore::new("io.agentic.app")),
                opener: Arc::new(WebbrowserOpener),
                github_base_url: "https://github.com".to_string(),
                callback_timeout_secs: 5 * 60,
                gh_binary: std::path::PathBuf::from("gh"),
            };
            app.manage(auth_state);

            // Manage the Db itself so commands that need to seed multiple
            // tables in one call (scripted_run) can do their own writes.
            app.manage(db);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::subscribe_events,
            commands::events::get_event_history,
            commands::scripted::start_scripted_run,
            commands::scripted::cancel_run,
            commands::chat::chat_send_message,
            commands::chat::chat_list_messages,
            commands::mention::mention_agent,
            commands::findings::triage_finding,
            commands::findings::list_findings,
            commands::auth::list_auth_accounts,
            commands::auth::delete_auth_account,
            commands::auth::connect_github,
            commands::auth::connect_github_via_gh,
            commands::ticket::start_ticket_run,
            commands::runs::list_runs,
            commands::permissions::permission_decide,
            commands::jira::fetch_jira_ticket,
            commands::agents::list_agents,
            commands::workspace::get_workspace_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
