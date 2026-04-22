mod discovery;
pub use discovery::discover_agent;

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
    /// Markdown body after the YAML frontmatter. Used as the agent's
    /// system prompt.
    pub system_prompt: String,
}

/// Private type for YAML deserialization. Mirrors the public fields of
/// `Agent` but without the `system_prompt` (which comes from the body,
/// not the YAML).
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

/// Parse an agent markdown file. `filename_stem` is the file stem (no
/// extension, no directory) used to validate that the `name` field in
/// frontmatter matches. `content` is the raw file contents including
/// the leading `---\n` fence.
pub fn parse_agent(filename_stem: &str, content: &str) -> Result<Agent> {
    let (yaml_text, body) = extract_frontmatter(content)?;
    let fm: AgentFrontmatter = serde_yml::from_str(yaml_text)
        .map_err(|e| CoreError::Parse(format!("agent frontmatter YAML: {e}")))?;
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

/// Extract the YAML frontmatter and the markdown body. The content must
/// start with `---\n` (or `---\r\n`); the next `---` on a line by itself
/// closes the frontmatter.
fn extract_frontmatter(content: &str) -> Result<(&str, &str)> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            CoreError::Parse("agent file missing leading '---' frontmatter fence".into())
        })?;
    // Find the closing `---` on its own line.
    let close = rest
        .split_inclusive('\n')
        .scan(0usize, |acc, line| {
            let start = *acc;
            *acc += line.len();
            Some((start, line))
        })
        .find(|(_, line)| line.trim_end_matches(['\r', '\n']) == "---")
        .ok_or_else(|| {
            CoreError::Parse("agent file missing closing '---' frontmatter fence".into())
        })?;
    let (yaml_end, close_line) = close;
    let yaml_text = &rest[..yaml_end];
    let body_start = yaml_end + close_line.len();
    // Greedy trim: drops all leading `\n`/`\r` after the closing fence so the
    // body starts at its first real character. Intentional blank lines at the
    // top of an agent's system prompt are not preserved.
    let body = rest[body_start..].trim_start_matches(['\n', '\r']);
    Ok((yaml_text, body))
}
