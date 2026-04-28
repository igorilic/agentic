use std::path::Path;

use agentic_core::{CoreError, discover_agent, discover_agent_with_home};

/// Write a minimal valid agent markdown file at `root/subdir/filename`.
/// `marker` is embedded in the system prompt body so tests can confirm
/// which file was loaded.
fn write_agent(root: &Path, subdir: &str, filename: &str, name: &str, marker: &str) {
    let dir = root.join(subdir);
    std::fs::create_dir_all(&dir).expect("create_dir_all");
    let content =
        format!("+++\nname = \"{name}\"\ndescription = \"test agent ({marker})\"\n+++\n{marker}\n");
    std::fs::write(dir.join(filename), content).expect("write agent file");
}

// ─── repo-local search-order priority (no home) ───────────────────────────────

#[test]
fn agentic_agents_dir_wins_over_claude_github_and_legacy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap(); // empty — no global agents

    write_agent(
        root,
        ".agentic/agents",
        "architect.md",
        "architect",
        "AGENTIC",
    );
    write_agent(
        root,
        ".claude/agents",
        "architect.md",
        "architect",
        "CLAUDE",
    );
    write_agent(
        root,
        ".github/agents",
        "architect.md",
        "architect",
        "GITHUB",
    );
    write_agent(root, "agents", "architect.md", "architect", "LEGACY");

    let agent = discover_agent_with_home(root, Some(home.path()), "architect").expect("discover");
    assert!(
        agent.description.contains("AGENTIC"),
        ".agentic/agents/ should win; got: {}",
        agent.description
    );
}

#[test]
fn claude_agents_dir_wins_over_github_and_legacy_when_agentic_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        root,
        ".claude/agents",
        "tdd-developer.md",
        "tdd-developer",
        "CLAUDE",
    );
    write_agent(
        root,
        ".github/agents",
        "tdd-developer.md",
        "tdd-developer",
        "GITHUB",
    );
    write_agent(
        root,
        "agents",
        "tdd-developer.md",
        "tdd-developer",
        "LEGACY",
    );

    let agent =
        discover_agent_with_home(root, Some(home.path()), "tdd-developer").expect("discover");
    assert!(
        agent.description.contains("CLAUDE"),
        ".claude/agents/ should beat .github and legacy; got: {}",
        agent.description
    );
}

#[test]
fn github_agents_dir_wins_over_legacy_when_claude_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(root, ".github/agents", "qa.md", "qa", "GITHUB");
    write_agent(root, "agents", "qa.md", "qa", "LEGACY");

    let agent = discover_agent_with_home(root, Some(home.path()), "qa").expect("discover");
    assert!(
        agent.description.contains("GITHUB"),
        ".github/agents/ should beat legacy; got: {}",
        agent.description
    );
}

// ─── home-dir fallback ────────────────────────────────────────────────────────

#[test]
fn home_claude_agents_used_when_repo_has_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "HOME_CLAUDE",
    );

    let agent =
        discover_agent_with_home(tmp.path(), Some(home.path()), "architect").expect("discover");
    assert!(
        agent.description.contains("HOME_CLAUDE"),
        "$HOME/.claude/agents/ should be discovered as fallback; got: {}",
        agent.description
    );
}

#[test]
fn home_copilot_agents_used_when_no_higher_priority_match() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".copilot/agents",
        "reviewer.md",
        "reviewer",
        "HOME_COPILOT",
    );

    let agent =
        discover_agent_with_home(tmp.path(), Some(home.path()), "reviewer").expect("discover");
    assert!(
        agent.description.contains("HOME_COPILOT"),
        "$HOME/.copilot/agents/ should be discovered; got: {}",
        agent.description
    );
}

#[test]
fn repo_claude_agents_wins_over_home_claude_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        root,
        ".claude/agents",
        "architect.md",
        "architect",
        "REPO_CLAUDE",
    );
    write_agent(
        home.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "HOME_CLAUDE",
    );

    let agent = discover_agent_with_home(root, Some(home.path()), "architect").expect("discover");
    assert!(
        agent.description.contains("REPO_CLAUDE"),
        "repo-local .claude/agents/ should beat $HOME; got: {}",
        agent.description
    );
}

#[test]
fn home_claude_wins_over_home_copilot_when_both_present() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(home.path(), ".claude/agents", "qa.md", "qa", "HOME_CLAUDE");
    write_agent(
        home.path(),
        ".copilot/agents",
        "qa.md",
        "qa",
        "HOME_COPILOT",
    );

    let agent = discover_agent_with_home(tmp.path(), Some(home.path()), "qa").expect("discover");
    assert!(
        agent.description.contains("HOME_CLAUDE"),
        "$HOME/.claude should beat $HOME/.copilot; got: {}",
        agent.description
    );
}

// ─── default discover_agent (no test home) — must not panic ───────────────────

#[test]
fn default_discover_agent_resolves_real_home_without_panicking() {
    let tmp = tempfile::tempdir().unwrap();
    write_agent(
        tmp.path(),
        ".agentic/agents",
        "architect.md",
        "architect",
        "REPO",
    );
    let agent = discover_agent(tmp.path(), "architect").expect("discover");
    assert_eq!(agent.name, "architect");
}

// ─── error path ───────────────────────────────────────────────────────────────

#[test]
fn missing_agent_returns_agent_not_found_with_all_six_searched_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let result = discover_agent_with_home(tmp.path(), Some(home.path()), "nonexistent");
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "nonexistent");
            assert_eq!(searched.len(), 6, "should try all 6 search locations");
            let paths: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            assert!(paths[0].contains(".agentic/agents"), "1st: {}", paths[0]);
            assert!(paths[1].contains(".claude/agents"), "2nd: {}", paths[1]);
            assert!(paths[2].contains(".github/agents"), "3rd: {}", paths[2]);
            assert!(
                paths[3].ends_with("agents/nonexistent.md"),
                "4th: {}",
                paths[3]
            );
            assert!(paths[4].contains(".claude/agents"), "5th: {}", paths[4]);
            assert!(paths[5].contains(".copilot/agents"), "6th: {}", paths[5]);
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}
