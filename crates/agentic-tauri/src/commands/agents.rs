//! Tauri IPC for listing discoverable agents.
//!
//! Exposes `list_agents(backend: String) -> Result<Vec<AgentInfoDto>, String>`.
//!
//! The DTO carries only `name`, `description`, `source` — no `path` field.
//! This keeps the user's home directory off the renderer process (P.3.1 rule).
//!
//! Workspace root resolution follows the same two-step approach as
//! `start_ticket_run`: honour `AGENTIC_WORKSPACE_ROOT` first, then fall back
//! to `std::env::current_dir()`.

use std::path::Path;

use agentic_core::{AgentSource, BackendKind, list_discoverable};

use super::workspace::resolve_workspace_root;

// ---------------------------------------------------------------------------
// DTO
// ---------------------------------------------------------------------------

/// Webview-safe agent descriptor. No filesystem paths are included.
#[derive(serde::Serialize, Clone, Debug, PartialEq, Eq)]
pub struct AgentInfoDto {
    pub name: String,
    /// `None` when the frontmatter `description` field was absent.
    pub description: Option<String>,
    /// `"project"` or `"home"`.
    pub source: String,
}

// ---------------------------------------------------------------------------
// Pure testable inner function
// ---------------------------------------------------------------------------

/// Core logic: given a backend string, a workspace root, and an optional home
/// directory, return the discovered agent DTOs.
///
/// Separated from the Tauri command so unit tests can call it directly
/// without spinning up a mock Tauri app.
pub fn list_agents_inner(
    backend: &str,
    ws_root: &Path,
    home: Option<&Path>,
) -> Result<Vec<AgentInfoDto>, String> {
    let backend_kind = BackendKind::parse(backend)?;

    let infos = list_discoverable(backend_kind, ws_root, home)
        .map_err(|e| format!("list_discoverable: {e}"))?;

    let dtos = infos
        .into_iter()
        .map(|info| AgentInfoDto {
            name: info.name,
            description: info.description,
            source: match info.source {
                AgentSource::Project => "project".to_string(),
                AgentSource::Home => "home".to_string(),
            },
        })
        .collect();

    Ok(dtos)
}

// ---------------------------------------------------------------------------
// Tauri command
// ---------------------------------------------------------------------------

/// List all agents discoverable for the given backend from the current
/// workspace root.
///
/// Backend is a kebab-case string: `"claude-code"` or `"copilot-cli"`.
/// Returns `Err` for an unknown backend or if the workspace root cannot
/// be resolved.
#[tauri::command]
pub async fn list_agents(backend: String) -> Result<Vec<AgentInfoDto>, String> {
    let ws_root = resolve_workspace_root()?;

    // Resolve the home directory via the `directories` crate (same crate
    // used elsewhere in the workspace). `None` is acceptable; the inner
    // function will simply skip home-directory search.
    let base = directories::BaseDirs::new();
    let home_path = base.as_ref().map(|b| b.home_dir().to_path_buf());

    list_agents_inner(&backend, &ws_root, home_path.as_deref())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn write_agent(dir: &std::path::Path, name: &str, description: &str) {
        let content = format!(
            "+++\nname = \"{name}\"\ndescription = \"{description}\"\npipeline_role = \"step\"\n+++\nbody\n"
        );
        std::fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn list_agents_inner_returns_project_dto() {
        let tmp = tempfile::tempdir().unwrap();
        let agents_dir = tmp.path().join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        write_agent(&agents_dir, "architect", "Plans the work");

        let result = list_agents_inner("claude-code", tmp.path(), None).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "architect");
        assert_eq!(result[0].description, Some("Plans the work".to_string()));
        assert_eq!(result[0].source, "project");
    }

    #[test]
    fn list_agents_inner_empty_workspace_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_agents_inner("claude-code", tmp.path(), None).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_agents_inner_unknown_backend_returns_err() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_agents_inner("frobnicate", tmp.path(), None);
        assert!(result.is_err());
    }

    #[test]
    fn agent_info_dto_has_no_path_field() {
        let dto = AgentInfoDto {
            name: "architect".to_string(),
            description: Some("Plans the work".to_string()),
            source: "project".to_string(),
        };
        let json = serde_json::to_value(&dto).unwrap();
        assert!(json.get("path").is_none());
        assert!(json.get("name").is_some());
        assert!(json.get("source").is_some());
    }
}
