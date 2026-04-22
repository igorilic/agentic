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

/// Parse an agent markdown file. `filename_stem` is the file stem (no
/// extension, no directory) used to validate that the `name` field in
/// frontmatter matches. `content` is the raw file contents including
/// the leading `---\n` fence.
pub fn parse_agent(_filename_stem: &str, _content: &str) -> Result<Agent> {
    unimplemented!()
}
