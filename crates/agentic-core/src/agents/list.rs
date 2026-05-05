use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::Result;
use crate::agents::{AgentInfo, AgentSource, parse_agent};
use crate::backends::BackendKind;

/// Derive the canonical agent name from a filesystem stem. Strips a trailing
/// `.agent` suffix so that both `architect.md` (stem=`architect`) and
/// `architect.agent.md` (stem=`architect.agent`) resolve to `architect`.
fn canonical_name(stem: &str) -> String {
    stem.strip_suffix(".agent").unwrap_or(stem).to_string()
}

/// Return every agent resolvable for `backend` under `repo_root` + `home`,
/// deduplicated by name with **project precedence over home**, each tagged
/// with [`AgentSource`].
///
/// Strict 2-path scoping per backend (higher priority first):
///
/// - ClaudeCode:
///   1. `<repo_root>/.claude/agents/` (`Project`)
///   2. `<home>/.claude/agents/` (`Home`)
/// - CopilotCli:
///   1. `<repo_root>/.github/agents/` (`Project`)
///   2. `<home>/.copilot/agents/` (`Home`)
///
/// Files that cannot be parsed are silently skipped (a `warn!` is emitted).
/// Files without a TOML frontmatter fence are accepted with `description: None`.
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
            let name = canonical_name(&stem);
            // Skip if a higher-priority directory already provided this name.
            if by_name.contains_key(&name) {
                continue;
            }
            match load_agent_info(&path, &name, *source) {
                Some(info) => {
                    by_name.insert(name, info);
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

/// Attempt to load an [`AgentInfo`] from `path`.
///
/// - If the file parses successfully, returns the frontmatter name + description.
/// - If the file has no leading `+++` frontmatter fence, falls back to
///   `name = canonical_name` and `description = None` (the whole file is
///   treated as the system prompt body — usable for listing even without
///   structured metadata).
/// - Any other parse error or I/O error is logged and returns `None`.
fn load_agent_info(path: &Path, name: &str, source: AgentSource) -> Option<AgentInfo> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            warn!("list_discoverable: cannot read {}: {e}", path.display());
            return None;
        }
    };
    match parse_agent(name, &content) {
        Ok(agent) => Some(AgentInfo {
            name: agent.name,
            description: Some(agent.description),
            source,
        }),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("missing leading '+++'") {
                // Fenceless file: accept with description=None.
                Some(AgentInfo {
                    name: name.to_string(),
                    description: None,
                    source,
                })
            } else {
                warn!(
                    "list_discoverable: skipping {} (parse error: {e})",
                    path.display()
                );
                None
            }
        }
    }
}

/// Return the ordered list of `(directory, AgentSource)` pairs to search for
/// the given backend. Higher-priority directories appear first.
///
/// Strict 2-path scoping per backend (no universal `.agentic/agents/` override):
///   - ClaudeCode: `<repo>/.claude/agents/` then `$HOME/.claude/agents/`
///   - CopilotCli: `<repo>/.github/agents/` then `$HOME/.copilot/agents/`
fn search_dirs(
    backend: BackendKind,
    repo_root: &Path,
    home: Option<&Path>,
) -> Vec<(PathBuf, AgentSource)> {
    let mut dirs: Vec<(PathBuf, AgentSource)> = Vec::new();

    // 1. Backend-specific project directory (highest priority).
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

    // 2. Backend-specific home directory (lowest priority).
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
