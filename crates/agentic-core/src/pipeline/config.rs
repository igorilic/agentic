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
        unimplemented!()
    }

    /// Parse a pipeline.toml string. Fails if:
    /// - TOML is syntactically invalid.
    /// - `[pipelines.default]` is absent.
    /// - An unknown top-level key exists (`deny_unknown_fields` on the root).
    pub fn parse_str(_content: &str) -> Result<Self> {
        unimplemented!()
    }

    /// Load from `<repo_root>/.agentic/pipeline.toml`. If the file is absent,
    /// returns `builtin_default()`.
    pub fn load(_repo_root: &Path) -> Result<Self> {
        unimplemented!()
    }

    /// Get the `default` pipeline. Guaranteed present — either parsed or the
    /// built-in fallback.
    pub fn default_pipeline(&self) -> &Pipeline {
        unimplemented!()
    }
}
