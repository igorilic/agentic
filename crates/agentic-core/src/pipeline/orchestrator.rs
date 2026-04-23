use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinHandle;

use crate::Result;
use crate::db::runs::RunRepo;
use crate::db::steps::StepRepo;
use crate::events::{Event, EventBus, EventEnvelope, StepStatus};

/// Background orchestrator consuming events from the bus and applying
/// them to `runs`/`run_steps` rows. One per bus per DB.
///
/// Consumed event variants:
/// - `StepStarted`: transitions the step row to `Running`.
/// - `StepComplete`: sets step status + `completed_at` (= envelope
///   `timestamp_ms`) + `duration_ms` (from event payload).
/// - `RunComplete`: sets run status + `completed_at` + `duration_ms`.
///
/// Other event variants are passed through (captured by `EventPersister`
/// for the event log; not persisted into run/step rows).
///
/// Per-event errors log via `tracing::error!` and the loop continues.
pub struct PipelineOrchestrator;

impl PipelineOrchestrator {
    pub fn spawn(bus: EventBus, runs: RunRepo, steps: StepRepo) -> JoinHandle<()> {
        let mut subscriber = bus.subscribe();
        tokio::spawn(async move {
            loop {
                match subscriber.recv().await {
                    Ok(envelope) => {
                        if let Err(e) = apply_event(&envelope, &runs, &steps) {
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

fn apply_event(envelope: &EventEnvelope, runs: &RunRepo, steps: &StepRepo) -> Result<()> {
    match &envelope.event {
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
