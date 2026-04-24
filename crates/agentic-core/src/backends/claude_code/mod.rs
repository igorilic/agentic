//! Claude Code backend adapter.
//!
//! Wires together the subprocess runner (step 6.2) and the stream parser
//! (step 6.1) to implement the `Backend` trait end-to-end.

pub mod parser;
pub mod pricing;
pub mod runner;

use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::io::BufReader;
use tokio_util::sync::CancellationToken;

use crate::backends::{
    Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId,
};
use crate::error::Result;
use crate::events::{Event, EventEnvelope, StepStatus};

use self::parser::parse_stream;
use self::pricing::pricing_for;
use self::runner::ClaudeRunner;

/// Build argv for `claude` subprocess invocation.
/// Does NOT include the binary itself (the runner prepends that).
pub(crate) fn build_claude_argv(req: &ExecuteRequest) -> Vec<String> {
    let mut args: Vec<String> = vec![
        "-p".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(), // REQUIRED when combining -p + stream-json
    ];
    if let Some(ref model) = req.model {
        args.push("--model".to_string());
        args.push(model.0.clone());
    }
    if !req.tools.is_empty() {
        let joined = req
            .tools
            .iter()
            .map(|t| t.0.as_str())
            .collect::<Vec<_>>()
            .join(",");
        args.push("--allowed-tools".to_string());
        args.push(joined);
    }
    args.push("--append-system-prompt".to_string());
    args.push(req.agent_prompt.clone());
    args
}

/// Backend adapter that drives the `claude` CLI.
#[derive(Debug, Clone)]
pub struct ClaudeCodeBackend {
    runner: ClaudeRunner,
}

impl ClaudeCodeBackend {
    /// Construct using the `CLAUDE_CODE_BIN` env var (falling back to `"claude"`).
    pub fn from_env() -> Self {
        Self {
            runner: ClaudeRunner::from_env(),
        }
    }

    /// Inject a specific binary — useful in tests.
    pub fn with_binary(binary: PathBuf) -> Self {
        Self {
            runner: ClaudeRunner::with_binary(binary),
        }
    }

    /// Inject a binary and a custom SIGTERM grace period — useful in signal tests.
    pub fn with_binary_and_grace(binary: PathBuf, grace: std::time::Duration) -> Self {
        Self {
            runner: ClaudeRunner::with_binary_and_grace(binary, grace),
        }
    }
}

#[async_trait]
impl Backend for ClaudeCodeBackend {
    fn id(&self) -> BackendId {
        BackendId("claude-code".to_string())
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn supported_models(&self) -> Vec<ModelId> {
        vec![
            ModelId("claude-opus-4-7".to_string()),
            ModelId("claude-sonnet-4-6".to_string()),
            ModelId("claude-haiku-4-5-20251001".to_string()),
        ]
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
                reason: format!("claude --version exited with code {:?}", run.exit_code),
            }),
            Err(e) => Ok(HealthStatus::Unhealthy {
                reason: e.to_string(),
            }),
        }
    }

    async fn execute(&self, req: ExecuteRequest, event_sink: EventSink) -> Result<ExecuteOutcome> {
        // Build argv — passes system prompt inline (not via a temp file).
        let args = build_claude_argv(&req);

        // Stdin: user_context as bytes.
        let stdin_bytes = req.user_context.into_bytes();

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
        // after the timeout duration. If the run finishes first the token is
        // already cancelled — a second cancel() on a CancellationToken is a no-op.
        let start = tokio::time::Instant::now();
        if let Some(deadline) = req.timeout {
            let cancel_clone = req.cancel.clone();
            tokio::spawn(async move {
                tokio::time::sleep(deadline).await;
                cancel_clone.cancel();
            });
        }

        // Run the subprocess.
        let run_outcome = self
            .runner
            .run(args, HashMap::new(), req.cwd, stdin_bytes, req.cancel)
            .await?;

        // Capture run_id and step_id as strings before parse_stream takes ownership.
        let run_id_str = req.run_id.0.clone();
        let step_id_str = req.step_id.0.clone();

        // Feed collected stdout lines through the parser.
        // parse_stream takes ownership of event_sink; clone the sender first so
        // we can emit StepComplete after parsing.
        let sink_for_complete = event_sink.clone();
        let stdout = run_outcome.stdout_lines.join("\n");
        let reader = BufReader::new(Cursor::new(stdout.into_bytes()));
        let parse_outcome =
            parse_stream(reader, event_sink, req.run_id.0, Some(req.step_id.0)).await?;

        // Determine status.
        // Distinguish timeout from external cancel: if a deadline was set and we
        // reached or exceeded it, report "timeout" (spec §11.4 error code).
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

        // Compute cost if the model is known.
        let cost_usd = req
            .model
            .as_ref()
            .and_then(pricing_for)
            .map(|p| p.compute_cost(&parse_outcome.token_usage));

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
    use crate::backends::{ModelId, ToolName};
    use tokio_util::sync::CancellationToken;

    fn make_test_request() -> ExecuteRequest {
        ExecuteRequest {
            workspace: crate::backends::WorkspaceRef {
                id: "ws-test".to_string(),
                root_path: std::env::temp_dir(),
            },
            run_id: crate::RunId("run-test".to_string()),
            step_id: crate::StepId("step-test".to_string()),
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
    fn argv_contains_verbose_for_stream_json() {
        let req = make_test_request();
        let argv = build_claude_argv(&req);
        assert!(argv.contains(&"-p".to_string()));
        assert!(argv.iter().any(|a| a == "--output-format"));
        assert!(argv.iter().any(|a| a == "stream-json"));
        assert!(
            argv.iter().any(|a| a == "--verbose"),
            "argv missing --verbose: {argv:?}"
        );
    }

    #[test]
    fn argv_passes_prompt_inline_not_as_path() {
        let mut req = make_test_request();
        req.agent_prompt = "You are a test assistant.\nFollow instructions.".to_string();
        let argv = build_claude_argv(&req);
        let prompt_idx = argv
            .iter()
            .position(|a| a == "--append-system-prompt")
            .expect("--append-system-prompt not in argv");
        let prompt_val = &argv[prompt_idx + 1];
        assert_eq!(
            prompt_val, "You are a test assistant.\nFollow instructions.",
            "expected inline prompt, got: {prompt_val}"
        );
        assert!(
            !prompt_val.starts_with("/tmp/"),
            "looks like a temp file path: {prompt_val}"
        );
        assert!(
            !prompt_val.starts_with("/var/"),
            "looks like a temp file path (macOS tmpdir): {prompt_val}"
        );
    }

    #[test]
    fn argv_includes_model_when_specified() {
        let mut req = make_test_request();
        req.model = Some(ModelId("claude-sonnet-4-6".into()));
        let argv = build_claude_argv(&req);
        let i = argv
            .iter()
            .position(|a| a == "--model")
            .expect("--model missing");
        assert_eq!(argv[i + 1], "claude-sonnet-4-6");
    }

    #[test]
    fn argv_omits_model_when_none() {
        let mut req = make_test_request();
        req.model = None;
        let argv = build_claude_argv(&req);
        assert!(
            !argv.iter().any(|a| a == "--model"),
            "argv should not contain --model when req.model is None"
        );
    }

    #[test]
    fn argv_joins_tools_with_comma() {
        let mut req = make_test_request();
        req.tools = vec![
            ToolName("Read".into()),
            ToolName("Edit".into()),
            ToolName("Bash".into()),
        ];
        let argv = build_claude_argv(&req);
        let i = argv
            .iter()
            .position(|a| a == "--allowed-tools")
            .expect("--allowed-tools missing");
        assert_eq!(argv[i + 1], "Read,Edit,Bash");
    }
}
