use std::path::{Path, PathBuf};

use crate::agents::{Agent, parse_agent};
use crate::backends::BackendKind;
use crate::{CoreError, Result};

/// Locate and parse an agent file for the given `name` within `repo_root`,
/// using the user's real home directory (via [`dirs::home_dir`]) for the
/// global fallback paths.
///
/// Both `<name>.md` and `<name>.agent.md` are tried in each directory so
/// that agent files following either the Claude-style convention (`foo.md`)
/// or the `.agent.md` convention (`foo.agent.md`) are discovered. The `.md`
/// variant is tried first within each directory (first match wins).
///
/// **ClaudeCode** (4 candidates in priority order):
///   1. `<repo_root>/.claude/agents/<name>.md`
///   2. `<repo_root>/.claude/agents/<name>.agent.md`
///   3. `$HOME/.claude/agents/<name>.md`
///   4. `$HOME/.claude/agents/<name>.agent.md`
///
/// **CopilotCli** (4 candidates in priority order):
///   1. `<repo_root>/.github/agents/<name>.md`
///   2. `<repo_root>/.github/agents/<name>.agent.md`
///   3. `$HOME/.copilot/agents/<name>.md`
///   4. `$HOME/.copilot/agents/<name>.agent.md`
///
/// Returns `CoreError::AgentNotFound` with all probed paths listed in
/// `searched` if none of the candidates exist.
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
    // Project dir first, home dir second. Within each dir, .md before .agent.md.
    let mut paths: Vec<PathBuf> = Vec::new();

    match backend {
        BackendKind::ClaudeCode => {
            let project_dir = repo_root.join(".claude").join("agents");
            paths.extend(paths_for_dir(&project_dir, name));
            if let Some(home) = home {
                let home_dir = home.join(".claude").join("agents");
                paths.extend(paths_for_dir(&home_dir, name));
            }
        }
        BackendKind::CopilotCli => {
            let project_dir = repo_root.join(".github").join("agents");
            paths.extend(paths_for_dir(&project_dir, name));
            if let Some(home) = home {
                let home_dir = home.join(".copilot").join("agents");
                paths.extend(paths_for_dir(&home_dir, name));
            }
        }
    }

    paths
}

/// Return `[<dir>/<name>.md, <dir>/<name>.agent.md]`. The `.md` variant
/// appears first so the first-match-wins loop in [`discover_agent_with_home`]
/// always prefers the plain extension.
fn paths_for_dir(dir: &Path, name: &str) -> [PathBuf; 2] {
    [
        dir.join(format!("{name}.md")),
        dir.join(format!("{name}.agent.md")),
    ]
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

    /// A `.agent.md` file using YAML frontmatter (`---` fences) must be
    /// resolved by `discover_agent_with_home` with a fallback Agent rather
    /// than propagating a parse error. The fallback carries the full file
    /// content as `system_prompt` and an empty `description`.
    #[test]
    fn it_resolves_yaml_frontmatter_file_with_default_metadata() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".github").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        // Mirrors the user's actual file format.
        let yaml_content = "\
---
description: \"Writes implementation-ready specs\"
tools: [read, edit, search, todo, agent]
model: \"Claude Opus 4.6\"
---
You are a spec-writer agent.
";
        std::fs::write(agents_dir.join("spec-writer.agent.md"), yaml_content).unwrap();

        let result = discover_agent_with_home(
            BackendKind::CopilotCli,
            repo.path(),
            Some(home.path()),
            "spec-writer",
        );
        assert!(
            result.is_ok(),
            "YAML-frontmatter agent file must resolve without error; got: {:?}",
            result
        );
        let agent = result.unwrap();
        assert_eq!(agent.name, "spec-writer", "fallback name must equal filename stem");
        assert!(
            agent.description.is_empty(),
            "fallback description must be empty (no TOML parsed); got: {:?}",
            agent.description
        );
        assert!(
            agent.system_prompt.contains("---"),
            "system_prompt must contain the full file content including YAML fences; got: {:?}",
            agent.system_prompt
        );
    }

    /// A plain markdown file with no frontmatter fence at all (neither `+++`
    /// nor `---`) must be resolved with a fallback Agent whose `system_prompt`
    /// is the entire file content.
    #[test]
    fn it_resolves_fenceless_markdown_file() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        let content = "You are a plain agent with no frontmatter.";
        std::fs::write(agents_dir.join("foo.md"), content).unwrap();

        let result = discover_agent_with_home(
            BackendKind::ClaudeCode,
            repo.path(),
            Some(home.path()),
            "foo",
        );
        assert!(
            result.is_ok(),
            "fenceless markdown file must resolve without error; got: {:?}",
            result
        );
        let agent = result.unwrap();
        assert_eq!(agent.name, "foo");
        assert!(
            agent.system_prompt.contains("plain agent"),
            "system_prompt must contain the file body; got: {:?}",
            agent.system_prompt
        );
    }

    /// A file that starts with `+++` (user intends TOML) but is missing the
    /// closing `+++` fence must still propagate a parse error — we do not
    /// silently fall back for malformed-but-fenced TOML files.
    #[test]
    fn it_propagates_malformed_toml_frontmatter_with_missing_close() {
        let repo = make_temp();
        let home = make_temp();
        let agents_dir = repo.path().join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();
        // Starts with +++ but has no closing fence.
        let content = "+++\nname = \"bad\"\ndescription = \"missing close\"\n";
        std::fs::write(agents_dir.join("bad.md"), content).unwrap();

        let result = discover_agent_with_home(
            BackendKind::ClaudeCode,
            repo.path(),
            Some(home.path()),
            "bad",
        );
        assert!(
            result.is_err(),
            "malformed TOML (missing closing fence) must propagate parse error; got Ok"
        );
        match result.unwrap_err() {
            crate::CoreError::Parse(_) => {}
            other => panic!("expected CoreError::Parse; got: {:?}", other),
        }
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
