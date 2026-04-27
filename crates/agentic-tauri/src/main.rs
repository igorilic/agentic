// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Import the commands module from our own library crate. Avoid `mod commands;`
// here, which would compile a second copy of the module into the binary crate
// and trip rustc's dead-code analyzer for items not used directly by main.rs.
use agentic_tauri::commands;

use std::sync::Arc;

use agentic_core::events::EventBus;
use agentic_core::{Db, Paths};
use commands::chat::ChatState;
use commands::events::EventBusState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let bus = Arc::new(EventBus::new());
            app.manage(EventBusState::new(bus));

            let paths = Paths::from_os().expect("resolve OS paths");
            paths.ensure_dirs().expect("ensure data dirs");
            let db = Arc::new(Db::open(&paths).expect("open database"));
            app.manage(ChatState::new(db));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::events::subscribe_events,
            commands::events::get_event_history,
            commands::scripted::start_scripted_run,
            commands::scripted::cancel_run,
            commands::chat::chat_send_message,
            commands::chat::chat_list_messages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
