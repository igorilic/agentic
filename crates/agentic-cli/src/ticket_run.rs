#![deny(unsafe_code)]

use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use agentic_core::{
    Backend, Db, Event, EventBus, EventEnvelope, ExecuteRequest, ModelId, Paths, Pipeline,
    PipelineStep, RunId, RunRepo, RunStatus, Step, StepId, StepRepo, StepStatus, ToolUseObserver,
    WorkspaceRef, discover_agent,
};

/// Injectable factory: given a `PipelineStep`, produce a backend for that step.
pub type BackendFactory<'a> = Box<dyn Fn(&PipelineStep) -> Box<dyn Backend> + Send + Sync + 'a>;

/// All context needed to execute a pipeline run.
///
/// Grouping these into a struct keeps `execute_pipeline` extensible: adding
/// fields like `profile` or `token_budget` in the future is non-breaking.
pub struct PipelineRunContext<'a> {
    pub db: &'a Db,
    pub bus: &'a EventBus,
    pub run_id: &'a str,
    pub ws_id: &'a str,
    pub ws_root: &'a Path,
    pub ticket_text: &'a str,
    pub model_override: Option<ModelId>,
    pub paths: &'a Paths,
}

/// Derive a stable workspace id from the canonical absolute path.
///
/// Uses the first 16 hex chars of a blake3 hash of the canonicalized path,
/// prefixed with `ws-`.  If `canonicalize` fails (e.g. the directory does not
/// exist yet), the raw path bytes are hashed instead — this is a safe fallback
/// for relative or not-yet-created paths.
pub fn stable_workspace_id(ws_root: &Path) -> String {
    let canonical = ws_root
        .canonicalize()
        .unwrap_or_else(|_| ws_root.to_path_buf());
    let hash = blake3::hash(canonical.to_string_lossy().as_bytes());
    let hex = hash.to_hex();
    format!("ws-{}", &hex.as_str()[..16])
}

/// Context for executing a single pipeline step, derived from [`PipelineRunContext`].
struct SingleStepCtx<'a> {
    bus: &'a EventBus,
    run_id: &'a str,
    ws_id: &'a str,
    ws_root: &'a Path,
    ticket_text: &'a str,
    model_override: Option<ModelId>,
    paths: &'a Paths,
    steps: &'a StepRepo,
    run_start: &'a Instant,
}

/// Execute all steps in `pipeline` against `ctx.ticket_text`.
///
/// For each step:
/// 1. Inserts a `Step` row with `status = Pending`.
/// 2. Discovers the agent file under `ctx.ws_root`.
/// 3. Builds an `ExecuteRequest` and calls `backend_factory(step).execute(req, sink)`.
/// 4. If a step fails and `stop_on_failure` is set, returns `Err` immediately.
///
/// After the loop, publishes `RunComplete { status: Completed }`.
pub async fn execute_pipeline<'a>(
    ctx: PipelineRunContext<'a>,
    pipeline: &Pipeline,
    backend_factory: BackendFactory<'_>,
) -> Result<()> {
    let PipelineRunContext {
        db,
        bus,
        run_id,
        ws_id,
        ws_root,
        ticket_text,
        model_override,
        paths,
    } = ctx;
    let runs = RunRepo::new(db);
    let steps = StepRepo::new(db);
    let run_start = Instant::now();

    for (i, pipeline_step) in pipeline.steps.iter().enumerate() {
        let step_ctx = SingleStepCtx {
            bus,
            run_id,
            ws_id,
            ws_root,
            ticket_text,
            model_override: model_override.clone(),
            paths,
            steps: &steps,
            run_start: &run_start,
        };
        execute_single_step(step_ctx, pipeline_step, i, &backend_factory).await?;
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

/// Execute a single pipeline step: insert DB row, spawn observer, call backend,
/// finalize observer, handle stop_on_failure.
async fn execute_single_step<'a>(
    ctx: SingleStepCtx<'a>,
    pipeline_step: &PipelineStep,
    seq: usize,
    backend_factory: &BackendFactory<'_>,
) -> Result<()> {
    let SingleStepCtx {
        bus,
        run_id,
        ws_id,
        ws_root,
        ticket_text,
        model_override,
        paths,
        steps,
        run_start,
    } = ctx;

    let step_id = ulid::Ulid::new().to_string();

    // Insert step row as Pending.
    steps.insert(Step {
        id: step_id.clone(),
        run_id: run_id.to_string(),
        seq: seq as i64,
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

    // Ensure step directory exists and compute diff path.
    let step_dir = paths
        .ensure_step_dir(run_id, seq, &pipeline_step.agent)
        .map_err(|e| {
            anyhow::anyhow!(
                "failed to create step dir for '{}': {}",
                pipeline_step.agent,
                e
            )
        })?;
    let diff_path = step_dir.join("file_changes.diff");

    // Spawn the tool-use observer BEFORE calling the backend.
    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        bus,
        run_id.to_string(),
        step_id.clone(),
        ws_root.to_path_buf(),
        observer_stop.clone(),
    );

    // Discover agent file.
    let agent = match discover_agent(ws_root, &pipeline_step.agent) {
        Ok(a) => a,
        Err(e) => {
            observer_stop.cancel();
            let _ignored = observer
                .finalize_into(&diff_path, &bus.sender(), run_id, &step_id)
                .await;
            return Err(anyhow::anyhow!(
                "agent '{}' not found in workspace '{}': {}",
                pipeline_step.agent,
                ws_root.display(),
                e
            ));
        }
    };

    // Build effective model: CLI override wins, then agent's own default.
    let model = model_override.or_else(|| agent.model.as_deref().map(|m| ModelId(m.to_string())));

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
    let execute_result = backend.execute(req, bus.sender()).await;

    // Stop observer and finalize on every path.
    // Finalize errors are logged but must not mask the backend outcome.
    observer_stop.cancel();
    if let Err(e) = observer
        .finalize_into(&diff_path, &bus.sender(), run_id, &step_id)
        .await
    {
        tracing::warn!(
            run_id = run_id,
            step_id = %step_id,
            error = %e,
            "file snapshot finalize failed"
        );
    }

    let outcome = execute_result
        .map_err(|e| anyhow::anyhow!("backend error for agent '{}': {}", pipeline_step.agent, e))?;

    if outcome.status == StepStatus::Failed && pipeline_step.stop_on_failure {
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
    /// Returns `(tmp, paths, db, bus, ws_id)` — `tmp` must be kept alive for
    /// the duration of the test.
    fn setup_workspace(agent_names: &[&str]) -> (TempDir, Paths, Db, EventBus, String) {
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
        (tmp, paths, db, bus, ws_id)
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
        let (tmp, paths, db, bus, ws_id) = setup_workspace(&agent_names);
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
            PipelineRunContext {
                db: &db,
                bus: &bus,
                run_id,
                ws_id: &ws_id,
                ws_root: &ws_root,
                ticket_text: "implement feature X",
                model_override: None,
                paths: &paths,
            },
            pipeline,
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

        // Assert the persister actually wrote events to stream_events.
        // Each step's ScriptedBackend emits: StepStarted + TextDelta + StepComplete = 3 events.
        // execute_pipeline publishes 1 RunComplete. Total: 4 steps × 3 + 1 = 13.
        let conn = db.conn().unwrap();
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stream_events WHERE run_id = ?1",
                params![run_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            total, 13,
            "expected 13 persisted events (4×3 step events + 1 RunComplete)"
        );

        let count_by_type = |event_type: &str| -> i64 {
            conn.query_row(
                "SELECT COUNT(*) FROM stream_events WHERE run_id = ?1 AND event_type = ?2",
                params![run_id, event_type],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(count_by_type("StepStarted"), 4);
        assert_eq!(count_by_type("TextDelta"), 4);
        assert_eq!(count_by_type("StepComplete"), 4);
        assert_eq!(count_by_type("RunComplete"), 1);
    }

    #[tokio::test]
    async fn execute_pipeline_stops_on_failure_when_step_sets_stop_on_failure() {
        // The default pipeline has stop_on_failure = true for architect (step 0)
        // and tdd-developer (step 1). We'll make step 1 fail.
        let agent_names = ["architect", "tdd-developer", "qa", "reviewer"];
        let (tmp, paths, db, bus, ws_id) = setup_workspace(&agent_names);
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
            PipelineRunContext {
                db: &db,
                bus: &bus,
                run_id,
                ws_id: &ws_id,
                ws_root: &ws_root,
                ticket_text: "implement feature X",
                model_override: None,
                paths: &paths,
            },
            pipeline,
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
        let (tmp, paths, db, bus, ws_id) = setup_workspace(&agent_names);
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
            PipelineRunContext {
                db: &db,
                bus: &bus,
                run_id,
                ws_id: &ws_id,
                ws_root: &ws_root,
                ticket_text: "implement feature X",
                model_override: None,
                paths: &paths,
            },
            pipeline,
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

    #[test]
    fn stable_workspace_id_is_deterministic_for_same_path() {
        let id1 = stable_workspace_id(Path::new("/tmp/foo"));
        let id2 = stable_workspace_id(Path::new("/tmp/foo"));
        assert_eq!(id1, id2, "same path should produce same workspace id");
    }

    #[test]
    fn stable_workspace_id_differs_for_different_paths() {
        let id1 = stable_workspace_id(Path::new("/tmp/foo"));
        let id2 = stable_workspace_id(Path::new("/tmp/bar"));
        assert_ne!(
            id1, id2,
            "different paths should produce different workspace ids"
        );
    }
}
