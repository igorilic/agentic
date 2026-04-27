// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import the commands module from our own library crate. Avoid `mod commands;`
// here, which would compile a second copy of the module into the binary crate
// and trip rustc's dead-code analyzer for items not used directly by main.rs.
use agentic_tauri::commands;

use std::sync::Arc;

use agentic_core::db::workspaces::{Workspace, WorkspaceRepo};
use agentic_core::events::EventBus;
use agentic_core::{Db, Paths, init as init_logging};
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
            // EventBusState::new internally handles the missing-runtime
            // case (Tauri 2's setup hook runs outside a tokio context).
            app.manage(EventBusState::new(bus));
            app.manage(ChatState::new(&db));
            app.manage(FindingsState::new(&db));
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
