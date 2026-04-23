#[cfg(any(test, feature = "testing"))]
use super::{
    Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId,
    TokenUsage,
};
#[cfg(any(test, feature = "testing"))]
use crate::Result;
#[cfg(any(test, feature = "testing"))]
use crate::events::{Event, EventEnvelope, StepStatus};
#[cfg(any(test, feature = "testing"))]
use async_trait::async_trait;

/// Test-only backend that replays a canned sequence of events.
///
/// Not a real backend — `supported_models` returns empty, `health_check`
/// always returns `Healthy`. Used by downstream orchestrator/pipeline
/// tests to simulate backend behavior without a real LLM.
#[cfg(any(test, feature = "testing"))]
#[derive(Debug, Clone)]
pub struct ScriptedBackend {
    script: Vec<Event>,
}

#[cfg(any(test, feature = "testing"))]
impl ScriptedBackend {
    pub fn new(script: Vec<Event>) -> Self {
        Self { script }
    }
}

#[cfg(any(test, feature = "testing"))]
#[async_trait]
impl Backend for ScriptedBackend {
    fn id(&self) -> BackendId {
        BackendId("scripted".to_string())
    }

    fn display_name(&self) -> &str {
        "Scripted"
    }

    fn supported_models(&self) -> Vec<ModelId> {
        Vec::new()
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }

    async fn execute(&self, req: ExecuteRequest, event_sink: EventSink) -> Result<ExecuteOutcome> {
        let mut saw_unrecoverable_error = false;

        for event in &self.script {
            // Honor cancellation before each event emission.
            if req.cancel.is_cancelled() {
                return Ok(ExecuteOutcome {
                    status: StepStatus::Failed,
                    summary: "cancelled".to_string(),
                    token_usage: TokenUsage::default(),
                    cost_usd: None,
                });
            }

            if let Event::Error {
                recoverable: false, ..
            } = event
            {
                saw_unrecoverable_error = true;
            }

            let envelope = EventEnvelope::now(
                req.run_id.0.clone(),
                Some(req.step_id.0.clone()),
                event.clone(),
            );
            // Silently ignore send error — no subscribers just means the
            // broadcast is empty; real tests seed receivers before execute.
            let _ = event_sink.send(envelope);
        }

        let status = if saw_unrecoverable_error {
            StepStatus::Failed
        } else {
            StepStatus::Passed
        };

        Ok(ExecuteOutcome {
            status,
            summary: "scripted".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
        })
    }
}
