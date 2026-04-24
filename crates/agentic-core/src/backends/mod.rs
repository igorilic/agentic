#[cfg(any(test, feature = "testing"))]
pub mod scripted;
#[cfg(any(test, feature = "testing"))]
pub use scripted::ScriptedBackend;

pub mod claude_code;
pub mod copilot_cli;
pub mod file_snapshots;

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::Result;
use crate::events::{EventEnvelope, StepStatus};

/// Stable identifier for a backend adapter (e.g., "claude-code").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BackendId(pub String);

/// Identifier for an LLM model (e.g., "claude-opus-4-7").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(pub String);

/// Token usage counters returned by a backend after a step completes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

/// Opaque ULID wrapper identifying a pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunId(pub String);

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Opaque ULID wrapper identifying a pipeline step.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StepId(pub String);

impl std::fmt::Display for StepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A tool name (e.g., "Read", "Write", "Bash"). Backend adapters use these
/// as allow-list entries when invoking sub-processes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ToolName(pub String);

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Health-check result for a backend — used by the UI to show a status
/// indicator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy { reason: String },
}

/// Minimal slice of a Workspace that a backend adapter needs: id for
/// correlation and root_path for subprocess cwd.
#[derive(Debug, Clone)]
pub struct WorkspaceRef {
    pub id: String,
    pub root_path: PathBuf,
}

impl From<&crate::db::workspaces::Workspace> for WorkspaceRef {
    fn from(ws: &crate::db::workspaces::Workspace) -> Self {
        Self {
            id: ws.id.clone(),
            root_path: std::path::PathBuf::from(&ws.root_path),
        }
    }
}

/// Channel sink for streaming events back to the orchestrator during
/// `Backend::execute`.
pub type EventSink = tokio::sync::broadcast::Sender<EventEnvelope>;

/// Request passed to `Backend::execute` for one agent invocation.
#[derive(Debug)]
pub struct ExecuteRequest {
    pub workspace: WorkspaceRef,
    pub run_id: RunId,
    pub step_id: StepId,
    pub agent_name: String,
    pub agent_prompt: String,
    pub user_context: String,
    pub model: Option<ModelId>,
    pub tools: Vec<ToolName>,
    pub cwd: PathBuf,
    pub timeout: Option<Duration>,
    pub cancel: CancellationToken,
}

/// Final outcome of a `Backend::execute` call.
#[derive(Debug, Clone)]
pub struct ExecuteOutcome {
    pub status: StepStatus,
    pub summary: String,
    pub token_usage: TokenUsage,
    pub cost_usd: Option<f64>,
}

/// Trait implemented by every backend adapter (claude-code, copilot-cli, …).
#[async_trait]
pub trait Backend: Send + Sync {
    /// Stable identifier (e.g., "claude-code", "copilot-cli").
    fn id(&self) -> BackendId;

    /// Human-readable name for the UI (e.g., "Claude Code").
    fn display_name(&self) -> &str;

    /// Models this backend can drive.
    fn supported_models(&self) -> Vec<ModelId>;

    /// Synchronously check readiness — CLI on PATH, auth valid, etc.
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Execute one agent invocation, streaming intermediate events into
    /// `event_sink`. Returns the final outcome on completion.
    async fn execute(&self, req: ExecuteRequest, event_sink: EventSink) -> Result<ExecuteOutcome>;
}
