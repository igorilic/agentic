use std::path::Path;

use agentic_core::{CoreError, discover_agent};

/// Write a minimal valid agent markdown file at `repo_root/subdir/filename`.
/// `marker` is embedded in the system prompt body so tests can confirm
/// which file was loaded.
fn write_agent(repo_root: &Path, subdir: &str, filename: &str, name: &str, marker: &str) {
    let dir = repo_root.join(subdir);
    std::fs::create_dir_all(&dir).expect("create_dir_all");
    let content = format!("---\nname: {name}\ndescription: test agent ({marker})\n---\n{marker}\n");
    std::fs::write(dir.join(filename), content).expect("write agent file");
}

#[test]
fn agentic_agents_dir_wins_over_claude_and_legacy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    write_agent(root, ".agentic/agents", "architect.md", "architect", "AGENTIC");
    write_agent(root, ".claude/agents", "architect.md", "architect", "CLAUDE");
    write_agent(root, "agents", "architect.md", "architect", "LEGACY");

    let agent = discover_agent(root, "architect").expect("discover");
    assert_eq!(agent.name, "architect");
    assert!(
        agent.description.contains("AGENTIC"),
        ".agentic/agents/ should win; got description: {}",
        agent.description
    );
    assert!(
        agent.system_prompt.trim_end().ends_with("AGENTIC"),
        "system_prompt should come from .agentic/agents/; got: {}",
        agent.system_prompt
    );
}

#[test]
fn claude_agents_dir_wins_over_legacy_when_agentic_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    // Intentionally omit .agentic/agents/
    write_agent(root, ".claude/agents", "tdd-developer.md", "tdd-developer", "CLAUDE");
    write_agent(root, "agents", "tdd-developer.md", "tdd-developer", "LEGACY");

    let agent = discover_agent(root, "tdd-developer").expect("discover");
    assert_eq!(agent.name, "tdd-developer");
    assert!(
        agent.description.contains("CLAUDE"),
        ".claude/agents/ should win over legacy when .agentic/ absent; got: {}",
        agent.description
    );
}

#[test]
fn missing_agent_returns_agent_not_found_with_all_searched_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let result = discover_agent(root, "nonexistent");
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "nonexistent");
            assert_eq!(searched.len(), 3, "should try all 3 search locations");
            // Confirm the order: .agentic, .claude, legacy
            let paths_as_str: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            assert!(paths_as_str[0].contains(".agentic/agents"), "first: {}", paths_as_str[0]);
            assert!(paths_as_str[1].contains(".claude/agents"), "second: {}", paths_as_str[1]);
            assert!(
                paths_as_str[2].ends_with("agents/nonexistent.md"),
                "third (legacy): {}",
                paths_as_str[2]
            );
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}
