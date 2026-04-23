#[cfg(any(test, feature = "testing"))]
use async_trait::async_trait;
#[cfg(any(test, feature = "testing"))]
use super::{Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId};
#[cfg(any(test, feature = "testing"))]
use crate::events::Event;
#[cfg(any(test, feature = "testing"))]
use crate::Result;

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
        unimplemented!()
    }

    fn display_name(&self) -> &str {
        unimplemented!()
    }

    fn supported_models(&self) -> Vec<ModelId> {
        unimplemented!()
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        unimplemented!()
    }

    async fn execute(
        &self,
        _req: ExecuteRequest,
        _event_sink: EventSink,
    ) -> Result<ExecuteOutcome> {
        unimplemented!()
    }
}
