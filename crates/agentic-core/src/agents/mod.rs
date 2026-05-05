mod discovery;
mod list;
pub use discovery::{discover_agent, discover_agent_with_home};
pub use list::list_discoverable;

use serde::{Deserialize, Serialize};

use crate::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineRole {
    /// Participates in the default pipeline.
    #[default]
    Step,
    /// `@`-mention only; not in the default pipeline.
    Mention,
    /// Step + `@`-mentionable outside the pipeline.
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tools: Option<Vec<String>>,
    pub allowed_questions: Option<u32>,
    pub pipeline_role: PipelineRole,
    pub timeout_seconds: Option<u64>,
    /// Markdown body after the TOML frontmatter. Used as the agent's
    /// system prompt.
    pub system_prompt: String,
}

/// Private type for TOML deserialization. Mirrors the public fields of
/// `Agent` but without the `system_prompt` (which comes from the body,
/// not the TOML).
#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    name: String,
    description: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    tools: Option<Vec<String>>,
    #[serde(default)]
    allowed_questions: Option<u32>,
    #[serde(default)]
    pipeline_role: PipelineRole,
    #[serde(default)]
    timeout_seconds: Option<u64>,
}

/// Whether an agent was found in a project-local directory or in the
/// user's home directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentSource {
    /// Agent file lives in a project-local directory (`.agentic/agents/`,
    /// `.claude/agents/`, or `.github/agents/`).
    Project,
    /// Agent file lives in the user's home directory (`~/.claude/agents/`
    /// or `~/.copilot/agents/`).
    Home,
}

/// Lightweight descriptor returned by [`list_discoverable`]. Contains only
/// the fields needed for display / selection UIs; the full [`Agent`] (with
/// system prompt) is loaded on demand via [`discover_agent`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInfo {
    /// File basename without the `.md` extension. Matches the `name` field
    /// in the TOML frontmatter.
    pub name: String,
    /// Optional description extracted from the TOML frontmatter `description`
    /// field. `None` if the field was absent or the file was only partially
    /// parsed.
    pub description: Option<String>,
    /// Whether this agent was found in a project directory or the home
    /// directory.
    pub source: AgentSource,
}

/// Parse an agent markdown file. `filename_stem` is the file stem (no
/// extension, no directory) used to validate that the `name` field in
/// frontmatter matches. `content` is the raw file contents including
/// the leading `+++\n` fence.
pub fn parse_agent(filename_stem: &str, content: &str) -> Result<Agent> {
    let (toml_text, body) = extract_frontmatter(content)?;
    let fm: AgentFrontmatter = toml::from_str(toml_text)
        .map_err(|e| CoreError::Parse(format!("agent frontmatter TOML: {e}")))?;
    if fm.name != filename_stem {
        return Err(CoreError::Parse(format!(
            "name mismatch: frontmatter has '{}', filename stem is '{}'",
            fm.name, filename_stem
        )));
    }
    Ok(Agent {
        name: fm.name,
        description: fm.description,
        model: fm.model,
        tools: fm.tools,
        allowed_questions: fm.allowed_questions,
        pipeline_role: fm.pipeline_role,
        timeout_seconds: fm.timeout_seconds,
        system_prompt: body.to_string(),
    })
}

/// Extract the TOML frontmatter and the markdown body. The content must
/// start with `+++\n` (or `+++\r\n`); the next `+++` on a line by itself
/// closes the frontmatter.
fn extract_frontmatter(content: &str) -> Result<(&str, &str)> {
    let rest = content
        .strip_prefix("+++\n")
        .or_else(|| content.strip_prefix("+++\r\n"))
        .ok_or_else(|| {
            CoreError::Parse("agent file missing leading '+++' frontmatter fence".into())
        })?;
    let close = rest
        .split_inclusive('\n')
        .scan(0usize, |acc, line| {
            let start = *acc;
            *acc += line.len();
            Some((start, line))
        })
        .find(|(_, line)| line.trim_end_matches(['\r', '\n']) == "+++")
        .ok_or_else(|| {
            CoreError::Parse("agent file missing closing '+++' frontmatter fence".into())
        })?;
    let (toml_end, close_line) = close;
    let toml_text = &rest[..toml_end];
    let body_start = toml_end + close_line.len();
    // Greedy trim: drops all leading `\n`/`\r` after the closing fence so the
    // body starts at its first real character. Intentional blank lines at the
    // top of an agent's system prompt are not preserved.
    let body = rest[body_start..].trim_start_matches(['\n', '\r']);
    Ok((toml_text, body))
}
