#![deny(unsafe_code)]

pub mod agents;
pub use agents::{Agent, PipelineRole, discover_agent, parse_agent};
pub mod backends;
pub mod events;
#[cfg(any(test, feature = "testing"))]
pub use backends::ScriptedBackend;
pub use backends::claude_code::ClaudeCodeBackend;
pub use backends::file_snapshots::{
    FileSnapshotter, FileState, FinalizeReport, MAX_DIFF_FILE_SIZE, SkipReason,
};
pub use backends::{
    Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId, RunId,
    StepId, TokenUsage, ToolName, WorkspaceRef,
};
pub use events::{
    ActionRequired, CURRENT_SCHEMA_VERSION, DEFAULT_CAPACITY, Event, EventBus, EventEnvelope,
    EventPersister, ProfileId, RunStatus, Severity, StepStatus, TicketKind, TicketRef, ToolStream,
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
pub use db::runs::{Run, RunRepo};
pub use db::steps::{Step, StepRepo};
pub use db::workspaces::{Workspace, WorkspaceRepo};
pub mod settings;
pub use settings::{EnvProvider, Key, MockEnv, RealEnv, Resolver, Setting, Source};
mod time;

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
