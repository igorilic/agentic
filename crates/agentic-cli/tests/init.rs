//! Integration tests for `agentic-cli init`.

use agentic_cli::init::{AGENT_NAMES, AgentDestination, write_agent_scaffolding};
use std::fs;

fn make_target() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn writes_all_four_agent_files_into_provided_agents_dir() {
    let tmp = make_target();
    let agents_dir = tmp.path().join(".claude").join("agents");

    let report = write_agent_scaffolding(&agents_dir, false).expect("init");

    assert_eq!(report.agents_dir, agents_dir);
    for name in AGENT_NAMES {
        let path = agents_dir.join(format!("{name}.md"));
        assert!(path.exists(), "expected {} to be created", path.display());
        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.starts_with("+++"),
            "{} should start with TOML frontmatter fence",
            path.display()
        );
        assert!(
            body.contains(&format!("name = \"{name}\"")),
            "{} should declare matching name",
            path.display()
        );
        assert!(
            body.contains("pipeline_role = \"step\""),
            "{} should declare pipeline_role = step",
            path.display()
        );
    }
    assert_eq!(report.created.len(), AGENT_NAMES.len());
}

#[test]
fn refuses_to_overwrite_existing_files_without_force() {
    let tmp = make_target();
    let agents_dir = tmp.path().join(".claude").join("agents");
    let architect = agents_dir.join("architect.md");

    write_agent_scaffolding(&agents_dir, false).unwrap();
    let original = fs::read_to_string(&architect).unwrap();

    let result = write_agent_scaffolding(&agents_dir, false);
    assert!(result.is_err(), "second init without --force must fail");

    let after = fs::read_to_string(&architect).unwrap();
    assert_eq!(
        original, after,
        "file content must not change on failed init"
    );
}

#[test]
fn force_overwrites_existing_files() {
    let tmp = make_target();
    let agents_dir = tmp.path().join(".claude").join("agents");
    let architect = agents_dir.join("architect.md");

    write_agent_scaffolding(&agents_dir, false).unwrap();
    fs::write(&architect, "stale content").unwrap();

    write_agent_scaffolding(&agents_dir, true).expect("--force should succeed");

    let body = fs::read_to_string(&architect).unwrap();
    assert!(
        body.starts_with("+++"),
        "force should restore the canonical template"
    );
}

#[test]
fn creates_parent_dirs_when_missing() {
    let tmp = make_target();
    // Deeply nested path, none of which exists yet.
    let agents_dir = tmp
        .path()
        .join("brand-new")
        .join("subdir")
        .join(".claude")
        .join("agents");

    write_agent_scaffolding(&agents_dir, false).expect("init");

    assert!(agents_dir.join("architect.md").exists());
}

// ─── AgentDestination resolution ──────────────────────────────────────────────

#[test]
fn destination_claude_repo_resolves_to_dotclaude_agents_under_repo() {
    let repo = std::path::Path::new("/tmp/some-repo");
    let resolved = AgentDestination::ClaudeRepo
        .resolve(repo, None)
        .expect("resolve");
    assert_eq!(resolved, repo.join(".claude").join("agents"));
}

#[test]
fn destination_copilot_repo_resolves_to_dotgithub_agents_under_repo() {
    let repo = std::path::Path::new("/tmp/some-repo");
    let resolved = AgentDestination::CopilotRepo
        .resolve(repo, None)
        .expect("resolve");
    assert_eq!(resolved, repo.join(".github").join("agents"));
}

#[test]
fn destination_agentic_repo_resolves_to_dotagentic_agents_under_repo() {
    let repo = std::path::Path::new("/tmp/some-repo");
    let resolved = AgentDestination::AgenticRepo
        .resolve(repo, None)
        .expect("resolve");
    assert_eq!(resolved, repo.join(".agentic").join("agents"));
}

#[test]
fn destination_claude_home_resolves_under_home_directory() {
    let repo = std::path::Path::new("/tmp/some-repo");
    let home = std::path::Path::new("/Users/test");
    let resolved = AgentDestination::ClaudeHome
        .resolve(repo, Some(home))
        .expect("resolve");
    assert_eq!(resolved, home.join(".claude").join("agents"));
}

#[test]
fn destination_copilot_home_resolves_under_home_directory() {
    let repo = std::path::Path::new("/tmp/some-repo");
    let home = std::path::Path::new("/Users/test");
    let resolved = AgentDestination::CopilotHome
        .resolve(repo, Some(home))
        .expect("resolve");
    assert_eq!(resolved, home.join(".copilot").join("agents"));
}

#[test]
fn destination_home_variants_error_when_home_is_none() {
    let repo = std::path::Path::new("/tmp/some-repo");
    assert!(AgentDestination::ClaudeHome.resolve(repo, None).is_err());
    assert!(AgentDestination::CopilotHome.resolve(repo, None).is_err());
}

// ─── G.5 regression guards ────────────────────────────────────────────────────
// These three tests guard against regressions in backend-scoped init paths.
// The legacy `<repo>/agents/` directory must never be written; each backend
// convention maps to its canonical location only.

/// `agentic-cli init` (default, no flags) must write agent files into
/// `<repo>/.claude/agents/` — the Claude Code project-local convention.
#[test]
fn init_default_writes_to_claude_agents() {
    let tmp = make_target();
    let repo_root = tmp.path();
    let destination = AgentDestination::ClaudeRepo
        .resolve(repo_root, None)
        .expect("resolve ClaudeRepo");

    write_agent_scaffolding(&destination, false).expect("init default");

    for name in AGENT_NAMES {
        let path = repo_root
            .join(".claude")
            .join("agents")
            .join(format!("{name}.md"));
        assert!(
            path.exists(),
            "default init must create {name}.md at {}",
            path.display()
        );
    }
}

/// `agentic-cli init --copilot` must write agent files into
/// `<repo>/.github/agents/` — the Copilot project-local convention.
#[test]
fn init_copilot_writes_to_github_agents() {
    let tmp = make_target();
    let repo_root = tmp.path();
    let destination = AgentDestination::CopilotRepo
        .resolve(repo_root, None)
        .expect("resolve CopilotRepo");

    write_agent_scaffolding(&destination, false).expect("init --copilot");

    for name in AGENT_NAMES {
        let path = repo_root
            .join(".github")
            .join("agents")
            .join(format!("{name}.md"));
        assert!(
            path.exists(),
            "--copilot must create {name}.md at {}",
            path.display()
        );
    }
}

/// The legacy `<repo>/agents/` path must never be written by any `init`
/// invocation. This is a dropped convention; writing there would confuse
/// users who expect agents at the backend-specific locations.
#[test]
fn init_does_not_write_to_legacy_agents_dir() {
    let tmp = make_target();
    let repo_root = tmp.path();

    // Run both the default (claude) and the --copilot variant.
    let claude_dest = AgentDestination::ClaudeRepo
        .resolve(repo_root, None)
        .expect("resolve ClaudeRepo");
    write_agent_scaffolding(&claude_dest, false).expect("init default");

    let copilot_dest = AgentDestination::CopilotRepo
        .resolve(repo_root, None)
        .expect("resolve CopilotRepo");
    write_agent_scaffolding(&copilot_dest, false).expect("init --copilot");

    // The legacy bare `<repo>/agents/` directory must not exist.
    let legacy_dir = repo_root.join("agents");
    assert!(
        !legacy_dir.exists(),
        "init must not create the legacy <repo>/agents/ directory (found {})",
        legacy_dir.display()
    );
}
