use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{CoreError, Result};

/// Root of the parsed pipeline.toml. `default` is always present (either
/// parsed from user config or supplied by the built-in fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineConfig {
    pub pipelines: HashMap<String, Pipeline>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pipeline {
    pub steps: Vec<PipelineStep>,
}

/// One step in a pipeline.
///
/// Retry policy is out of scope for v1 — each step runs once.
/// Failure routing is controlled by `stop_on_failure` only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStep {
    pub agent: String,
    #[serde(default = "default_stop_on_failure")]
    pub stop_on_failure: bool,
    #[serde(default)]
    pub allowed_questions: Option<u32>,
}

fn default_stop_on_failure() -> bool {
    true
}

impl PipelineConfig {
    /// The built-in default pipeline per spec §10.4:
    ///   architect → tdd-developer → qa → reviewer
    pub fn builtin_default() -> Self {
        let default_pipeline = Pipeline {
            steps: vec![
                PipelineStep {
                    agent: "architect".to_string(),
                    stop_on_failure: true,
                    allowed_questions: Some(5),
                },
                PipelineStep {
                    agent: "tdd-developer".to_string(),
                    stop_on_failure: true,
                    allowed_questions: None,
                },
                PipelineStep {
                    agent: "qa".to_string(),
                    stop_on_failure: false,
                    allowed_questions: None,
                },
                PipelineStep {
                    agent: "reviewer".to_string(),
                    stop_on_failure: false,
                    allowed_questions: None,
                },
            ],
        };
        let mut pipelines = HashMap::new();
        pipelines.insert("default".to_string(), default_pipeline);
        Self { pipelines }
    }

    /// Build a `Pipeline` from a user-supplied agent name list.
    ///
    /// Each step is configured with `stop_on_failure = true` and
    /// other fields at their defaults. Returns an error if `agents`
    /// is empty.
    pub fn from_agents(agents: &[String]) -> Result<Pipeline> {
        if agents.is_empty() {
            return Err(CoreError::Config(
                "agents list is empty — supply at least one agent".to_string(),
            ));
        }
        Ok(Pipeline {
            steps: agents
                .iter()
                .map(|name| PipelineStep {
                    agent: name.clone(),
                    stop_on_failure: true,
                    allowed_questions: None,
                })
                .collect(),
        })
    }

    /// Parse a pipeline.toml string. Fails if:
    /// - TOML is syntactically invalid.
    /// - `[pipelines.default]` is absent.
    /// - An unknown top-level key exists (`deny_unknown_fields` on the root).
    pub fn parse_str(content: &str) -> Result<Self> {
        let config: Self =
            toml::from_str(content).map_err(|e| CoreError::Parse(format!("pipeline.toml: {e}")))?;
        if !config.pipelines.contains_key("default") {
            return Err(CoreError::Parse(
                "pipeline.toml must define [pipelines.default]".to_string(),
            ));
        }
        Ok(config)
    }

    /// Load from `<repo_root>/.agentic/pipeline.toml`. If the file is absent,
    /// returns `builtin_default()`.
    pub fn load(repo_root: &Path) -> Result<Self> {
        let path = repo_root.join(".agentic").join("pipeline.toml");
        if !path.is_file() {
            return Ok(Self::builtin_default());
        }
        let content = std::fs::read_to_string(&path)?;
        Self::parse_str(&content)
    }

    /// Get the `default` pipeline. Guaranteed present — either parsed or the
    /// built-in fallback.
    pub fn default_pipeline(&self) -> &Pipeline {
        self.pipelines
            .get("default")
            .expect("default pipeline invariant violated — check parse_str / builtin_default")
    }
}
