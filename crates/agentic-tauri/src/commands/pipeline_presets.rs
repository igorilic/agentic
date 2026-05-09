//! Tauri IPC for pipeline preset CRUD.
//!
//! Four commands wrapping [`PipelinePresetRepo`]:
//! - `list_pipeline_presets` — ordered list (name ASC).
//! - `save_pipeline_preset` — create a new preset.
//! - `update_pipeline_preset` — rename / replace agents on an existing preset.
//! - `delete_pipeline_preset` — remove a preset by id.
//!
//! Errors are converted to `String` at the boundary via `.map_err(|e| e.to_string())`,
//! matching the convention used by `auth.rs` and `findings.rs`.

use agentic_core::{Db, PipelinePreset, PipelinePresetRepo};
use tauri::State;

#[tauri::command]
pub async fn list_pipeline_presets(db_state: State<'_, Db>) -> Result<Vec<PipelinePreset>, String> {
    PipelinePresetRepo::new(&db_state)
        .list()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_pipeline_preset(
    db_state: State<'_, Db>,
    name: String,
    agents: Vec<String>,
) -> Result<PipelinePreset, String> {
    PipelinePresetRepo::new(&db_state)
        .create(&name, &agents)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_pipeline_preset(
    db_state: State<'_, Db>,
    id: String,
    name: String,
    agents: Vec<String>,
) -> Result<PipelinePreset, String> {
    PipelinePresetRepo::new(&db_state)
        .update(&id, &name, &agents)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_pipeline_preset(db_state: State<'_, Db>, id: String) -> Result<(), String> {
    PipelinePresetRepo::new(&db_state)
        .delete(&id)
        .map_err(|e| e.to_string())
}
