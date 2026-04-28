use std::path::{Path, PathBuf};

use crate::agents::{Agent, parse_agent};
use crate::{CoreError, Result};

/// Locate and parse an agent file for the given `name` within `repo_root`,
/// using the user's real home directory (via [`dirs::home_dir`]) for the
/// global fallback paths.
///
/// Search order (first match wins):
///   1. `<repo_root>/.agentic/agents/<name>.md`   — explicit project override
///   2. `<repo_root>/.claude/agents/<name>.md`    — Claude Code convention,
///      shared with Claude Code itself
///   3. `<repo_root>/.github/agents/<name>.md`    — Copilot project convention
///   4. `<repo_root>/agents/<name>.md`            — legacy
///   5. `$HOME/.claude/agents/<name>.md`          — Claude Code global
///   6. `$HOME/.copilot/agents/<name>.md`         — Copilot global
///
/// Returns `CoreError::AgentNotFound` with every probed path listed in
/// `searched` if none of the candidates exist.
pub fn discover_agent(repo_root: &Path, name: &str) -> Result<Agent> {
    let base = directories::BaseDirs::new();
    let home = base.as_ref().map(|b| b.home_dir());
    discover_agent_with_home(repo_root, home, name)
}

/// Same as [`discover_agent`] but with an injectable home directory. Use
/// this in tests so they don't see the developer's real `~/.claude/`.
pub fn discover_agent_with_home(
    repo_root: &Path,
    home: Option<&Path>,
    name: &str,
) -> Result<Agent> {
    let candidates = candidate_paths(repo_root, home, name);

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

fn candidate_paths(repo_root: &Path, home: Option<&Path>, name: &str) -> Vec<PathBuf> {
    let filename = format!("{name}.md");
    let mut paths = vec![
        repo_root.join(".agentic").join("agents").join(&filename),
        repo_root.join(".claude").join("agents").join(&filename),
        repo_root.join(".github").join("agents").join(&filename),
        repo_root.join("agents").join(&filename),
    ];
    if let Some(home) = home {
        paths.push(home.join(".claude").join("agents").join(&filename));
        paths.push(home.join(".copilot").join("agents").join(&filename));
    }
    paths
}
