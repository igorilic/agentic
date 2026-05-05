use std::path::{Path, PathBuf};

use crate::agents::{Agent, parse_agent};
use crate::backends::BackendKind;
use crate::{CoreError, Result};

/// Locate and parse an agent file for the given `name` within `repo_root`,
/// using the user's real home directory (via [`dirs::home_dir`]) for the
/// global fallback paths.
///
/// Search order (first match wins):
///
/// **Universal (both backends):**
///   1. `<repo_root>/.agentic/agents/<name>.md` — explicit project override
///
/// **ClaudeCode:**
///   2. `<repo_root>/.claude/agents/<name>.md`  — Claude Code project convention
///   3. `$HOME/.claude/agents/<name>.md`         — Claude Code global
///
/// **CopilotCli:**
///   2. `<repo_root>/.github/agents/<name>.md`  — Copilot project convention
///   3. `$HOME/.copilot/agents/<name>.md`        — Copilot global
///
/// The legacy `<repo_root>/agents/` path is no longer searched.
///
/// Returns `CoreError::AgentNotFound` with every probed path listed in
/// `searched` (exactly 3 paths per call) if none of the candidates exist.
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
    // Universal first-priority override.
    let mut paths = vec![repo_root.join(".agentic").join("agents").join(&filename)];

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
