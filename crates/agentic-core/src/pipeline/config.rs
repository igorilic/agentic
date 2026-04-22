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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineStep {
    pub agent: String,
    #[serde(default = "default_stop_on_failure")]
    pub stop_on_failure: bool,
    #[serde(default)]
    pub allowed_questions: Option<u32>,
    #[serde(default)]
    pub qa_fix_loop_cap: Option<u32>,
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
                    qa_fix_loop_cap: None,
                },
                PipelineStep {
                    agent: "tdd-developer".to_string(),
                    stop_on_failure: true,
                    allowed_questions: None,
                    qa_fix_loop_cap: Some(3),
                },
                PipelineStep {
                    agent: "qa".to_string(),
                    stop_on_failure: false,
                    allowed_questions: None,
                    qa_fix_loop_cap: None,
                },
                PipelineStep {
                    agent: "reviewer".to_string(),
                    stop_on_failure: false,
                    allowed_questions: None,
                    qa_fix_loop_cap: None,
                },
            ],
        };
        let mut pipelines = HashMap::new();
        pipelines.insert("default".to_string(), default_pipeline);
        Self { pipelines }
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
