// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;

use agentic_core::events::EventBus;
use commands::events::EventBusState;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let bus = Arc::new(EventBus::new());
            app.manage(EventBusState::new(bus));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::events::subscribe_events])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
