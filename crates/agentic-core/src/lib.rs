#![deny(unsafe_code)]

pub mod events;
pub use events::{
    ActionRequired, BackendId, Event, EventEnvelope, ModelId, ProfileId, RunStatus, Severity,
    StepStatus, TicketKind, TicketRef, TokenUsage, ToolStream,
};
pub mod error;
pub use error::{CoreError, Result};
pub mod logging;
pub use logging::{init, init_test_subscriber};
pub mod paths;
pub use paths::Paths;
pub mod db;
pub use db::Db;
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
