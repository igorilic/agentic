use std::sync::Arc;

use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::Result;
use crate::db::runs::RunRepo;
use crate::db::steps::StepRepo;
use crate::events::{Event, EventBus, EventEnvelope, PermissionDecision, PermissionSource, RunStatus, StepStatus};
use crate::permissions::config::OnTimeout;
use crate::permissions::gate::GateOutcome;
use crate::permissions::gate_async::AsyncGate;

/// Background orchestrator consuming events from the bus and applying
/// them to `runs`/`run_steps` rows. One per bus per DB.
///
/// Consumed event variants:
/// - `RunStarted`: transitions the run row to `Running`. Idempotent —
///   if the run is already `Running`, no-op (handles duplicate
///   emissions / event replay).
/// - `StepStarted`: transitions the step row to `Running`.
/// - `StepComplete`: sets step status + `completed_at` (= envelope
///   `timestamp_ms`) + `duration_ms` (from event payload).
/// - `RunComplete`: sets run status + `completed_at` + `duration_ms`.
/// - `ToolUseStart`: delegates to the `AsyncGate` via a per-request
///   `tokio::spawn` so the orchestrator loop stays non-blocking.
///
/// Other event variants are passed through (captured by `EventPersister`
/// for the event log; not persisted into run/step rows).
///
/// Per-event errors log via `tracing::error!` and the loop continues.
pub struct PipelineOrchestrator;

impl PipelineOrchestrator {
    /// Spawn the orchestrator.
    ///
    /// # Arguments
    ///
    /// - `bus`: the in-process event bus. Drop to signal shutdown.
    /// - `runs`: run row repository.
    /// - `steps`: step row repository.
    /// - `gate`: async permission gate. Shared via `Arc`; one instance per
    ///   orchestrator. The gate holds the `SessionAllowlist` and the
    ///   `ConfigGate` which embeds the allowlist/denylist.
    ///
    /// Per-tool-call gating uses approach (a): a `tokio::spawn` per
    /// `ToolUseStart` so the bus-consuming loop is never blocked behind the
    /// gate's interactive-prompt timeout (up to 60 s by default).
    pub fn spawn(bus: EventBus, runs: RunRepo, steps: StepRepo, gate: Arc<AsyncGate>) -> JoinHandle<()> {
        let mut subscriber = bus.subscribe();
        tokio::spawn(async move {
            loop {
                match subscriber.recv().await {
                    Ok(envelope) => {
                        if let Event::ToolUseStart { .. } = &envelope.event {
                            handle_tool_use_start(envelope, gate.clone(), bus.clone());
                        } else if let Err(e) = apply_event(&envelope, &runs, &steps) {
                            tracing::error!(
                                run_id = %envelope.run_id,
                                event_id = %envelope.event_id,
                                error = %e,
                                "orchestrator: failed to apply event; continuing",
                            );
                        }
                    }
                    Err(RecvError::Closed) => {
                        tracing::info!("orchestrator: bus closed; shutting down");
                        break;
                    }
                    Err(RecvError::Lagged(n)) => {
                        tracing::warn!(
                            skipped = n,
                            "orchestrator: lagged behind bus; skipped {n} envelopes",
                        );
                    }
                }
            }
        })
    }
}

/// Spawn a per-request task that evaluates the gate for a `ToolUseStart`
/// envelope and publishes an audit `PermissionResolved` envelope for
/// allowlist/denylist hits.
///
/// Approach (a): non-blocking. The orchestrator bus loop is not held behind
/// the gate's interactive-prompt timeout (up to 60 s). Ordering relative to
/// subsequent events on the same step is not guaranteed, which is acceptable
/// for an observational gate (the tool call has already executed by the time
/// we see the envelope).
///
/// Audit envelope publishing policy:
/// - `AllowlistConfig` / `DenylistConfig` sources: the gate never touches the
///   bus for these (pure sync evaluation). The orchestrator publishes the
///   `PermissionResolved` for the audit log.
/// - `User` / `Timeout` / `Cancelled` / `SessionAllowlist` sources: the gate
///   or the UI already emitted the `PermissionResolved` envelope. The
///   orchestrator does NOT double-publish.
fn handle_tool_use_start(envelope: EventEnvelope, gate: Arc<AsyncGate>, bus: EventBus) {
    let run_id = envelope.run_id.clone();
    let step_id = envelope.step_id.clone();

    let (tool_name, input) = match &envelope.event {
        Event::ToolUseStart { tool_name, input, .. } => (tool_name.clone(), input.clone()),
        _ => return, // unreachable — caller checks the variant
    };

    tokio::spawn(async move {
        let arg = extract_tool_arg(&tool_name, &input);

        // TODO: thread a real orchestrator-level CancellationToken once the
        // orchestrator gains a graceful-shutdown cancellation path (later step).
        let cancel = CancellationToken::new();

        let outcome = gate
            .evaluate_async(
                &tool_name,
                &arg,
                &run_id,
                step_id.as_deref(),
                cancel,
                OnTimeout::Deny,
            )
            .await;

        match outcome {
            GateOutcome::AnnotateAllow {
                source: PermissionSource::AllowlistConfig,
            } => {
                publish_audit_resolved(
                    &bus,
                    &run_id,
                    step_id.as_deref(),
                    PermissionDecision::AllowOnce,
                    PermissionSource::AllowlistConfig,
                );
            }
            GateOutcome::AnnotateDeny {
                source: PermissionSource::DenylistConfig,
            } => {
                tracing::warn!(
                    tool_name = %tool_name,
                    arg = %arg,
                    "permission gate denied tool call (advisory — call already executed by subprocess)"
                );
                publish_audit_resolved(
                    &bus,
                    &run_id,
                    step_id.as_deref(),
                    PermissionDecision::Deny,
                    PermissionSource::DenylistConfig,
                );
            }
            _ => {
                // User / Timeout / Cancelled / SessionAllowlist:
                // the gate or the UI already published PermissionResolved.
                // The orchestrator does not double-publish.
            }
        }
    });
}

/// Publish an audit `PermissionResolved` envelope with a fresh ULID `request_id`.
///
/// Used for allowlist and denylist short-circuit outcomes that never go through
/// the bus on the gate side.
fn publish_audit_resolved(
    bus: &EventBus,
    run_id: &str,
    step_id: Option<&str>,
    decision: PermissionDecision,
    source: PermissionSource,
) {
    bus.publish(EventEnvelope::now(
        run_id.to_string(),
        step_id.map(str::to_string),
        Event::PermissionResolved {
            request_id: ulid::Ulid::new().to_string(),
            decision,
            source,
        },
    ));
}

/// Extract the argument string for a tool call from its `input` JSON value.
///
/// # Bash / bash
/// Tries `input.command` (Claude Code convention) then `input.cmd` (Copilot
/// convention). Falls back to empty string if neither key is present.
///
/// # Other tools
/// Serializes the entire `input` value to a compact JSON string. Allowlist /
/// denylist patterns for non-Bash tools must therefore match the serialized
/// form (e.g. `Read({"file_path":"/tmp/*"})`) — or use the wildcard shorthand
/// `Read(*)` which matches any JSON via the glob `*`.
///
/// This is a v1 limitation; a per-tool registry that exposes a canonical arg
/// string would supersede this in a later step.
pub(crate) fn extract_tool_arg(tool: &str, input: &serde_json::Value) -> String {
    match tool {
        "Bash" | "bash" => input
            .get("command")
            .and_then(|v| v.as_str())
            .or_else(|| input.get("cmd").and_then(|v| v.as_str()))
            .map(String::from)
            .unwrap_or_default(),
        _ => serde_json::to_string(input).unwrap_or_default(),
    }
}

fn apply_event(envelope: &EventEnvelope, runs: &RunRepo, steps: &StepRepo) -> Result<()> {
    match &envelope.event {
        Event::RunStarted { .. } => {
            let current = runs
                .get(&envelope.run_id)?
                .ok_or_else(|| crate::CoreError::Db(format!("run not found: {}", envelope.run_id)))?
                .status;
            if current == RunStatus::Running {
                // Already transitioned (e.g., duplicate RunStarted, replay). No-op.
                return Ok(());
            }
            runs.transition(&envelope.run_id, RunStatus::Running)?;
        }
        Event::StepStarted { .. } => {
            if let Some(step_id) = &envelope.step_id {
                steps.transition(step_id, StepStatus::Running)?;
            }
        }
        Event::StepComplete {
            status,
            duration_ms,
            ..
        } => {
            if let Some(step_id) = &envelope.step_id {
                steps.mark_complete(
                    step_id,
                    *status,
                    envelope.timestamp_ms,
                    *duration_ms as i64,
                )?;
            }
        }
        Event::RunComplete {
            status,
            duration_ms,
            ..
        } => {
            runs.mark_complete(
                &envelope.run_id,
                *status,
                envelope.timestamp_ms,
                *duration_ms as i64,
            )?;
        }
        _ => {
            // Other events captured by EventPersister only.
        }
    }
    Ok(())
}
