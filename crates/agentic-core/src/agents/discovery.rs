use std::path::{Path, PathBuf};

use crate::agents::{Agent, parse_agent};
use crate::{CoreError, Result};

/// Locate and parse an agent file for the given `name` within `repo_root`.
///
/// Search order per spec §10.2 (first match wins):
///   1. `<repo_root>/.agentic/agents/<name>.md`
///   2. `<repo_root>/.claude/agents/<name>.md`
///   3. `<repo_root>/agents/<name>.md`
///
/// Returns `CoreError::AgentNotFound` with all three paths listed in
/// `searched` if none of the candidates exist.
pub fn discover_agent(repo_root: &Path, name: &str) -> Result<Agent> {
    let filename = format!("{name}.md");
    let candidates: Vec<PathBuf> = vec![
        repo_root.join(".agentic").join("agents").join(&filename),
        repo_root.join(".claude").join("agents").join(&filename),
        repo_root.join("agents").join(&filename),
    ];

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
