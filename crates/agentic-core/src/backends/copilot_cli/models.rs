//! Copilot CLI model list resolution.
//!
//! Tries `copilot models list` at runtime; falls back to a bundled list.

use std::collections::HashMap;

use crate::backends::ModelId;
use crate::backends::copilot_cli::runner::CopilotRunner;

/// Bundled fallback model list. Conservative set observed to work with
/// Copilot CLI 1.0.34. Ordered by our preference (newest / most capable first).
pub const BUNDLED_MODELS: &[&str] = &["claude-opus-4.6", "claude-sonnet-4-6", "gpt-5.2", "gpt-5"];

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
pub async fn resolve_models(runner: &CopilotRunner) -> Vec<ModelId> {
    let cancel = tokio_util::sync::CancellationToken::new();
    let args = vec!["models".to_string(), "list".to_string()];

    let outcome = runner
        .run(
            args,
            HashMap::new(),
            std::env::temp_dir(),
            Vec::new(),
            cancel,
        )
        .await;

    match outcome {
        Ok(out) if out.exit_code == Some(0) => {
            let joined = out.stdout_lines.join("\n");
            parse_models_response(&joined).unwrap_or_else(bundled_models)
        }
        _ => bundled_models(),
    }
}

/// Parse a JSON response from `copilot models list`. Accepts either:
/// - `["model-a", "model-b"]` — flat array of strings
/// - `{"models": ["model-a", "model-b"]}` — object with `models` field (string entries)
/// - `{"models": [{"id": "model-a"}, {"id": "model-b"}]}` — object with id'd entries
///
/// Returns `None` if no valid structure parses.
fn parse_models_response(text: &str) -> Option<Vec<ModelId>> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;

    // Try: flat array of strings
    if let Some(arr) = v.as_array() {
        let ids: Vec<ModelId> = arr
            .iter()
            .filter_map(|x| x.as_str().map(|s| ModelId(s.to_string())))
            .collect();
        if !ids.is_empty() {
            return Some(ids);
        }
    }

    // Try: { "models": [...] }
    if let Some(models) = v.get("models").and_then(|m| m.as_array()) {
        let ids: Vec<ModelId> = models
            .iter()
            .filter_map(|m| {
                if let Some(s) = m.as_str() {
                    Some(ModelId(s.to_string()))
                } else {
                    m.get("id")
                        .and_then(|x| x.as_str())
                        .map(|id| ModelId(id.to_string()))
                }
            })
            .collect();
        if !ids.is_empty() {
            return Some(ids);
        }
    }

    None
}
