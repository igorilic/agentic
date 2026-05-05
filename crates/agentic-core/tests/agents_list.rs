/// Integration tests for `list_discoverable` — the iteration surface for
/// all discoverable agents within a repo + home directory pair.
///
/// Each test isolates itself in a temporary directory and injects a fake
/// home directory so the developer's real `~/.claude/` never bleeds in.
use std::path::Path;

use agentic_core::{AgentSource, BackendKind, list_discoverable};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Write a minimal valid agent markdown file (TOML frontmatter style).
fn write_agent(root: &Path, subdir: &str, filename: &str, name: &str, description: &str) {
    let dir = root.join(subdir);
    std::fs::create_dir_all(&dir).expect("create_dir_all");
    let content =
        format!("+++\nname = \"{name}\"\ndescription = \"{description}\"\n+++\nSystem prompt.\n");
    std::fs::write(dir.join(filename), content).expect("write agent file");
}

/// Write a file with intentionally broken frontmatter (no closing `+++`).
fn write_malformed_agent(root: &Path, subdir: &str, filename: &str) {
    let dir = root.join(subdir);
    std::fs::create_dir_all(&dir).expect("create_dir_all");
    // Missing closing `+++` — should cause a parse error.
    let content = "+++\nname = \"broken\"\ndescription = \"malformed\"\n";
    std::fs::write(dir.join(filename), content).expect("write malformed agent file");
}

// ─── 1. Empty result when no agents anywhere ─────────────────────────────────

#[test]
fn list_discoverable_returns_empty_when_no_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable should not error");

    assert!(
        result.is_empty(),
        "Expected empty list when no agent files exist, got: {result:?}"
    );
}

// ─── 2. Project agents returned in alphabetical order ────────────────────────

#[test]
fn list_discoverable_returns_project_agents_alphabetical() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(tmp.path(), ".claude/agents", "zebra.md", "zebra", "Z agent");
    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "A agent",
    );
    write_agent(tmp.path(), ".claude/agents", "qa.md", "qa", "Q agent");

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    let names: Vec<&str> = result.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["architect", "qa", "zebra"],
        "Agents should be sorted alphabetically; got: {names:?}"
    );
    // All should be Project-sourced.
    for agent in &result {
        assert_eq!(
            agent.source,
            AgentSource::Project,
            "{} should have source=Project",
            agent.name
        );
    }
}

// ─── 3. description extracted from frontmatter ───────────────────────────────

#[test]
fn list_discoverable_returns_project_file_with_source_project() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "Plans the work",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert_eq!(result.len(), 1);
    let info = &result[0];
    assert_eq!(info.name, "architect");
    assert_eq!(info.source, AgentSource::Project);
    assert_eq!(
        info.description.as_deref(),
        Some("Plans the work"),
        "description should be extracted from frontmatter"
    );
}

// ─── 4. Home-only agent returns source = Home ────────────────────────────────

#[test]
fn list_discoverable_returns_home_file_with_source_home_when_project_absent() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".claude/agents",
        "qa.md",
        "qa",
        "Quality assurance",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert_eq!(result.len(), 1);
    let info = &result[0];
    assert_eq!(info.name, "qa");
    assert_eq!(info.source, AgentSource::Home);
    assert_eq!(info.description.as_deref(), Some("Quality assurance"));
}

// ─── 5. Malformed file skipped, other valid agents returned ──────────────────

#[test]
fn list_discoverable_skips_malformed_files_keeps_valid_ones() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "Good agent",
    );
    write_malformed_agent(tmp.path(), ".claude/agents", "qa.md");

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable should not fail even with malformed files");

    let names: Vec<&str> = result.iter().map(|a| a.name.as_str()).collect();
    assert!(
        names.contains(&"architect"),
        "Valid agent 'architect' should be in results; got: {names:?}"
    );
    assert!(
        !names.contains(&"qa"),
        "Malformed agent 'qa' should be skipped; got: {names:?}"
    );
}

// ─── 6. Project precedence over home (name collision) ────────────────────────

#[test]
fn list_discoverable_project_precedence_wins_over_home() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Same name in both project and home with different descriptions.
    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "project version",
    );
    write_agent(
        home.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "home version",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert_eq!(
        result.len(),
        1,
        "Duplicate name should produce exactly one result; got: {result:?}"
    );
    let info = &result[0];
    assert_eq!(info.source, AgentSource::Project, "Project should win");
    assert_eq!(
        info.description.as_deref(),
        Some("project version"),
        "Project description should win; got: {:?}",
        info.description
    );
}

// ─── 7. .agentic/ universal override beats backend-specific dirs ─────────────

#[test]
fn list_discoverable_agentic_universal_override_beats_both_backend_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".agentic/agents",
        "architect.md",
        "architect",
        "agentic version",
    );
    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "claude version",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert_eq!(
        result.len(),
        1,
        "Should deduplicate when same name appears in multiple dirs; got: {result:?}"
    );
    let info = &result[0];
    assert_eq!(
        info.source,
        AgentSource::Project,
        "Should be Project source"
    );
    assert_eq!(
        info.description.as_deref(),
        Some("agentic version"),
        ".agentic/agents/ description should win; got: {:?}",
        info.description
    );
}

// ─── 8. Backend scoping: CopilotCli cannot see .claude/agents/ ───────────────

#[test]
fn list_discoverable_copilot_cli_cannot_see_claude_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(tmp.path(), ".claude/agents", "qa.md", "qa", "Claude only");

    let result = list_discoverable(BackendKind::CopilotCli, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert!(
        result.is_empty(),
        "CopilotCli should not see .claude/agents/; got: {result:?}"
    );
}

// ─── 9. Backend scoping: ClaudeCode cannot see .github/agents/ ───────────────

#[test]
fn list_discoverable_claude_code_cannot_see_github_agents() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".github/agents",
        "reviewer.md",
        "reviewer",
        "Copilot only",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert!(
        result.is_empty(),
        "ClaudeCode should not see .github/agents/; got: {result:?}"
    );
}

// ─── 10. .agentic/ visible to both backends ──────────────────────────────────

#[test]
fn list_discoverable_agentic_dir_visible_to_both_backends() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".agentic/agents",
        "orchestrator.md",
        "orchestrator",
        "Universal agent",
    );

    for backend in [BackendKind::ClaudeCode, BackendKind::CopilotCli] {
        let result =
            list_discoverable(backend, tmp.path(), Some(home.path())).expect("list_discoverable");

        assert_eq!(
            result.len(),
            1,
            "{backend:?}: .agentic/agents/ should be visible; got: {result:?}"
        );
        assert_eq!(result[0].source, AgentSource::Project);
        assert_eq!(result[0].name, "orchestrator");
    }
}

// ─── 11. Home agents with no project match returned with source = Home ────────

#[test]
fn list_discoverable_returns_home_only_agents_when_no_project_match() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".claude/agents",
        "tdd-developer.md",
        "tdd-developer",
        "TDD specialist",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "tdd-developer");
    assert_eq!(result[0].source, AgentSource::Home);
}

// ─── 12. Empty result when home is None ──────────────────────────────────────

#[test]
fn list_discoverable_returns_empty_when_no_home_and_no_project_agents() {
    let tmp = tempfile::tempdir().unwrap();

    let result =
        list_discoverable(BackendKind::ClaudeCode, tmp.path(), None).expect("list_discoverable");

    assert!(
        result.is_empty(),
        "Should be empty with no agents: {result:?}"
    );
}

// ─── 13. Mixed project + home agents, all appear in alphabetical order ────────

#[test]
fn list_discoverable_mixed_project_and_home_sorted_alphabetically() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        tmp.path(),
        ".claude/agents",
        "reviewer.md",
        "reviewer",
        "Project reviewer",
    );
    write_agent(
        home.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "Home architect",
    );

    let result = list_discoverable(BackendKind::ClaudeCode, tmp.path(), Some(home.path()))
        .expect("list_discoverable");

    let names: Vec<&str> = result.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["architect", "reviewer"],
        "Mixed agents should be sorted alphabetically; got: {names:?}"
    );
    assert_eq!(result[0].source, AgentSource::Home);
    assert_eq!(result[1].source, AgentSource::Project);
}
