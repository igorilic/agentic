use std::path::PathBuf;

use agentic_core::{
    Backend, BackendId, EventSink, ExecuteOutcome, ExecuteRequest, HealthStatus, ModelId, Result,
    RunId, StepId, StepStatus, TokenUsage, WorkspaceRef,
};
use async_trait::async_trait;

struct NullBackend;

#[async_trait]
impl Backend for NullBackend {
    fn id(&self) -> BackendId {
        BackendId("null".to_string())
    }

    fn display_name(&self) -> &str {
        "Null"
    }

    fn supported_models(&self) -> Vec<ModelId> {
        Vec::new()
    }

    async fn health_check(&self) -> Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }

    async fn execute(&self, _req: ExecuteRequest, _sink: EventSink) -> Result<ExecuteOutcome> {
        Ok(ExecuteOutcome {
            status: StepStatus::Passed,
            summary: "null executed".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
        })
    }
}

#[test]
fn null_backend_satisfies_send_sync_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}
    assert_send_sync_static::<NullBackend>();
}

#[test]
fn null_backend_is_dyn_compatible() {
    let _boxed: Box<dyn Backend> = Box::new(NullBackend);
}

#[tokio::test]
async fn null_backend_executes_and_returns_outcome() {
    let (sink, _rx) = tokio::sync::broadcast::channel(16);
    let backend = NullBackend;

    let req = ExecuteRequest {
        workspace: WorkspaceRef {
            id: "ws1".to_string(),
            root_path: PathBuf::from("/tmp/ws1"),
        },
        run_id: RunId("run1".to_string()),
        step_id: StepId("step1".to_string()),
        agent_name: "test".to_string(),
        agent_prompt: "prompt".to_string(),
        user_context: "ctx".to_string(),
        model: None,
        tools: Vec::new(),
        cwd: PathBuf::from("/tmp/ws1"),
        timeout: None,
        cancel: tokio_util::sync::CancellationToken::new(),
    };

    let outcome = backend.execute(req, sink).await.expect("execute");
    assert_eq!(outcome.status, StepStatus::Passed);
    assert_eq!(outcome.summary, "null executed");
}
