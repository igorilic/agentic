//! Copilot CLI backend adapter.
//!
//! Wires together the subprocess runner and the stream parser to implement
//! the `Backend` trait for GitHub Copilot's CLI.
//!
//! # Cost
//! Pricing is not yet available for Copilot (no equivalent to `pricing.rs`).
//! `cost_usd` is always `None`. A pricing table can be added in a follow-up step.

pub mod models;
pub mod parser;
pub mod runner;

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::io::BufReader;
use tokio_util::sync::CancellationToken;

use crate::backends::{
    Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId,
};
use crate::error::{CoreError, Result};
use crate::events::{Event, EventEnvelope, StepStatus};

use self::parser::parse_stream;
use self::runner::{CopilotRunner, StreamingRun};

/// Build argv for `copilot` subprocess invocation.
/// Does NOT include the binary itself (the runner prepends that).
///
/// Copilot takes the prompt inline via `-p <text>`, not via stdin.
pub(crate) fn build_copilot_argv(req: &ExecuteRequest, combined_prompt: &str) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-p".to_string(),
        combined_prompt.to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--allow-all-tools".to_string(),
    ];
    if let Some(ref model) = req.model {
        args.push("--model".to_string());
        args.push(model.0.clone());
    }
    args
}

/// Backend adapter that drives the `copilot` CLI.
#[derive(Debug, Clone)]
pub struct CopilotCliBackend {
    runner: CopilotRunner,
}

impl CopilotCliBackend {
    /// Construct using the `COPILOT_CLI_BIN` env var (falling back to `"copilot"`).
    pub fn from_env() -> Self {
        Self {
            runner: CopilotRunner::from_env(),
        }
    }

    /// Inject a specific binary — useful in tests.
    pub fn with_binary(binary: PathBuf) -> Self {
        Self {
            runner: CopilotRunner::with_binary(binary),
        }
    }

    /// Inject a binary and a custom SIGTERM grace period — useful in signal tests.
    pub fn with_binary_and_grace(binary: PathBuf, grace: std::time::Duration) -> Self {
        Self {
            runner: CopilotRunner::with_binary_and_grace(binary, grace),
        }
    }
}

#[async_trait]
impl Backend for CopilotCliBackend {
    fn id(&self) -> BackendId {
        BackendId("copilot-cli".to_string())
    }

    fn display_name(&self) -> &str {
        "Copilot CLI"
    }

    fn supported_models(&self) -> Vec<ModelId> {
        models::bundled_models()
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        // Probe the binary with `--version`; non-zero exit → unhealthy.
        let cancel = CancellationToken::new();
        let cwd = std::env::temp_dir();
        let outcome = self
            .runner
            .run(
                vec!["--version".to_string()],
                HashMap::new(),
                cwd,
                vec![],
                cancel,
            )
            .await;

        match outcome {
            Ok(run) if run.exit_code == Some(0) => Ok(HealthStatus::Healthy),
            Ok(run) => Ok(HealthStatus::Unhealthy {
                reason: format!("copilot --version exited with code {:?}", run.exit_code),
            }),
            Err(e) => Ok(HealthStatus::Unhealthy {
                reason: e.to_string(),
            }),
        }
    }

    async fn execute(&self, req: ExecuteRequest, event_sink: EventSink) -> Result<ExecuteOutcome> {
        // Combine agent_prompt and user_context into one prompt for Copilot.
        // Copilot has no equivalent to `--append-system-prompt`, so we prepend
        // the agent prompt separated by a section divider.
        let combined_prompt = if req.agent_prompt.is_empty() {
            req.user_context.clone()
        } else {
            format!("{}\n\n---\n\n{}", req.agent_prompt, req.user_context)
        };

        // Build argv with the combined prompt inline (Copilot doesn't use stdin for prompts).
        let args = build_copilot_argv(&req, &combined_prompt);

        // Copilot doesn't read prompts from stdin — pass empty bytes.
        let stdin_bytes: Vec<u8> = Vec::new();

        // Emit synthetic StepStarted before spawning the subprocess so that
        // the orchestrator can transition the step row to Running.
        let started = EventEnvelope::now(
            req.run_id.0.clone(),
            Some(req.step_id.0.clone()),
            Event::StepStarted {
                agent: req.agent_name.clone(),
                model: req
                    .model
                    .clone()
                    .unwrap_or_else(|| ModelId("unknown".to_string())),
            },
        );
        let _ = event_sink.send(started);

        // Honour the optional deadline: spawn a task that fires the cancel token
        // after the timeout duration.
        let start = tokio::time::Instant::now();
        if let Some(deadline) = req.timeout {
            let cancel_clone = req.cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(deadline).await;
                cancel_clone.cancel();
            });
        }

        // Spawn the subprocess and get a live stdout reader (streaming path).
        // Stderr is drained inside run_streaming so the subprocess never blocks.
        let StreamingRun {
            stdout,
            wait_handle,
        } = self
            .runner
            .run_streaming(args, HashMap::new(), req.cwd, stdin_bytes, req.cancel)
            .map_err(|e| CoreError::Backend(e.to_string()))?;

        // Capture run_id and step_id as strings before parse_stream takes ownership.
        let run_id_str = req.run_id.0.clone();
        let step_id_str = req.step_id.0.clone();

        // Clone sink before parse_stream consumes it.
        let sink_for_complete = event_sink.clone();

        // Run parser and wait concurrently.
        // parse_stream drains stdout live; wait_handle resolves once the subprocess exits.
        // We use tokio::join! (not try_join!) so both futures always run to completion
        // and we can handle their errors independently.
        let reader = BufReader::new(stdout);
        let (parse_result, wait_result) = tokio::join!(
            parse_stream(reader, event_sink, req.run_id.0, Some(req.step_id.0)),
            async {
                wait_handle
                    .await
                    .map_err(|e| CoreError::Backend(format!("wait task panicked: {e}")))?
            }
        );

        let parse_outcome = parse_result?;
        let run_outcome = wait_result?;

        // Determine status.
        // Distinguish timeout from external cancel: if a deadline was set and we
        // reached or exceeded it, report "timeout".
        let timed_out = req.timeout.is_some_and(|t| start.elapsed() >= t);
        let (status, summary) = if run_outcome.was_cancelled && timed_out {
            (StepStatus::Failed, "timeout".to_string())
        } else if run_outcome.was_cancelled {
            (StepStatus::Failed, "cancelled".to_string())
        } else if parse_outcome.saw_unrecoverable_error {
            let msg = parse_outcome
                .error_message
                .unwrap_or_else(|| "upstream error".to_string());
            (StepStatus::Failed, msg)
        } else if run_outcome.exit_code != Some(0) {
            let code = run_outcome
                .exit_code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            (StepStatus::Failed, format!("subprocess exited {code}"))
        } else {
            (StepStatus::Passed, "ok".to_string())
        };

        // Copilot has no pricing table yet — cost is always None.
        let cost_usd: Option<f64> = None;

        // Emit synthetic StepComplete so the orchestrator updates the step row.
        let completed = EventEnvelope::now(
            run_id_str,
            Some(step_id_str),
            Event::StepComplete {
                status,
                summary: summary.clone(),
                token_usage: parse_outcome.token_usage.clone(),
                cost_usd,
                duration_ms: start.elapsed().as_millis() as u64,
            },
        );
        let _ = sink_for_complete.send(completed);

        Ok(ExecuteOutcome {
            status,
            summary,
            token_usage: parse_outcome.token_usage,
            cost_usd,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{ModelId, ToolName, WorkspaceRef};
    use crate::{RunId, StepId};
    use tokio_util::sync::CancellationToken;

    fn make_test_request() -> ExecuteRequest {
        ExecuteRequest {
            workspace: WorkspaceRef {
                id: "ws-test".to_string(),
                root_path: std::env::temp_dir(),
            },
            run_id: RunId("run-test".to_string()),
            step_id: StepId("step-test".to_string()),
            agent_name: "test-agent".to_string(),
            agent_prompt: "You are a test assistant.".to_string(),
            user_context: "Do the thing.".to_string(),
            model: None,
            tools: vec![],
            cwd: std::env::temp_dir(),
            timeout: None,
            cancel: CancellationToken::new(),
        }
    }

    #[test]
    fn argv_contains_p_output_format_allow_all_tools() {
        let req = make_test_request();
        let combined = "You are a test assistant.\n\n---\n\nDo the thing.";
        let argv = build_copilot_argv(&req, combined);
        assert!(argv.contains(&"-p".to_string()));
        assert!(argv.contains(&combined.to_string()));
        assert!(argv.iter().any(|a| a == "--output-format"));
        assert!(argv.iter().any(|a| a == "json"));
        assert!(argv.iter().any(|a| a == "--allow-all-tools"));
    }

    #[test]
    fn argv_includes_model_when_specified() {
        let mut req = make_test_request();
        req.model = Some(ModelId("claude-opus-4.6".into()));
        let combined = "prompt";
        let argv = build_copilot_argv(&req, combined);
        let i = argv
            .iter()
            .position(|a| a == "--model")
            .expect("--model missing");
        assert_eq!(argv[i + 1], "claude-opus-4.6");
    }

    #[test]
    fn argv_omits_model_when_none() {
        let req = make_test_request();
        let argv = build_copilot_argv(&req, "prompt");
        assert!(
            !argv.iter().any(|a| a == "--model"),
            "argv should not contain --model when req.model is None"
        );
    }

    #[test]
    fn combined_prompt_prepends_agent_prompt_with_divider() {
        let combined = format!("{}\n\n---\n\n{}", "Agent prompt", "User context");
        assert!(combined.starts_with("Agent prompt"));
        assert!(combined.contains("\n\n---\n\n"));
        assert!(combined.ends_with("User context"));
    }

    #[test]
    fn combined_prompt_uses_user_context_alone_when_agent_prompt_empty() {
        let agent_prompt = "";
        let user_context = "Do the thing.";
        let combined = if agent_prompt.is_empty() {
            user_context.to_string()
        } else {
            format!("{}\n\n---\n\n{}", agent_prompt, user_context)
        };
        assert_eq!(combined, "Do the thing.");
    }

    #[test]
    fn backend_id_and_display_name() {
        let backend = CopilotCliBackend::from_env();
        assert_eq!(backend.id(), BackendId("copilot-cli".to_string()));
        assert_eq!(backend.display_name(), "Copilot CLI");
    }

    #[test]
    fn supported_models_returns_bundled_list() {
        let backend = CopilotCliBackend::from_env();
        let models = backend.supported_models();
        assert!(
            !models.is_empty(),
            "supported_models should return bundled list"
        );
        assert!(models.iter().any(|m| m.0 == "claude-opus-4.6"));
    }

    #[test]
    fn argv_does_not_contain_tools_field() {
        let mut req = make_test_request();
        req.tools = vec![ToolName("Read".into())];
        let argv = build_copilot_argv(&req, "prompt");
        // Copilot uses --allow-all-tools (already present), not --allowed-tools
        assert!(
            !argv.iter().any(|a| a == "--allowed-tools"),
            "Copilot argv should not contain --allowed-tools"
        );
    }
}
