fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "subscribe_events",
            "get_event_history",
            "start_scripted_run",
            "cancel_run",
            "chat_send_message",
            "chat_list_messages",
            "mention_agent",
            "triage_finding",
            "list_findings",
        ]),
    ))
    .expect("failed to run tauri build");
}
