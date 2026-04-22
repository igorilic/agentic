#![deny(unsafe_code)]

pub mod error;
pub use error::{CoreError, Result};
pub mod logging;
pub use logging::{init, init_test_subscriber};
pub mod paths;
pub use paths::Paths;
pub mod db;
pub use db::Db;
pub use db::workspaces::{Workspace, WorkspaceRepo};
mod time;
pub mod settings;

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
