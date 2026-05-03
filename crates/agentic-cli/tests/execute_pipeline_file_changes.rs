//! Integration test: `execute_pipeline` wires `ToolUseObserver` per step and
//! writes a `file_changes.diff` under `<data_dir>/runs/<run_id>/step-00-<agent>/`.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agentic_cli::ticket_run::{BackendFactory, PipelineRunContext, execute_pipeline};
use agentic_core::backends::EventSink;
use agentic_core::permissions::config::{OnTimeout, PermissionsConfig, PermissionsSettings};
use agentic_core::permissions::gate_async::AsyncGate;
use agentic_core::{
    Backend, BackendId, Db, Event, EventBus, EventEnvelope, EventPersister, ExecuteOutcome,
    ExecuteRequest, HealthStatus, ModelId, Paths, Pipeline, PipelineStep, Run, RunRepo, RunStatus,
    StepStatus, TokenUsage, Workspace, WorkspaceRepo,
};
use async_trait::async_trait;
use rusqlite::params;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Mutex;

// ---------------------------------------------------------------------------
// FileEditingBackend: emits ToolUseStart, writes files to disk, then completes
fn passthrough_gate(bus: &EventBus) -> Arc<AsyncGate> {
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

// ---------------------------------------------------------------------------

/// A one-off test backend that:
/// 1. Publishes `StepStarted`.
/// 2. Writes `<ws>/greet.txt = "hi\n"` to disk.
/// 3. Publishes `ToolUseStart { Write, greet.txt }`.
/// 4. Sleeps briefly so the observer can capture the pre-state.
/// 5. Overwrites `<ws>/greet.txt = "hello\n"`.
/// 6. Publishes `ToolUseEnd`.
/// 7. Publishes `StepComplete { Passed }`.
struct FileEditingBackend {
    ws_root: PathBuf,
    step_id_out: Arc<Mutex<Option<String>>>,
}

impl FileEditingBackend {
    fn new(ws_root: PathBuf, step_id_out: Arc<Mutex<Option<String>>>) -> Self {
        Self {
            ws_root,
            step_id_out,
        }
    }
}

#[async_trait]
impl Backend for FileEditingBackend {
    fn id(&self) -> BackendId {
        BackendId("file-editing-test".to_string())
    }

    fn display_name(&self) -> &str {
        "FileEditingTestBackend"
    }

    fn supported_models(&self) -> Vec<ModelId> {
        vec![ModelId("fake".to_string())]
    }

    async fn health_check(&self) -> agentic_core::Result<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }

    async fn execute(
        &self,
        req: ExecuteRequest,
        event_sink: EventSink,
    ) -> agentic_core::Result<ExecuteOutcome> {
        // Record the step_id so the test can look it up later.
        *self.step_id_out.lock().await = Some(req.step_id.0.clone());

        let run_id = req.run_id.0.clone();
        let step_id_str = req.step_id.0.clone();

        let greet_path = self.ws_root.join("greet.txt");

        // 1. StepStarted
        let _ = event_sink.send(EventEnvelope::now(
            run_id.clone(),
            Some(step_id_str.clone()),
            Event::StepStarted {
                agent: "tdd-developer".to_string(),
                model: ModelId("fake".to_string()),
            },
        ));

        // 2. Write initial content to disk.
        std::fs::write(&greet_path, b"hi\n").unwrap();

        // 3. Publish ToolUseStart so the observer captures the pre-state ("hi\n").
        let _ = event_sink.send(EventEnvelope::now(
            run_id.clone(),
            Some(step_id_str.clone()),
            Event::ToolUseStart {
                tool_call_id: "t1".to_string(),
                tool_name: "Write".to_string(),
                input: json!({
                    "file_path": greet_path.to_string_lossy().as_ref(),
                    "content": "hello"
                }),
            },
        ));

        // 4. Yield to scheduler so the observer task processes ToolUseStart and
        //    captures the pre-state before we overwrite.
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }

        // 5. Overwrite the file (simulates Claude writing the new content).
        std::fs::write(&greet_path, b"hello\n").unwrap();

        // 6. ToolUseEnd
        let _ = event_sink.send(EventEnvelope::now(
            run_id.clone(),
            Some(step_id_str.clone()),
            Event::ToolUseEnd {
                tool_call_id: "t1".to_string(),
                exit_code: Some(0),
                duration_ms: 10,
            },
        ));

        // 7. StepComplete
        let _ = event_sink.send(EventEnvelope::now(
            run_id.clone(),
            Some(step_id_str.clone()),
            Event::StepComplete {
                status: StepStatus::Passed,
                summary: "wrote greet.txt".to_string(),
                token_usage: TokenUsage::default(),
                cost_usd: None,
                duration_ms: 30,
            },
        ));

        Ok(ExecuteOutcome {
            status: StepStatus::Passed,
            summary: "wrote greet.txt".to_string(),
            token_usage: TokenUsage::default(),
            cost_usd: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn setup_workspace(tmp: &TempDir, agent_names: &[&str]) -> (Db, EventBus, String) {
    let base = tmp.path();
    let paths = Paths::for_tests(base);
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();

    let agents_dir = base.join(".agentic").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();

    for &name in agent_names {
        let content =
            format!("+++\nname = \"{name}\"\ndescription = \"test agent\"\n+++\nYou are {name}.\n");
        std::fs::write(agents_dir.join(format!("{name}.md")), content).unwrap();
    }

    let ws_id = "test-ws-fc-01".to_string();
    {
        let ws_repo = WorkspaceRepo::new(&db);
        ws_repo
            .insert(Workspace {
                id: ws_id.clone(),
                name: "test".to_string(),
                root_path: base.to_string_lossy().to_string(),
                remote_url: None,
                profile: "custom".to_string(),
                created_at: 0,
                last_opened: 0,
            })
            .unwrap();
    }

    let bus = EventBus::new();
    (db, bus, ws_id)
}

fn seed_run(db: &Db, run_id: &str, ws_id: &str) {
    let runs = RunRepo::new(db);
    runs.insert(Run {
        id: run_id.to_string(),
        workspace_id: ws_id.to_string(),
        pipeline_name: "default".to_string(),
        status: RunStatus::Pending,
        ticket_type: None,
        ticket_ref: None,
        ticket_title: None,
        ticket_body: None,
        backend: "file-editing-test".to_string(),
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
}

// ---------------------------------------------------------------------------
// Test 3: execute_pipeline writes per-step diff file
// ---------------------------------------------------------------------------

#[tokio::test]
async fn execute_pipeline_writes_per_step_diff_file() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let (db, bus, ws_id) = setup_workspace(&tmp, &["tdd-developer"]);
    let ws_root = base.to_path_buf();

    let run_id = "fc-run-01";
    seed_run(&db, run_id, &ws_id);

    // Capture the step_id assigned by execute_pipeline so we can query it.
    let step_id_out: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let step_id_out_clone = step_id_out.clone();

    let paths = Paths::for_tests(base);
    paths.ensure_dirs().unwrap();

    let orch_handle = agentic_core::PipelineOrchestrator::spawn(
        bus.clone(),
        RunRepo::new(&db),
        agentic_core::StepRepo::new(&db),
        passthrough_gate(&bus),
    );
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // Build a 1-step pipeline with just "tdd-developer".
    let pipeline = Pipeline {
        steps: vec![PipelineStep {
            agent: "tdd-developer".to_string(),
            stop_on_failure: false,
            allowed_questions: None,
            qa_fix_loop_cap: None,
        }],
    };

    let ws_root_clone = ws_root.clone();
    let factory: BackendFactory<'_> = Box::new(move |_step: &PipelineStep| -> Box<dyn Backend> {
        Box::new(FileEditingBackend::new(
            ws_root_clone.clone(),
            step_id_out_clone.clone(),
        ))
    });

    let result = execute_pipeline(
        PipelineRunContext {
            db: &db,
            bus: &bus,
            run_id,
            ws_id: &ws_id,
            ws_root: &ws_root,
            ticket_text: "create greet.txt",
            model_override: None,
            paths: &paths,

            external_cancel: None,
        },
        &pipeline,
        factory,
    )
    .await;

    assert!(
        result.is_ok(),
        "execute_pipeline should succeed; got: {:?}",
        result
    );

    drop(bus);
    orch_handle.await.unwrap();
    pers_handle.await.unwrap();

    // Retrieve the step_id.
    let step_id = step_id_out
        .lock()
        .await
        .clone()
        .expect("step_id should have been set");

    // Assert: diff file at expected path contains "+hello".
    let diff_path = paths
        .step_dir(run_id, 0, "tdd-developer")
        .join("file_changes.diff");
    assert!(
        diff_path.exists(),
        "diff file should exist at {}",
        diff_path.display()
    );
    let diff_content = std::fs::read_to_string(&diff_path).unwrap();
    assert!(
        diff_content.contains("+hello"),
        "diff should contain '+hello'; got:\n{diff_content}"
    );

    // Assert: FileChange event in stream_events for this step.
    {
        let conn = db.conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stream_events \
                 WHERE event_type = 'FileChange' AND step_id = ?1",
                params![step_id],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            count >= 1,
            "should have at least one FileChange row for step_id={step_id}; got {count}"
        );
    }

    // Assert: run is Completed, step is Passed.
    let runs_repo = RunRepo::new(&db);
    let run = runs_repo.get(run_id).unwrap().unwrap();
    assert_eq!(run.status, RunStatus::Completed);

    let steps_repo = agentic_core::StepRepo::new(&db);
    let steps = steps_repo.list_by_run(run_id).unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0].status, StepStatus::Passed);
}
