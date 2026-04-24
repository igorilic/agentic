#![deny(unsafe_code)]

use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use agentic_core::{
    Backend, Db, Event, EventBus, EventEnvelope, ExecuteRequest, ModelId, Pipeline, PipelineStep,
    RunId, RunRepo, RunStatus, Step, StepId, StepRepo, StepStatus, WorkspaceRef, discover_agent,
};

/// Injectable factory: given a `PipelineStep`, produce a backend for that step.
pub type BackendFactory<'a> = Box<dyn Fn(&PipelineStep) -> Box<dyn Backend> + Send + Sync + 'a>;

/// Execute all steps in `pipeline` against `ticket_text`.
///
/// For each step:
/// 1. Inserts a `Step` row with `status = Pending`.
/// 2. Discovers the agent file under `ws_root`.
/// 3. Builds an `ExecuteRequest` and calls `backend_factory(step).execute(req, sink)`.
/// 4. If a step fails and `stop_on_failure` is set, returns `Err` immediately.
///
/// After the loop, publishes `RunComplete { status: Completed }`.
pub async fn execute_pipeline(
    db: &Db,
    bus: &EventBus,
    run_id: &str,
    ws_id: &str,
    ws_root: &Path,
    pipeline: &Pipeline,
    ticket_text: &str,
    model_override: Option<ModelId>,
    backend_factory: BackendFactory<'_>,
) -> Result<()> {
    let runs = RunRepo::new(db);
    let steps = StepRepo::new(db);
    let run_start = Instant::now();

    for (i, pipeline_step) in pipeline.steps.iter().enumerate() {
        let step_id = ulid::Ulid::new().to_string();

        // Insert step row as Pending.
        steps.insert(Step {
            id: step_id.clone(),
            run_id: run_id.to_string(),
            seq: i as i64,
            agent_name: pipeline_step.agent.clone(),
            status: StepStatus::Pending,
            started_at: None,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            retry_count: 0,
        })?;

        // Discover agent file.
        let agent = discover_agent(ws_root, &pipeline_step.agent).map_err(|e| {
            anyhow::anyhow!(
                "agent '{}' not found in workspace '{}': {}",
                pipeline_step.agent,
                ws_root.display(),
                e
            )
        })?;

        // Build effective model: CLI override wins, then agent's own default.
        let model = model_override
            .clone()
            .or_else(|| agent.model.as_deref().map(|m| ModelId(m.to_string())));

        // Build the execute request.
        let req = ExecuteRequest {
            workspace: WorkspaceRef {
                id: ws_id.to_string(),
                root_path: ws_root.to_path_buf(),
            },
            run_id: RunId(run_id.to_string()),
            step_id: StepId(step_id.clone()),
            agent_name: pipeline_step.agent.clone(),
            agent_prompt: agent.system_prompt.clone(),
            user_context: ticket_text.to_string(),
            model,
            tools: agent
                .tools
                .unwrap_or_default()
                .into_iter()
                .map(agentic_core::ToolName)
                .collect(),
            cwd: ws_root.to_path_buf(),
            timeout: agent.timeout_seconds.map(std::time::Duration::from_secs),
            cancel: CancellationToken::new(),
        };

        let backend = backend_factory(pipeline_step);
        let outcome = backend.execute(req, bus.sender()).await.map_err(|e| {
            anyhow::anyhow!("backend error for agent '{}': {}", pipeline_step.agent, e)
        })?;

        if outcome.status == StepStatus::Failed && pipeline_step.stop_on_failure {
            // Publish RunComplete(Failed) then return error.
            let elapsed_ms = run_start.elapsed().as_millis() as u64;
            bus.publish(EventEnvelope::now(
                run_id.to_string(),
                None,
                Event::RunComplete {
                    status: RunStatus::Failed,
                    duration_ms: elapsed_ms,
                    summary: format!(
                        "step '{}' failed and stop_on_failure is set",
                        pipeline_step.agent
                    ),
                },
            ));
            return Err(anyhow::anyhow!(
                "step '{}' failed: {}",
                pipeline_step.agent,
                outcome.summary
            ));
        }
    }

    // All steps completed; publish final RunComplete.
    let elapsed_ms = run_start.elapsed().as_millis() as u64;
    bus.publish(EventEnvelope::now(
        run_id.to_string(),
        None,
        Event::RunComplete {
            status: RunStatus::Completed,
            duration_ms: elapsed_ms,
            summary: "ticket run complete".to_string(),
        },
    ));

    // Give the orchestrator time to process the RunComplete event.
    // We do NOT await drain here — caller is responsible for shutting down infra.
    let _ = runs;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Arc;

    use agentic_core::{
        Db, Event, EventBus, EventPersister, ModelId, Paths, PipelineConfig, PipelineStep, Run,
        RunRepo, RunStatus, ScriptedBackend, StepRepo, StepStatus, TokenUsage,
    };
    use rusqlite::params;
    use tempfile::TempDir;

    /// Create a minimal tempdir workspace with DB and agent fixture files.
    fn setup_workspace(agent_names: &[&str]) -> (TempDir, Db, EventBus, String) {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        let paths = Paths::for_tests(base);
        paths.ensure_dirs().unwrap();
        let db = Db::open(&paths).unwrap();

        // Create .agentic/agents directory.
        let agents_dir = base.join(".agentic").join("agents");
        std::fs::create_dir_all(&agents_dir).unwrap();

        // Write minimal agent fixture files for each requested agent.
        for &name in agent_names {
            let content = format!(
                "+++\nname = \"{name}\"\ndescription = \"test agent\"\n+++\nYou are the {name} agent.\n"
            );
            std::fs::write(agents_dir.join(format!("{name}.md")), content).unwrap();
        }

        // Seed workspace row.
        let ws_id = "test-ws-01".to_string();
        {
            let conn = db.conn().unwrap();
            conn.execute(
                "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
                 VALUES (?1, 'test', ?2, 'custom', 0, 0)",
                params![ws_id, base.to_string_lossy().to_string()],
            )
            .unwrap();
        }

        let bus = EventBus::new();
        (tmp, db, bus, ws_id)
    }

    /// Seed a run row in Running status (workaround for GH #17).
    fn seed_run(db: &Db, run_id: &str, ws_id: &str) {
        let runs = RunRepo::new(db);
        runs.insert(Run {
            id: run_id.to_string(),
            workspace_id: ws_id.to_string(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Running,
            ticket_type: None,
            ticket_ref: None,
            ticket_title: None,
            ticket_body: None,
            backend: "claude-code".to_string(),
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

    /// Build a `BackendFactory` where every step returns a scripted backend
    /// emitting StepStarted → TextDelta → StepComplete(Passed).
    fn passing_factory() -> BackendFactory<'static> {
        Box::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(ScriptedBackend::new(vec![
                Event::StepStarted {
                    agent: "test".to_string(),
                    model: ModelId("fake".to_string()),
                },
                Event::TextDelta {
                    content: "step did work".to_string(),
                },
                Event::StepComplete {
                    status: StepStatus::Passed,
                    summary: "ok".to_string(),
                    token_usage: TokenUsage::default(),
                    cost_usd: None,
                    duration_ms: 50,
                },
            ]))
        })
    }

    #[tokio::test]
    async fn execute_pipeline_happy_path_drives_all_steps() {
        let agent_names = ["architect", "tdd-developer", "qa", "reviewer"];
        let (tmp, db, bus, ws_id) = setup_workspace(&agent_names);
        let ws_root = tmp.path().to_path_buf();

        let run_id = "happy-run-01";
        seed_run(&db, run_id, &ws_id);

        // Subscribe before spawning infra so no events are lost.
        let orch_handle = agentic_core::PipelineOrchestrator::spawn(
            bus.clone(),
            RunRepo::new(&db),
            StepRepo::new(&db),
        );
        let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

        // Build a 4-step pipeline.
        let pipeline = PipelineConfig::builtin_default();
        let pipeline = pipeline.default_pipeline();

        let result = execute_pipeline(
            &db,
            &bus,
            run_id,
            &ws_id,
            &ws_root,
            pipeline,
            "implement feature X",
            None,
            passing_factory(),
        )
        .await;

        assert!(result.is_ok(), "expected Ok, got: {:?}", result);

        // Shut down infrastructure.
        drop(bus);
        orch_handle.await.unwrap();
        pers_handle.await.unwrap();

        // Assert all 4 step rows are Passed with completed_at set and summary = "scripted".
        let steps_repo = StepRepo::new(&db);
        let steps = steps_repo.list_by_run(run_id).unwrap();
        assert_eq!(steps.len(), 4, "expected 4 steps");
        for step in &steps {
            assert_eq!(
                step.status,
                StepStatus::Passed,
                "step '{}' should be Passed, was {:?}",
                step.agent_name,
                step.status
            );
            assert!(
                step.completed_at.is_some(),
                "step '{}' should have completed_at set",
                step.agent_name
            );
        }

        // Assert run row is Completed.
        let runs_repo = RunRepo::new(&db);
        let run = runs_repo.get(run_id).unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn execute_pipeline_stops_on_failure_when_step_sets_stop_on_failure() {
        // The default pipeline has stop_on_failure = true for architect (step 0)
        // and tdd-developer (step 1). We'll make step 1 fail.
        let agent_names = ["architect", "tdd-developer", "qa", "reviewer"];
        let (tmp, db, bus, ws_id) = setup_workspace(&agent_names);
        let ws_root = tmp.path().to_path_buf();

        let run_id = "fail-run-01";
        seed_run(&db, run_id, &ws_id);

        let orch_handle = agentic_core::PipelineOrchestrator::spawn(
            bus.clone(),
            RunRepo::new(&db),
            StepRepo::new(&db),
        );
        let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

        // Build a factory: step 0 passes, step 1 fails, rest would pass.
        let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let call_count_clone = call_count.clone();
        let factory: BackendFactory<'_> =
            Box::new(move |_step: &PipelineStep| -> Box<dyn Backend> {
                let n = call_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if n == 1 {
                    // Second step fails: emit unrecoverable Error so ExecuteOutcome.status == Failed.
                    // Also emit StepComplete(Failed) so orchestrator updates the DB row.
                    Box::new(ScriptedBackend::new(vec![
                        Event::StepStarted {
                            agent: "test".to_string(),
                            model: ModelId("fake".to_string()),
                        },
                        Event::Error {
                            code: "TEST_FAIL".to_string(),
                            message: "test failure".to_string(),
                            recoverable: false,
                            retry_after_ms: None,
                        },
                        Event::StepComplete {
                            status: StepStatus::Failed,
                            summary: "test failure".to_string(),
                            token_usage: TokenUsage::default(),
                            cost_usd: None,
                            duration_ms: 10,
                        },
                    ]))
                } else {
                    Box::new(ScriptedBackend::new(vec![
                        Event::StepStarted {
                            agent: "test".to_string(),
                            model: ModelId("fake".to_string()),
                        },
                        Event::StepComplete {
                            status: StepStatus::Passed,
                            summary: "ok".to_string(),
                            token_usage: TokenUsage::default(),
                            cost_usd: None,
                            duration_ms: 50,
                        },
                    ]))
                }
            });

        let pipeline = PipelineConfig::builtin_default();
        let pipeline = pipeline.default_pipeline();

        let result = execute_pipeline(
            &db,
            &bus,
            run_id,
            &ws_id,
            &ws_root,
            pipeline,
            "implement feature X",
            None,
            factory,
        )
        .await;

        assert!(result.is_err(), "expected Err when step fails");

        drop(bus);
        orch_handle.await.unwrap();
        pers_handle.await.unwrap();

        let steps_repo = StepRepo::new(&db);
        let steps = steps_repo.list_by_run(run_id).unwrap();

        // Only 2 steps should have been inserted (architect + tdd-developer).
        assert_eq!(steps.len(), 2, "expected 2 steps inserted");

        let step0 = steps.iter().find(|s| s.seq == 0).unwrap();
        let step1 = steps.iter().find(|s| s.seq == 1).unwrap();
        assert_eq!(step0.status, StepStatus::Passed, "step 0 should be Passed");
        assert_eq!(step1.status, StepStatus::Failed, "step 1 should be Failed");

        // Run should be Failed.
        let runs_repo = RunRepo::new(&db);
        let run = runs_repo.get(run_id).unwrap().unwrap();
        assert_eq!(run.status, RunStatus::Failed);
    }

    #[tokio::test]
    async fn execute_pipeline_missing_agent_file_errors_cleanly() {
        // Only architect and tdd-developer agents exist; qa is missing.
        let agent_names = ["architect", "tdd-developer", "reviewer"];
        let (tmp, db, bus, ws_id) = setup_workspace(&agent_names);
        let ws_root = tmp.path().to_path_buf();

        let run_id = "missing-agent-run-01";
        seed_run(&db, run_id, &ws_id);

        let orch_handle = agentic_core::PipelineOrchestrator::spawn(
            bus.clone(),
            RunRepo::new(&db),
            StepRepo::new(&db),
        );
        let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

        // Factory that always returns passing backend (should be called for steps 0-1).
        let factory: BackendFactory<'_> = Box::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            Box::new(ScriptedBackend::new(vec![
                Event::StepStarted {
                    agent: "test".to_string(),
                    model: ModelId("fake".to_string()),
                },
                Event::StepComplete {
                    status: StepStatus::Passed,
                    summary: "ok".to_string(),
                    token_usage: TokenUsage::default(),
                    cost_usd: None,
                    duration_ms: 50,
                },
            ]))
        });

        let pipeline = PipelineConfig::builtin_default();
        let pipeline = pipeline.default_pipeline();

        let result = execute_pipeline(
            &db,
            &bus,
            run_id,
            &ws_id,
            &ws_root,
            pipeline,
            "implement feature X",
            None,
            factory,
        )
        .await;

        drop(bus);
        orch_handle.await.unwrap();
        pers_handle.await.unwrap();

        assert!(result.is_err(), "expected Err for missing agent");
        let err_str = format!("{:?}", result.unwrap_err());
        assert!(
            err_str.to_lowercase().contains("qa"),
            "error should mention 'qa', got: {err_str}"
        );
    }
}
