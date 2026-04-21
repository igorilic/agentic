#![deny(unsafe_code)]

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert_eq!(agentic_core::VERSION, env!("CARGO_PKG_VERSION"));
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
