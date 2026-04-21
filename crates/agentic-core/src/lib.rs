#![deny(unsafe_code)]

/// The semver version string of the `agentic-core` crate.
///
/// # Examples
///
/// ```
/// assert!(!agentic_core::VERSION.is_empty());
/// assert!(agentic_core::VERSION.chars().next().unwrap().is_ascii_digit());
/// ```
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
