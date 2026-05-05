use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::Result;
use crate::agents::{AgentInfo, AgentSource, parse_agent};
use crate::backends::BackendKind;

/// Return every agent resolvable for `backend` under `repo_root` + `home`,
/// deduplicated by name with **project precedence over home**, each tagged
/// with [`AgentSource`].
///
/// Search order (higher priority first — first match wins for a given name):
///
/// 1. `<repo_root>/.agentic/agents/` — universal project override (`Project`)
/// 2. `<repo_root>/.claude/agents/` or `<repo_root>/.github/agents/` — backend project dir (`Project`)
/// 3. `<home>/.claude/agents/` or `<home>/.copilot/agents/` — backend home dir (`Home`)
///
/// Files that cannot be parsed are silently skipped (a `warn!` is emitted).
/// The returned list is sorted alphabetically by name.
pub fn list_discoverable(
    backend: BackendKind,
    repo_root: &Path,
    home: Option<&Path>,
) -> Result<Vec<AgentInfo>> {
    // Build the ordered list of (directory, source) pairs to search.
    let dirs = search_dirs(backend, repo_root, home);

    // Collect into a map keyed by agent name; the first entry for each name
    // wins (priority order is encoded in `dirs`).
    let mut by_name: HashMap<String, AgentInfo> = HashMap::new();

    for (dir, source) in &dirs {
        if !dir.is_dir() {
            continue;
        }
        let read_dir = match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(e) => {
                warn!("list_discoverable: cannot read dir {}: {e}", dir.display());
                continue;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_owned(),
                None => continue,
            };
            // Skip if a higher-priority directory already provided this name.
            if by_name.contains_key(&stem) {
                continue;
            }
            match load_agent_info(&path, &stem, *source) {
                Some(info) => {
                    by_name.insert(stem, info);
                }
                None => {
                    // parse error already warned inside load_agent_info
                }
            }
        }
    }

    let mut agents: Vec<AgentInfo> = by_name.into_values().collect();
    agents.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(agents)
}

/// Attempt to load an [`AgentInfo`] from `path`. Returns `None` on any error
/// (read or parse); emits a `warn!` so problems are visible in logs.
fn load_agent_info(path: &Path, stem: &str, source: AgentSource) -> Option<AgentInfo> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("list_discoverable: cannot read {}: {e}", path.display());
            return None;
        }
    };
    match parse_agent(stem, &content) {
        Ok(agent) => Some(AgentInfo {
            name: agent.name,
            description: Some(agent.description),
            source,
        }),
        Err(e) => {
            warn!(
                "list_discoverable: skipping {} (parse error: {e})",
                path.display()
            );
            None
        }
    }
}

/// Return the ordered list of `(directory, AgentSource)` pairs to search for
/// the given backend. Higher-priority directories appear first.
fn search_dirs(
    backend: BackendKind,
    repo_root: &Path,
    home: Option<&Path>,
) -> Vec<(PathBuf, AgentSource)> {
    let mut dirs: Vec<(PathBuf, AgentSource)> = Vec::new();

    // 1. Universal project override (highest priority).
    dirs.push((
        repo_root.join(".agentic").join("agents"),
        AgentSource::Project,
    ));

    // 2. Backend-specific project directory.
    match backend {
        BackendKind::ClaudeCode => {
            dirs.push((
                repo_root.join(".claude").join("agents"),
                AgentSource::Project,
            ));
        }
        BackendKind::CopilotCli => {
            dirs.push((
                repo_root.join(".github").join("agents"),
                AgentSource::Project,
            ));
        }
    }

    // 3. Backend-specific home directory (lowest priority).
    if let Some(home) = home {
        match backend {
            BackendKind::ClaudeCode => {
                dirs.push((home.join(".claude").join("agents"), AgentSource::Home));
            }
            BackendKind::CopilotCli => {
                dirs.push((home.join(".copilot").join("agents"), AgentSource::Home));
            }
        }
    }

    dirs
}
