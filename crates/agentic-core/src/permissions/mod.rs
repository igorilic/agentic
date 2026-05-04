//! Permission gate subsystem.
//!
//! ## Public API surface
//!
//! **Carrier / config types** (`PermissionsConfig`, `PermissionRule`, `OnTimeout`,
//! `PermissionsSettings`, `PermissionsConfigError`) are re-exported at the
//! crate root because downstream callers (e.g. the Tauri layer loading
//! `permissions.toml`) need direct access without importing submodule paths.
//!
//! **Logic types** (`AsyncGate`, `ConfigGate`, `GateOutcome`, `PermissionGate`,
//! `SessionAllowlist`) are kept at module level (`permissions::AsyncGate`)
//! because they are constructed internally by the orchestrator and have no
//! external callers in v1. Promote to crate-root re-export only when a
//! downstream consumer needs them directly.

pub mod config;
pub mod gate;
pub mod gate_async;
pub mod matcher;
pub mod risk;
pub mod session;
pub use config::{
    OnTimeout, PermissionRule, PermissionsConfig, PermissionsConfigError, PermissionsSettings,
    builtin_permissions_toml,
};
pub use gate::{ConfigGate, GateOutcome, PermissionGate};
pub use gate_async::AsyncGate;
pub use matcher::{Pattern, PatternParseError};
pub use risk::classify;
pub use session::SessionAllowlist;
