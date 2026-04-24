//! Copilot CLI model list resolution.
//!
//! Tries `copilot models list` at runtime; falls back to a bundled list.

use crate::backends::ModelId;
use crate::backends::copilot_cli::runner::CopilotRunner;

/// Bundled fallback model list. Conservative set observed to work with
/// Copilot CLI 1.0.34. Ordered by our preference (newest / most capable first).
pub const BUNDLED_MODELS: &[&str] = &[
    "claude-opus-4.6",
    "claude-sonnet-4-6",
    "gpt-5.2",
    "gpt-5",
];

/// Returns the bundled fallback model list as `Vec<ModelId>`.
pub fn bundled_models() -> Vec<ModelId> {
    BUNDLED_MODELS
        .iter()
        .map(|s| ModelId((*s).to_string()))
        .collect()
}

/// Try `copilot models list` via `runner`. On success, parse the JSON output
/// into `Vec<ModelId>`. On any failure (non-zero exit, parse error, IO),
/// return the bundled list. Never returns an error.
pub async fn resolve_models(_runner: &CopilotRunner) -> Vec<ModelId> {
    todo!("Step 7.4 GREEN: implement runtime probe")
}
