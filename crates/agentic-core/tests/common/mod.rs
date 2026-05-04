use std::sync::Arc;
use std::time::Duration;

use agentic_core::EventBus;
use agentic_core::permissions::config::{OnTimeout, PermissionsConfig, PermissionsSettings};
use agentic_core::permissions::gate_async::AsyncGate;

/// Permissive gate that allows everything — for orchestrator tests that
/// don't care about gating logic.
pub fn passthrough_gate(bus: &EventBus) -> Arc<AsyncGate> {
    Arc::new(AsyncGate::new(
        PermissionsConfig {
            allowlist: vec![],
            denylist: vec![],
            settings: PermissionsSettings {
                default_on_timeout: OnTimeout::Deny,
            },
        },
        bus.clone(),
        Duration::from_secs(60),
        "test-agent".to_string(),
    ))
}
