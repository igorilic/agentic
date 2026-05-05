#![cfg(test)]

use agentic_tauri::commands::agents::{AgentInfoDto, list_agents_inner};
use tokio::sync::Mutex as AsyncMutex;

/// Serialise tests that mutate process-global env vars.
static ENV_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Write a minimal valid agent `.md` file into `dir/<name>.md`.
fn write_agent(dir: &std::path::Path, name: &str, description: &str) {
    let content = format!(
        "+++\nname = \"{name}\"\ndescription = \"{description}\"\npipeline_role = \"step\"\n+++\nbody\n"
    );
    std::fs::write(dir.join(format!("{name}.md")), content).unwrap();
}

// ---------------------------------------------------------------------------
// Test 1: Project-tagged entries for .claude/agents/ files
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_returns_dtos_for_project_claude_agents() {
    let _g = ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().unwrap();
    let agents_dir = tmp.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    write_agent(&agents_dir, "architect", "Plans the work");

    let result = list_agents_inner("claude-code", tmp.path(), None);
    let agents = result.expect("list_agents_inner should succeed");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "architect");
    assert_eq!(agents[0].description, Some("Plans the work".to_string()));
    assert_eq!(agents[0].source, "project");
}

// ---------------------------------------------------------------------------
// Test 2: Empty workspace → empty Vec
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_returns_empty_when_no_agents() {
    let _g = ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().unwrap();

    let result = list_agents_inner("claude-code", tmp.path(), None);
    let agents = result.expect("list_agents_inner should succeed with empty workspace");
    assert!(agents.is_empty());
}

// ---------------------------------------------------------------------------
// Test 3: Unknown backend → Err
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_invalid_backend_string_returns_err() {
    let _g = ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().unwrap();

    let result = list_agents_inner("frobnicate", tmp.path(), None);
    assert!(result.is_err(), "expected Err for unknown backend");
    let msg = result.unwrap_err();
    // Should mention something about valid backends
    assert!(
        msg.to_lowercase().contains("backend")
            || msg.to_lowercase().contains("invalid")
            || msg.to_lowercase().contains("unknown"),
        "error message should mention the problem: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Test 4: DTO does NOT have a `path` field (compile-time type check)
// ---------------------------------------------------------------------------

#[test]
fn list_agents_strips_path_field() {
    // This is a compile-time check: if AgentInfoDto had a `path` field,
    // the struct literal below would have to include it or use `..`.
    // The test just constructs the DTO to verify only name/description/source exist.
    let dto = AgentInfoDto {
        name: "architect".to_string(),
        description: Some("Plans the work".to_string()),
        source: "project".to_string(),
    };
    // Verify serialization excludes `path`
    let json = serde_json::to_value(&dto).unwrap();
    assert!(json.get("path").is_none(), "DTO must not expose a path field");
    assert!(json.get("name").is_some());
    assert!(json.get("description").is_some());
    assert!(json.get("source").is_some());
}

// ---------------------------------------------------------------------------
// Test 5: Home-tagged entries
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_returns_home_tagged_entries() {
    let _g = ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().unwrap();
    let home_dir = tempfile::tempdir().unwrap();
    let home_agents_dir = home_dir.path().join(".claude").join("agents");
    std::fs::create_dir_all(&home_agents_dir).unwrap();
    write_agent(&home_agents_dir, "reviewer", "Reviews the code");

    // Workspace has no agents, home has one
    let result = list_agents_inner("claude-code", tmp.path(), Some(home_dir.path()));
    let agents = result.expect("list_agents_inner should succeed");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].name, "reviewer");
    assert_eq!(agents[0].source, "home");
}

// ---------------------------------------------------------------------------
// Test 6: Alphabetically sorted
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn list_agents_returns_alphabetically_sorted_entries() {
    let _g = ENV_LOCK.lock().await;
    let tmp = tempfile::tempdir().unwrap();
    let agents_dir = tmp.path().join(".claude").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    // Write in reverse order
    write_agent(&agents_dir, "reviewer", "Reviews code");
    write_agent(&agents_dir, "architect", "Plans work");

    let result = list_agents_inner("claude-code", tmp.path(), None);
    let agents = result.expect("list_agents_inner should succeed");
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].name, "architect");
    assert_eq!(agents[1].name, "reviewer");
}
