use std::path::PathBuf;

// Sub-type newtypes (no serde derives yet — RED phase)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketRef {
    pub kind: String,
    pub reference: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    CompletedWithTechDebt,
    Failed,
    Cancelled,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Passed,
    Failed,
    NeedsTriage,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRequired {
    AnswerClarifyingQuestions { question_ids: Vec<String> },
    TriageFindings { finding_ids: Vec<String> },
    QaRetryDecision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    RunStarted {
        ticket: TicketRef,
        profile: ProfileId,
        backend: BackendId,
        model: ModelId,
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
    },
    TextDelta { content: String },
    ThinkingDelta { content: String },
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
    RetryStarted { attempt: u32, reason: String },
    Error {
        code: String,
        message: String,
        recoverable: bool,
        retry_after_ms: Option<u64>,
    },
    UserActionNeeded { action: ActionRequired },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventEnvelope {
    pub event_id: String,
    pub run_id: String,
    pub step_id: Option<String>,
    pub timestamp_ms: i64,
    pub event: Event,
}

impl EventEnvelope {
    pub fn now(run_id: String, step_id: Option<String>, event: Event) -> Self {
        Self {
            event_id: ulid::Ulid::new().to_string(),
            run_id,
            step_id,
            timestamp_ms: crate::time::now_ms(),
            event,
        }
    }
}
