#![deny(unsafe_code)]

pub mod agents;
pub use agents::{
    Agent, AgentInfo, AgentSource, PipelineRole, discover_agent, discover_agent_with_home,
    list_discoverable, parse_agent,
};
pub mod backends;
pub mod events;
pub mod findings;
#[cfg(any(test, feature = "testing"))]
pub use backends::ScriptedBackend;
pub use backends::claude_code::ClaudeCodeBackend;
pub use backends::copilot_cli::CopilotCliBackend;
pub use backends::file_snapshots::{
    FileSnapshotter, FileState, FinalizeReport, MAX_DIFF_FILE_SIZE, SkipReason,
};
pub use backends::{
    Backend, BackendId, BackendKind, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus,
    ModelId, RunId, StepId, TokenUsage, ToolName, WorkspaceRef,
};
pub use events::{
    ActionRequired, CURRENT_SCHEMA_VERSION, DEFAULT_CAPACITY, DEFAULT_HISTORY_CAP, Event, EventBus,
    EventEnvelope, EventHistoryBuffer, EventPersister, PermissionDecision, PermissionRisk,
    PermissionSource, ProfileId, RunStatus, Severity, StepStatus, TicketKind, TicketRef,
    ToolStream,
};
pub mod error;
pub use error::{CoreError, Result};
pub mod logging;
pub use logging::{init, init_test_subscriber};
pub mod paths;
pub use paths::Paths;
pub mod pipeline;
pub use pipeline::{
    Pipeline, PipelineConfig, PipelineOrchestrator, PipelineSm, PipelineStep, SmInput,
    ToolUseObserver, ToolUseObserverHandle,
};
pub mod db;
pub use db::Db;
pub use db::chat::{ChatMessage, ChatRepo};
pub use db::runs::{Run, RunRepo};
pub use db::steps::{Step, StepRepo};
pub use db::workspaces::{Workspace, WorkspaceRepo};
pub mod settings;
pub use settings::{EnvProvider, Key, MockEnv, RealEnv, Resolver, Setting, SettingsError, Source};
pub mod ticket_sources;
pub use ticket_sources::{
    FreeTextTicketSource, GithubTicketSource, GitlabAuth, GitlabTicketSource, JiraAuth,
    JiraTicketSource, Ticket, TicketComment, TicketSource, TicketSourceError,
};
pub mod auth;
#[cfg(any(test, feature = "testing"))]
pub use auth::MemSecretStore;
pub use auth::{AccessToken, GithubOauthClient, GithubOauthError, validate_state};
pub use auth::{
    AccountStatus, GithubRefreshStrategy, RefreshError, RefreshScheduler, RefreshStrategy,
};
pub use auth::{
    CallbackQuery, KeyringSecretStore, LoopbackError, LoopbackListener, SecretStore,
    SecretStoreError, start_loopback,
};
pub use auth::{DeviceAuthorization, DeviceCodeClient, DeviceCodeError};
pub use auth::{GhDelegate, GhDelegateError};
pub use auth::{GitlabOauthClient, GitlabOauthError};
pub use auth::{PkceChallenge, generate_state};
pub mod permissions;
pub use permissions::{
    OnTimeout, PermissionRule, PermissionsConfig, PermissionsConfigError, PermissionsSettings,
};
mod time;

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
