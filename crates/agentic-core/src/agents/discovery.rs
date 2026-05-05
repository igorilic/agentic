use std::path::{Path, PathBuf};

use crate::agents::{Agent, parse_agent};
use crate::backends::BackendKind;
use crate::{CoreError, Result};

/// Locate and parse an agent file for the given `name` within `repo_root`,
/// using the user's real home directory (via [`dirs::home_dir`]) for the
/// global fallback paths.
///
/// Strict 2-path scoping per backend (first match wins):
///
/// **ClaudeCode:**
///   1. `<repo_root>/.claude/agents/<name>.md`  — Claude Code project convention
///   2. `$HOME/.claude/agents/<name>.md`         — Claude Code global
///
/// **CopilotCli:**
///   1. `<repo_root>/.github/agents/<name>.md`  — Copilot project convention
///   2. `$HOME/.copilot/agents/<name>.md`        — Copilot global
///
/// Returns `CoreError::AgentNotFound` with every probed path listed in
/// `searched` (exactly 2 paths per call) if none of the candidates exist.
pub fn discover_agent(backend: BackendKind, repo_root: &Path, name: &str) -> Result<Agent> {
    let base = directories::BaseDirs::new();
    let home = base.as_ref().map(|b| b.home_dir());
    discover_agent_with_home(backend, repo_root, home, name)
}

/// Same as [`discover_agent`] but with an injectable home directory. Use
/// this in tests so they don't see the developer's real `~/.claude/`.
pub fn discover_agent_with_home(
    backend: BackendKind,
    repo_root: &Path,
    home: Option<&Path>,
    name: &str,
) -> Result<Agent> {
    let candidates = candidate_paths(backend, repo_root, home, name);

    for path in &candidates {
        if path.is_file() {
            let content = std::fs::read_to_string(path)?;
            return parse_agent(name, &content);
        }
    }

    Err(CoreError::AgentNotFound {
        name: name.to_string(),
        searched: candidates,
    })
}

fn candidate_paths(
    backend: BackendKind,
    repo_root: &Path,
    home: Option<&Path>,
    name: &str,
) -> Vec<PathBuf> {
    let filename = format!("{name}.md");
    // Strict 2-path scoping: project dir first, then home dir for the backend.
    let mut paths: Vec<PathBuf> = Vec::new();

    match backend {
        BackendKind::ClaudeCode => {
            paths.push(repo_root.join(".claude").join("agents").join(&filename));
            if let Some(home) = home {
                paths.push(home.join(".claude").join("agents").join(&filename));
            }
        }
        BackendKind::CopilotCli => {
            paths.push(repo_root.join(".github").join("agents").join(&filename));
            if let Some(home) = home {
                paths.push(home.join(".copilot").join("agents").join(&filename));
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn stub_content(name: &str) -> String {
        format!(
            "+++\nname = \"{name}\"\ndescription = \"stub\"\npipeline_role = \"step\"\n+++\nbody"
        )
    }

    fn make_temp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    /// `<repo>/.github/agents/spec-writer.agent.md` is found when
    /// `spec-writer.md` does not exist (project dir, CopilotCli).
    #[test]
    fn it_resolves_dot_agent_md_when_dot_md_absent_project() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".github").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(
            agents_dir.join("spec-writer.agent.md"),
            stub_content("spec-writer"),
        )
        .unwrap();

        let result = discover_agent_with_home(
            BackendKind::CopilotCli,
            repo.path(),
            Some(home.path()),
            "spec-writer",
        );
        assert!(
            result.is_ok(),
            "should resolve spec-writer.agent.md from project dir; got: {:?}",
            result
        );
    }

    /// `$HOME/.copilot/agents/spec-writer.agent.md` is found when
    /// `spec-writer.md` does not exist anywhere (home dir, CopilotCli).
    #[test]
    fn it_resolves_dot_agent_md_when_dot_md_absent_home() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = home.path().join(".copilot").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(
            agents_dir.join("spec-writer.agent.md"),
            stub_content("spec-writer"),
        )
        .unwrap();

        let result = discover_agent_with_home(
            BackendKind::CopilotCli,
            repo.path(),
            Some(home.path()),
            "spec-writer",
        );
        assert!(
            result.is_ok(),
            "should resolve spec-writer.agent.md from home dir; got: {:?}",
            result
        );
    }

    /// When both `spec-writer.md` and `spec-writer.agent.md` exist in the
    /// project dir, the plain `.md` file (written first in candidate_paths)
    /// must win.
    #[test]
    fn it_prefers_dot_md_over_dot_agent_md_in_same_dir() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".github").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        // Plain .md body is "plain-body"; .agent.md body is "agent-body".
        std::fs::write(
            agents_dir.join("spec-writer.md"),
            "+++\nname = \"spec-writer\"\ndescription = \"plain\"\npipeline_role = \"step\"\n+++\nplain-body",
        )
        .unwrap();
        std::fs::write(
            agents_dir.join("spec-writer.agent.md"),
            "+++\nname = \"spec-writer\"\ndescription = \"agent\"\npipeline_role = \"step\"\n+++\nagent-body",
        )
        .unwrap();

        let agent = discover_agent_with_home(
            BackendKind::CopilotCli,
            repo.path(),
            Some(home.path()),
            "spec-writer",
        )
        .expect("should resolve");

        assert_eq!(
            agent.description, "plain",
            ".md should win over .agent.md; got description: {}",
            agent.description
        );
    }

    /// `.agent.md` variant is also discovered for the ClaudeCode backend.
    #[test]
    fn it_resolves_dot_agent_md_for_claude_code_backend() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        std::fs::write(agents_dir.join("foo.agent.md"), stub_content("foo")).unwrap();

        let result = discover_agent_with_home(
            BackendKind::ClaudeCode,
            repo.path(),
            Some(home.path()),
            "foo",
        );
        assert!(
            result.is_ok(),
            "should resolve foo.agent.md for ClaudeCode; got: {:?}",
            result
        );
    }

    /// When no file exists the `searched` field must list exactly 4 paths
    /// in the correct order: project .md, project .agent.md, home .md, home .agent.md.
    #[test]
    fn error_lists_all_four_candidate_paths_when_none_match() {
        let repo = make_temp();
        let home = make_temp();

        let err = discover_agent_with_home(
            BackendKind::CopilotCli,
            repo.path(),
            Some(home.path()),
            "spec-writer",
        )
        .expect_err("should fail: no files present");

        let searched = match err {
            crate::CoreError::AgentNotFound { searched, .. } => searched,
            other => panic!("expected AgentNotFound; got: {:?}", other),
        };

        assert_eq!(
            searched.len(),
            4,
            "expected 4 candidate paths, got {}: {:?}",
            searched.len(),
            searched
        );

        let project_md = repo
            .path()
            .join(".github")
            .join("agents")
            .join("spec-writer.md");
        let project_agent_md = repo
            .path()
            .join(".github")
            .join("agents")
            .join("spec-writer.agent.md");
        let home_md = home
            .path()
            .join(".copilot")
            .join("agents")
            .join("spec-writer.md");
        let home_agent_md = home
            .path()
            .join(".copilot")
            .join("agents")
            .join("spec-writer.agent.md");

        assert_eq!(searched[0], project_md, "path[0] should be project .md");
        assert_eq!(
            searched[1], project_agent_md,
            "path[1] should be project .agent.md"
        );
        assert_eq!(searched[2], home_md, "path[2] should be home .md");
        assert_eq!(
            searched[3], home_agent_md,
            "path[3] should be home .agent.md"
        );
    }
}
