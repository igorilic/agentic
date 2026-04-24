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
use crate::error::{CoreError, Result};
use crate::events::StepStatus;

use self::parser::parse_stream;
use self::pricing::pricing_for;
use self::runner::ClaudeRunner;

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
        // Write the system prompt to a temp file so we can pass its path
        // as --append-system-prompt. Keep the handle alive until after run().
        let mut system_prompt_file = tempfile::NamedTempFile::new()
            .map_err(|e| CoreError::Backend(format!("failed to create temp file: {e}")))?;
        use std::io::Write as _;
        system_prompt_file
            .write_all(req.agent_prompt.as_bytes())
            .map_err(|e| CoreError::Backend(format!("failed to write system prompt: {e}")))?;
        let system_prompt_path = system_prompt_file.path().to_owned();

        // Build argv.
        let mut args: Vec<String> = vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
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
        args.push(system_prompt_path.to_string_lossy().into_owned());

        // Stdin: user_context as bytes.
        let stdin_bytes = req.user_context.into_bytes();

        // Run the subprocess.
        let run_outcome = self
            .runner
            .run(args, HashMap::new(), req.cwd, stdin_bytes, req.cancel)
            .await?;

        // The temp file can be dropped now (subprocess has already read it).
        drop(system_prompt_file);

        // Feed collected stdout lines through the parser.
        let stdout = run_outcome.stdout_lines.join("\n");
        let reader = BufReader::new(Cursor::new(stdout.into_bytes()));
        let parse_outcome =
            parse_stream(reader, event_sink, req.run_id.0, Some(req.step_id.0)).await?;

        // Determine status.
        let (status, summary) = if run_outcome.was_cancelled {
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

        Ok(ExecuteOutcome {
            status,
            summary,
            token_usage: parse_outcome.token_usage,
            cost_usd,
        })
    }
}
