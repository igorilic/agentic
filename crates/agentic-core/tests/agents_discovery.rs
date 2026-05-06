use std::path::Path;

use agentic_core::{BackendKind, CoreError, discover_agent, discover_agent_with_home};

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

// ─── strict 2-path scoping: .agentic/ is NOT searched ───────────────────────

#[test]
fn discover_strict_2_path_scope_excludes_agentic_dir_for_claude() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap(); // empty — no global agents

    // Agent ONLY in .agentic/agents/ — strict scoping means this is NOT found.
    write_agent(
        root,
        ".agentic/agents",
        "architect.md",
        "architect",
        "AGENTIC",
    );

    let result = discover_agent_with_home(
        BackendKind::ClaudeCode,
        root,
        Some(home.path()),
        "architect",
    );
    match result {
        Err(CoreError::AgentNotFound { .. }) => {}
        Ok(a) => panic!(
            ".agentic/agents/ should NOT be searched under strict 2-path scope (ClaudeCode); got: {}",
            a.description
        ),
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn discover_strict_2_path_scope_excludes_agentic_dir_for_copilot() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        root,
        ".agentic/agents",
        "architect.md",
        "architect",
        "AGENTIC",
    );

    let result = discover_agent_with_home(
        BackendKind::CopilotCli,
        root,
        Some(home.path()),
        "architect",
    );
    match result {
        Err(CoreError::AgentNotFound { .. }) => {}
        Ok(a) => panic!(
            ".agentic/agents/ should NOT be searched under strict 2-path scope (CopilotCli); got: {}",
            a.description
        ),
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

// ─── ClaudeCode: project-level .claude/agents/ ───────────────────────────────

#[test]
fn claude_backend_finds_project_claude_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        root,
        ".claude/agents",
        "architect.md",
        "architect",
        "CLAUDE",
    );

    let agent = discover_agent_with_home(
        BackendKind::ClaudeCode,
        root,
        Some(home.path()),
        "architect",
    )
    .expect("discover");
    assert!(
        agent.description.contains("CLAUDE"),
        ".claude/agents/ should be found for ClaudeCode; got: {}",
        agent.description
    );
}

#[test]
fn claude_backend_does_not_find_github_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    // Only place the agent in .github — ClaudeCode should NOT see it.
    write_agent(
        root,
        ".github/agents",
        "architect.md",
        "architect",
        "GITHUB",
    );

    let result = discover_agent_with_home(
        BackendKind::ClaudeCode,
        root,
        Some(home.path()),
        "architect",
    );
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "architect");
            let paths: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            // Error must list Claude paths but NOT .github
            assert!(
                paths.iter().any(|p| p.contains(".claude/agents")),
                "error should list .claude/agents path; got: {paths:?}"
            );
            assert!(
                !paths.iter().any(|p| p.contains(".github/agents")),
                "error should NOT list .github/agents path for ClaudeCode; got: {paths:?}"
            );
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

// ─── CopilotCli: project-level .github/agents/ ───────────────────────────────

#[test]
fn copilot_backend_finds_project_github_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        root,
        ".github/agents",
        "architect.md",
        "architect",
        "GITHUB",
    );

    let agent = discover_agent_with_home(
        BackendKind::CopilotCli,
        root,
        Some(home.path()),
        "architect",
    )
    .expect("discover");
    assert!(
        agent.description.contains("GITHUB"),
        ".github/agents/ should be found for CopilotCli; got: {}",
        agent.description
    );
}

#[test]
fn copilot_backend_does_not_find_claude_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    // Only place the agent in .claude — CopilotCli should NOT see it.
    write_agent(
        root,
        ".claude/agents",
        "architect.md",
        "architect",
        "CLAUDE",
    );

    let result = discover_agent_with_home(
        BackendKind::CopilotCli,
        root,
        Some(home.path()),
        "architect",
    );
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "architect");
            let paths: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            // Error must list Copilot paths but NOT .claude
            assert!(
                paths.iter().any(|p| p.contains(".github/agents")),
                "error should list .github/agents path; got: {paths:?}"
            );
            assert!(
                !paths.iter().any(|p| p.contains(".claude/agents")),
                "error should NOT list .claude/agents path for CopilotCli; got: {paths:?}"
            );
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

// ─── .agentic/ is NOT searched (strict 2-path scoping for both backends) ─────

#[test]
fn agentic_dir_not_searched_for_either_backend() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    // Agent only in .agentic/agents/ — should NOT be found by either backend.
    write_agent(
        root,
        ".agentic/agents",
        "architect.md",
        "architect",
        "AGENTIC",
    );

    for backend in [BackendKind::ClaudeCode, BackendKind::CopilotCli] {
        let result = discover_agent_with_home(backend, root, Some(home.path()), "architect");
        match result {
            Err(CoreError::AgentNotFound { .. }) => {}
            Ok(a) => panic!(
                "{backend:?}: .agentic/agents/ should NOT be searched; got: {}",
                a.description
            ),
            Err(other) => panic!("{backend:?}: unexpected error: {other:?}"),
        }
    }
}

// ─── home-dir fallback per backend ───────────────────────────────────────────

#[test]
fn claude_backend_falls_through_to_home_claude() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "HOME_CLAUDE",
    );

    let agent = discover_agent_with_home(
        BackendKind::ClaudeCode,
        tmp.path(),
        Some(home.path()),
        "architect",
    )
    .expect("discover");
    assert!(
        agent.description.contains("HOME_CLAUDE"),
        "$HOME/.claude/agents/ should be found as ClaudeCode fallback; got: {}",
        agent.description
    );
}

#[test]
fn copilot_backend_falls_through_to_home_copilot() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    write_agent(
        home.path(),
        ".copilot/agents",
        "architect.md",
        "architect",
        "HOME_COPILOT",
    );

    let agent = discover_agent_with_home(
        BackendKind::CopilotCli,
        tmp.path(),
        Some(home.path()),
        "architect",
    )
    .expect("discover");
    assert!(
        agent.description.contains("HOME_COPILOT"),
        "$HOME/.copilot/agents/ should be found as CopilotCli fallback; got: {}",
        agent.description
    );
}

#[test]
fn claude_backend_does_not_find_home_copilot_agent() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Only place the agent in $HOME/.copilot — ClaudeCode should NOT see it.
    write_agent(
        home.path(),
        ".copilot/agents",
        "architect.md",
        "architect",
        "HOME_COPILOT",
    );

    let result = discover_agent_with_home(
        BackendKind::ClaudeCode,
        tmp.path(),
        Some(home.path()),
        "architect",
    );
    match result {
        Err(CoreError::AgentNotFound { .. }) => {
            // Correct: ClaudeCode does not search $HOME/.copilot/
        }
        Ok(_) => panic!("expected AgentNotFound: ClaudeCode should not see $HOME/.copilot/"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

// ─── cross-backend isolation: repo-local ─────────────────────────────────────

#[test]
fn claude_project_agent_invisible_to_copilot_cli() {
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

    let result = discover_agent_with_home(
        BackendKind::CopilotCli,
        root,
        Some(home.path()),
        "tdd-developer",
    );
    match result {
        Err(CoreError::AgentNotFound { .. }) => {}
        Ok(_) => panic!("CopilotCli should not see .claude/agents/"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

#[test]
fn github_project_agent_invisible_to_claude_code() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    write_agent(root, ".github/agents", "qa.md", "qa", "GITHUB");

    let result = discover_agent_with_home(BackendKind::ClaudeCode, root, Some(home.path()), "qa");
    match result {
        Err(CoreError::AgentNotFound { .. }) => {}
        Ok(_) => panic!("ClaudeCode should not see .github/agents/"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

// ─── legacy <repo>/agents/ is ignored ────────────────────────────────────────

#[test]
fn legacy_repo_agents_dir_is_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = tempfile::tempdir().unwrap();

    // Write agent ONLY in the legacy location.
    write_agent(root, "agents", "architect.md", "architect", "LEGACY");

    // Both backends should return AgentNotFound.
    for backend in [BackendKind::ClaudeCode, BackendKind::CopilotCli] {
        let result = discover_agent_with_home(backend, root, Some(home.path()), "architect");
        match result {
            Err(CoreError::AgentNotFound { .. }) => {}
            Ok(_) => panic!("{backend:?}: legacy <repo>/agents/ should be ignored"),
            Err(other) => panic!("{backend:?}: expected AgentNotFound, got {other:?}"),
        }
    }
}

// ─── repo-local beats home-local (ClaudeCode) ────────────────────────────────

#[test]
fn repo_claude_beats_home_claude() {
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

    let agent = discover_agent_with_home(
        BackendKind::ClaudeCode,
        root,
        Some(home.path()),
        "architect",
    )
    .expect("discover");
    assert!(
        agent.description.contains("REPO_CLAUDE"),
        "repo-local .claude/agents/ should beat $HOME; got: {}",
        agent.description
    );
}

// ─── default discover_agent (real home) — must not panic ─────────────────────

#[test]
fn default_discover_agent_resolves_real_home_without_panicking_claude() {
    let tmp = tempfile::tempdir().unwrap();
    write_agent(
        tmp.path(),
        ".claude/agents",
        "architect.md",
        "architect",
        "REPO",
    );
    let agent = discover_agent(BackendKind::ClaudeCode, tmp.path(), "architect").expect("discover");
    assert_eq!(agent.name, "architect");
}

#[test]
fn default_discover_agent_resolves_real_home_without_panicking_copilot() {
    let tmp = tempfile::tempdir().unwrap();
    write_agent(
        tmp.path(),
        ".github/agents",
        "architect.md",
        "architect",
        "REPO",
    );
    let agent = discover_agent(BackendKind::CopilotCli, tmp.path(), "architect").expect("discover");
    assert_eq!(agent.name, "architect");
}

// ─── error lists exactly the backend's 2 paths (strict 2-path scoping) ───────

#[test]
fn error_lists_all_searched_paths_for_claude_backend() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let result = discover_agent_with_home(
        BackendKind::ClaudeCode,
        tmp.path(),
        Some(home.path()),
        "nonexistent",
    );
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "nonexistent");
            assert_eq!(
                searched.len(),
                4,
                "ClaudeCode should search 4 paths (project+home × .md+.agent.md, no .agentic/); got: {searched:?}"
            );
            let paths: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            assert!(
                paths.iter().all(|p| p.contains(".claude/agents")),
                "all paths under .claude/agents; got: {paths:?}"
            );
            // Must NOT contain .agentic, .github or .copilot paths
            assert!(
                !paths.iter().any(|p| p.contains(".agentic")),
                "should not list .agentic paths; got: {paths:?}"
            );
            assert!(
                !paths.iter().any(|p| p.contains(".github")),
                "should not list .github paths; got: {paths:?}"
            );
            assert!(
                !paths.iter().any(|p| p.contains(".copilot")),
                "should not list .copilot paths; got: {paths:?}"
            );
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}

#[test]
fn error_lists_all_searched_paths_for_copilot_backend() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let result = discover_agent_with_home(
        BackendKind::CopilotCli,
        tmp.path(),
        Some(home.path()),
        "nonexistent",
    );
    match result {
        Err(CoreError::AgentNotFound { name, searched }) => {
            assert_eq!(name, "nonexistent");
            assert_eq!(
                searched.len(),
                4,
                "CopilotCli should search 4 paths (project .github + home .copilot × .md+.agent.md, no .agentic/); got: {searched:?}"
            );
            let paths: Vec<String> = searched
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            assert!(
                paths.iter().any(|p| p.contains(".github/agents")),
                "should contain project .github/agents; got: {paths:?}"
            );
            assert!(
                paths.iter().any(|p| p.contains(".copilot/agents")),
                "should contain home .copilot/agents; got: {paths:?}"
            );
            // Must NOT contain .agentic or .claude paths
            assert!(
                !paths.iter().any(|p| p.contains(".agentic")),
                "should not list .agentic paths; got: {paths:?}"
            );
            assert!(
                !paths.iter().any(|p| p.contains(".claude")),
                "should not list .claude paths for CopilotCli; got: {paths:?}"
            );
        }
        Ok(_) => panic!("expected AgentNotFound, got Ok"),
        Err(other) => panic!("expected AgentNotFound, got {other:?}"),
    }
}
