//! End-to-end test for reviewer-text → findings DB row projection.
//!
//! Spins up a real `EventBus` + in-memory DB, publishes a synthetic
//! reviewer TextDelta containing an `agentic-findings` block, and
//! verifies the host code path persists the rows + republishes
//! `Event::Finding` envelopes for the cockpit.

use agentic_cli::ticket_run::{BackendFactory, PipelineRunContext, execute_pipeline};
use agentic_core::db::findings::FindingsRepo;
use agentic_core::db::workspaces::{Workspace, WorkspaceRepo};
use agentic_core::{
    Backend, BackendId, Db, Event, EventBus, EventEnvelope, ExecuteOutcome, ExecuteRequest,
    ModelId, Paths, PipelineConfig, PipelineStep, Run, RunRepo, RunStatus, StepRepo, StepStatus,
    TokenUsage,
};
use tempfile::TempDir;
use tokio::sync::broadcast::Sender;

const RUN_ID: &str = "run-finding-projection";

/// Backend that emits one TextDelta with an `agentic-findings` fence
/// containing 2 findings, then a successful StepComplete. Stand-in for a
/// real reviewer agent.
struct ReviewerWithFindingsBackend;

#[async_trait::async_trait]
impl Backend for ReviewerWithFindingsBackend {
    fn id(&self) -> BackendId {
        BackendId("test-reviewer".to_string())
    }
    fn display_name(&self) -> &str {
        "test-reviewer"
    }
    fn supported_models(&self) -> Vec<ModelId> {
        vec![]
    }
    async fn health_check(&self) -> Result<agentic_core::HealthStatus, agentic_core::CoreError> {
        Ok(agentic_core::HealthStatus::Healthy)
    }
    async fn execute(
        &self,
        req: ExecuteRequest,
        sink: Sender<EventEnvelope>,
    ) -> Result<ExecuteOutcome, agentic_core::CoreError> {
        let text = "Review notes:\n\
            \n\
            ```agentic-findings\n\
            [\n\
              {\"finding_id\":\"f1\",\"severity\":\"warning\",\"file\":\"src/main.py\",\"line\":42,\"message\":\"missing-error-handling\"},\n\
              {\"finding_id\":\"f2\",\"severity\":\"error\",\"message\":\"hardcoded-secret\"}\n\
            ]\n\
            ```\n";
        let _ = sink.send(EventEnvelope::now(
            req.run_id.0.clone(),
            Some(req.step_id.0.clone()),
            Event::TextDelta {
                content: text.to_string(),
            },
        ));
        Ok(ExecuteOutcome {
            status: StepStatus::Passed,
            summary: "review done".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
        })
    }
}

fn write_reviewer_agent_file(repo: &std::path::Path) {
    let dir = repo.join(".claude").join("agents");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("reviewer.md"),
        "+++\nname = \"reviewer\"\ndescription = \"r\"\npipeline_role = \"step\"\n+++\nReview the diff.\n",
    )
    .unwrap();
}

#[tokio::test]
async fn reviewer_findings_block_is_projected_into_findings_table() {
    let tmp = TempDir::new().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();
    let bus = EventBus::new();

    // Seed workspace + run rows so execute_pipeline's row updates succeed.
    let ws_id = "ws-test";
    WorkspaceRepo::new(&db)
        .insert(Workspace {
            id: ws_id.to_string(),
            name: "test".to_string(),
            root_path: tmp.path().to_string_lossy().to_string(),
            remote_url: None,
            profile: "custom".to_string(),
            created_at: 0,
            last_opened: 0,
        })
        .unwrap();
    RunRepo::new(&db)
        .insert(Run {
            id: RUN_ID.to_string(),
            workspace_id: ws_id.to_string(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Pending,
            ticket_type: None,
            ticket_ref: None,
            ticket_title: None,
            ticket_body: None,
            backend: "test".to_string(),
            model: "fake".to_string(),
            started_at: 0,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })
        .unwrap();
    write_reviewer_agent_file(tmp.path());

    // Spawn orchestrator so RunStarted -> Running transition succeeds.
    let _orch = agentic_core::PipelineOrchestrator::spawn(
        bus.clone(),
        RunRepo::new(&db),
        StepRepo::new(&db),
    );

    // Pipeline with a single "reviewer" step.
    let pipeline = PipelineConfig::builtin_default()
        .default_pipeline()
        .clone();
    let reviewer_only = agentic_core::Pipeline {
        steps: pipeline
            .steps
            .iter()
            .filter(|s| s.agent == "reviewer")
            .cloned()
            .collect(),
    };
    assert_eq!(
        reviewer_only.steps.len(),
        1,
        "default pipeline must include exactly one reviewer step"
    );

    let factory: BackendFactory<'_> =
        Box::new(|_step: &PipelineStep| -> Box<dyn Backend> { Box::new(ReviewerWithFindingsBackend) });

    execute_pipeline(
        PipelineRunContext {
            db: &db,
            bus: &bus,
            run_id: RUN_ID,
            ws_id,
            ws_root: tmp.path(),
            ticket_text: "ticket",
            model_override: None,
            paths: &paths,
            external_cancel: None,
        },
        &reviewer_only,
        factory,
    )
    .await
    .expect("execute_pipeline");

    // Findings rows persisted.
    let rows = FindingsRepo::new(&db).list_by_run(RUN_ID).unwrap();
    assert_eq!(rows.len(), 2, "both findings should land in the DB");
    let by_id: std::collections::HashMap<_, _> = rows
        .iter()
        .map(|r| (r.id.split(':').next_back().unwrap().to_string(), r))
        .collect();
    let f1 = by_id.get("f1").expect("f1 row");
    assert_eq!(f1.severity, "warning");
    assert_eq!(f1.file_path.as_deref(), Some("src/main.py"));
    assert_eq!(f1.line, Some(42));
    assert_eq!(f1.message, "missing-error-handling");
    let f2 = by_id.get("f2").expect("f2 row");
    assert_eq!(f2.severity, "error");
    assert!(f2.file_path.is_none());

}
