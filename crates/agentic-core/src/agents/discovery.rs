use std::path::{Path, PathBuf};

use crate::agents::{parse_agent, Agent};
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
pub fn discover_agent(_repo_root: &Path, _name: &str) -> Result<Agent> {
    unimplemented!()
}
