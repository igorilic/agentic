#![deny(unsafe_code)]

pub mod error;
pub mod logging;
pub use logging::{init, init_test_subscriber};

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
