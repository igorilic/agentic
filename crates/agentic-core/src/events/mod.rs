use serde::{Deserialize, Serialize};
use std::path::PathBuf;

mod bus;
pub use bus::{DEFAULT_CAPACITY, EventBus};
pub mod history;
pub use history::{DEFAULT_HISTORY_CAP, EventHistoryBuffer};
mod persist;
pub use persist::EventPersister;

// Re-export types that moved to `backends` — keeps `events::BackendId` etc. working.
pub use crate::backends::{BackendId, ModelId, TokenUsage};

/// Current wire-format schema version for `EventEnvelope`. Bump when the
/// envelope or any Event variant shape changes in a way that's not
/// backward-compatible (renamed fields, changed types, removed variants).
/// Bumping additively (new optional fields, new variants) does NOT require
/// a version bump — serde's `#[serde(default)]` handles those.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

fn default_schema_version() -> u32 {
    CURRENT_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TicketKind {
    GithubIssue,
    GitlabIssue,
    Jira,
    FreeText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketRef {
    pub kind: TicketKind,
    pub reference: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    CompletedWithTechDebt,
    Failed,
    Cancelled,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    NeedsTriage,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ActionRequired {
    AnswerClarifyingQuestions { question_ids: Vec<String> },
    TriageFindings { finding_ids: Vec<String> },
    QaRetryDecision,
}

/// Risk level for a permission gate request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionRisk {
    Low,
    Medium,
    High,
}

/// Decision recorded by the permission gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionDecision {
    AllowOnce,
    AllowSession,
    Deny,
    TimedOut,
}

/// Who or what resolved the permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionSource {
    User,
    AllowlistConfig,
    DenylistConfig,
    SessionAllowlist,
    Timeout,
    /// The run was cancelled (e.g. via `CancellationToken`) before a decision
    /// arrived. Added in P.2.2 — additive, no schema version bump required.
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "PascalCase")]
pub enum Event {
    RunStarted {
        ticket: TicketRef,
        profile: ProfileId,
        backend: BackendId,
        model: ModelId,
        #[serde(default)]
        agents: Vec<String>,
    },
    RunComplete {
        status: RunStatus,
        duration_ms: u64,
        summary: String,
    },
    StepStarted {
        agent: String,
        model: ModelId,
    },
    StepComplete {
        status: StepStatus,
        summary: String,
        token_usage: TokenUsage,
        cost_usd: Option<f64>,
        duration_ms: u64,
    },
    TextDelta {
        content: String,
    },
    ThinkingDelta {
        content: String,
    },
    ToolUseStart {
        tool_call_id: String,
        tool_name: String,
        input: serde_json::Value,
    },
    ToolUseDelta {
        tool_call_id: String,
        stream: ToolStream,
        content: String,
    },
    ToolUseEnd {
        tool_call_id: String,
        exit_code: Option<i32>,
        duration_ms: u64,
    },
    FileChange {
        path: PathBuf,
        before_hash: String,
        after_hash: String,
    },
    Finding {
        finding_id: String,
        severity: Severity,
        file: Option<PathBuf>,
        line: Option<u32>,
        message: String,
        suggestion: Option<String>,
    },
    ClarifyingQuestion {
        question_id: String,
        question: String,
        suggested_answers: Vec<String>,
    },
    RetryStarted {
        attempt: u32,
        reason: String,
    },
    Error {
        code: String,
        message: String,
        recoverable: bool,
        retry_after_ms: Option<u64>,
    },
    UserActionNeeded {
        action: ActionRequired,
    },
    // schema: additive — no CURRENT_SCHEMA_VERSION bump required
    PermissionRequest {
        request_id: String,
        agent: String,
        tool: String,
        arg: String,
        scope: String,
        risk: PermissionRisk,
        reason: String,
    },
    // schema: additive — no CURRENT_SCHEMA_VERSION bump required
    PermissionResolved {
        request_id: String,
        decision: PermissionDecision,
        source: PermissionSource,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Envelope schema version. See [`CURRENT_SCHEMA_VERSION`]. Defaults to
    /// 1 on deserialization of old BLOBs/JSON that predate this field.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub event_id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub timestamp_ms: i64,
    pub event: Event,
}

impl EventEnvelope {
    /// Create a new envelope with a fresh ULID `event_id` and `timestamp_ms`
    /// set to `crate::time::now_ms()`.
    pub fn now(run_id: String, step_id: Option<String>, event: Event) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            event_id: ulid::Ulid::new().to_string(),
            run_id,
            step_id,
            timestamp_ms: crate::time::now_ms(),
            event,
        }
    }
}
