//! Live E2E smoke test — real `CopilotCliBackend` + `AsyncGate` + sandbox project.
//!
//! This test is `#[ignore]`d and is NOT run by `cargo test` by default.
//! It requires:
//!   - `copilot` (GitHub Copilot CLI) on `PATH` — auth is the CLI's responsibility
//!     (authenticate via `gh auth login` or other credential stores)
//!
//! Run manually:
//!   cargo test -p agentic-cli --test e2e_copilot_live -- --ignored --nocapture
//!
//! See the step G.4 spec for prerequisites and interpretation guide.

use std::sync::Arc;
use std::time::Duration;

use agentic_cli::ticket_run::{BackendFactory, PipelineRunContext, execute_pipeline};
use agentic_core::permissions::config::PermissionsConfig;
use agentic_core::permissions::gate_async::AsyncGate;
use agentic_core::{
    Backend, CopilotCliBackend, Db, Event, EventBus, EventPersister, ModelId, Paths, Pipeline,
    PipelineOrchestrator, PipelineStep, Run, RunRepo, RunStatus, StepRepo, Workspace,
    WorkspaceRepo,
};

// ---------------------------------------------------------------------------
// The smoke test
// ---------------------------------------------------------------------------

#[ignore = "live: requires `copilot` CLI on PATH (auth via gh auth login); run via: cargo test -p agentic-cli --test e2e_copilot_live -- --ignored --nocapture"]
#[tokio::test]
async fn copilot_one_step_pipeline_runs_against_real_copilot_cli() {
    // 1. Skip immediately if the `copilot` CLI is not available.
    //    Auth (GitHub token, subscription via `gh auth login`, or other credential
    //    stores) is the CLI's responsibility — we only check it exists and runs.
    let copilot_status = std::process::Command::new("copilot")
        .arg("--version")
        .output();

    match copilot_status {
        Ok(out) if out.status.success() => {
            // CLI present and runnable — proceed.
        }
        Ok(out) => {
            eprintln!(
                "skipping: `copilot --version` exited {:?}; stderr: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            );
            return;
        }
        Err(e) => {
            eprintln!("skipping: `copilot` CLI not on PATH ({})", e);
            return;
        }
    }

    eprintln!("\n=== G.4 Copilot live smoke test starting ===");

    // 2. Sandbox setup — ephemeral tmpdir, auto-cleaned on drop.
    let dir = tempfile::tempdir().expect("tempdir");
    let sandbox = dir.path();

    eprintln!("sandbox: {}", sandbox.display());

    // Seed a placeholder file so Copilot has something to list.
    std::fs::write(sandbox.join("hello.txt"), "hello from the smoke test\n").unwrap();

    // git init (Copilot requires a git repo for safe-to-write checks, same as Claude).
    for (args, desc) in &[
        (vec!["init", "--quiet"], "git init"),
        (
            vec!["config", "user.email", "test@example.com"],
            "git config email",
        ),
        (vec!["config", "user.name", "Test"], "git config name"),
        (vec!["add", "-A"], "git add"),
        (vec!["commit", "-m", "initial"], "git commit"),
    ] {
        let status = std::process::Command::new("git")
            .current_dir(sandbox)
            .args(args)
            .status()
            .unwrap_or_else(|e| panic!("{desc} failed: {e}"));
        if !status.success() {
            eprintln!("  warning: {desc} exited with {status}");
        }
    }

    // 3. Plant a minimal agent file under .github/agents/ — the Copilot backend
    //    discovery path per G.2.
    let agents_dir = sandbox.join(".github").join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();
    std::fs::write(
        agents_dir.join("reviewer.md"),
        "+++\n\
         name = \"reviewer\"\n\
         description = \"Reviews the workspace and reports findings.\"\n\
         pipeline_role = \"step\"\n\
         +++\n\
         You are a helpful reviewer. List the files present in the current directory \
         using the ls or bash tool, then exit. Be concise.\n",
    )
    .unwrap();

    // 4. Write permissions.toml — allowlist covers Copilot tool names (bash, view, ls).
    //    Denylist rejects destructive shell patterns.  The gate must resolve at least
    //    one call via AllowlistConfig.
    let perm_toml = r#"
[allowlist]
patterns = [
  "bash(*)",
  "Bash(*)",
  "view(*)",
  "ls(*)",
  "Read(*)",
  "LS(*)",
  "Glob(*)",
  "Grep(*)",
]

[denylist]
patterns = [
  "bash(rm -rf /*)",
  "bash(sudo *)",
  "Bash(rm -rf /*)",
  "Bash(sudo *)",
]

[settings]
default_on_timeout = "deny"
"#;
    std::fs::write(sandbox.join("permissions.toml"), perm_toml).unwrap();

    // 5. Bootstrap DB + paths.
    let paths = Paths::for_tests(sandbox);
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();
    let bus = EventBus::new();

    // 6. Seed workspace + run rows (execute_pipeline requires them pre-existing).
    let ws_id = "ws-copilot-smoke";
    let run_id = "run-copilot-smoke";

    WorkspaceRepo::new(&db)
        .insert(Workspace {
            id: ws_id.to_string(),
            name: "copilot-sandbox".to_string(),
            root_path: sandbox.to_string_lossy().to_string(),
            remote_url: None,
            profile: "custom".to_string(),
            created_at: 0,
            last_opened: 0,
        })
        .unwrap();

    RunRepo::new(&db)
        .insert(Run {
            id: run_id.to_string(),
            workspace_id: ws_id.to_string(),
            pipeline_name: "default".to_string(),
            status: RunStatus::Pending,
            ticket_type: None,
            ticket_ref: None,
            ticket_title: None,
            ticket_body: None,
            backend: "copilot-cli".to_string(),
            model: "gpt-4o".to_string(),
            started_at: 0,
            completed_at: None,
            duration_ms: None,
            token_usage: None,
            cost_usd: None,
            summary: None,
            subprocess_pid: None,
        })
        .unwrap();

    // 7. Load permissions config and wire the AsyncGate.
    let perms_config =
        PermissionsConfig::load(&sandbox.join("permissions.toml")).expect("load permissions.toml");
    eprintln!(
        "permissions loaded: {} allowlist, {} denylist patterns",
        perms_config.allowlist.len(),
        perms_config.denylist.len()
    );

    let gate = Arc::new(AsyncGate::new(
        perms_config,
        bus.clone(),
        Duration::from_secs(60),
        "reviewer".to_string(),
    ));

    // 8. Spawn the orchestrator (handles RunStarted → Running transitions) and
    //    event persister.
    let _orch =
        PipelineOrchestrator::spawn(bus.clone(), RunRepo::new(&db), StepRepo::new(&db), gate);
    let _pers = EventPersister::spawn(bus.subscribe(), db.clone());

    // 9. Subscribe to the bus BEFORE launching the pipeline so we don't miss
    //    early envelopes.
    let mut subscriber = bus.subscribe();

    // 10. Build single-step pipeline (reviewer only — fast and exercises
    //     read-only tools which the allowlist covers).
    let pipeline = Pipeline {
        steps: vec![PipelineStep {
            agent: "reviewer".to_string(),
            stop_on_failure: false,
            allowed_questions: None,
        }],
    };

    // 11. Ticket prompt — intentionally minimal to keep cost and runtime low.
    let ticket_text = "List the files in the current directory and exit. \
        Use only read-only tools (ls, view, bash). Do not write or modify any files.";

    // 12. Spawn execute_pipeline in a task so we can concurrently drain the bus.
    let sandbox_path = sandbox.to_path_buf();
    let paths_clone = Paths::for_tests(sandbox);
    let db_ref = db.clone();
    let bus_ref = bus.clone();

    let pipeline_handle = tokio::spawn(async move {
        let factory: BackendFactory<'_> = Box::new(|_step: &PipelineStep| -> Box<dyn Backend> {
            // CopilotCliBackend::from_env() honours COPILOT_CLI_BIN override.
            Box::new(CopilotCliBackend::from_env())
        });

        execute_pipeline(
            PipelineRunContext {
                db: &db_ref,
                bus: &bus_ref,
                run_id,
                ws_id,
                ws_root: &sandbox_path,
                ticket_text,
                model_override: Some(ModelId("gpt-4o".to_string())),
                paths: &paths_clone,
                backend_kind: agentic_core::BackendKind::CopilotCli,
                external_cancel: None,
            },
            &pipeline,
            factory,
        )
        .await
    });

    // 13. Drain bus envelopes. Categorise permission-related events and count
    //     tool calls. Break on RunComplete or outer 3-minute timeout.
    let mut perm_requests: Vec<String> = Vec::new(); // tool name
    let mut perm_resolveds: Vec<(String, String)> = Vec::new(); // (decision, source)
    let mut tool_use_starts: usize = 0;
    let mut step_started_seen = false;
    let mut run_complete_seen = false;

    let drain_result = tokio::time::timeout(Duration::from_secs(180), async {
        loop {
            match subscriber.recv().await {
                Ok(env) => {
                    match &env.event {
                        Event::ToolUseStart { tool_name, .. } => {
                            tool_use_starts += 1;
                            eprintln!("  ToolUseStart: {tool_name}");
                        }
                        Event::PermissionRequest {
                            request_id, tool, ..
                        } => {
                            eprintln!("  PermissionRequest: tool={tool} id={request_id}");
                            perm_requests.push(tool.clone());
                        }
                        Event::PermissionResolved {
                            request_id,
                            decision,
                            source,
                        } => {
                            let d = format!("{decision:?}");
                            let s = format!("{source:?}");
                            eprintln!(
                                "  PermissionResolved: id={request_id} decision={d} source={s}"
                            );
                            perm_resolveds.push((d, s));
                        }
                        Event::RunStarted { ticket, .. } => {
                            eprintln!("  RunStarted: ticket={}", ticket.reference);
                        }
                        Event::StepStarted { agent, .. } => {
                            eprintln!("  StepStarted: agent={agent}");
                            step_started_seen = true;
                        }
                        Event::StepComplete { status, .. } => {
                            eprintln!("  StepComplete: status={status:?}");
                        }
                        Event::Finding {
                            severity, message, ..
                        } => {
                            eprintln!("  Finding: severity={severity:?} message={message}");
                        }
                        Event::RunComplete { status, .. } => {
                            eprintln!("  RunComplete: status={status:?}");
                            run_complete_seen = true;
                            break;
                        }
                        // High-volume / low-signal variants — suppress silently.
                        Event::TextDelta { .. }
                        | Event::ThinkingDelta { .. }
                        | Event::ToolUseDelta { .. }
                        | Event::ToolUseEnd { .. } => {}
                        _ => {
                            // Other variants (FileChange, RetryStarted, Error, etc.)
                            // are uncommon; suppress to avoid noise.
                        }
                    }
                }
                Err(_) => {
                    // Channel closed — pipeline finished and bus was dropped.
                    run_complete_seen = true;
                    break;
                }
            }
        }
    })
    .await;

    // Wait for the pipeline task regardless.
    let pipeline_result = pipeline_handle.await;

    // 14. Summary.
    eprintln!("\n=== SUMMARY ===");
    eprintln!("ToolUseStart envelopes  : {tool_use_starts}");
    eprintln!("StepStarted seen        : {step_started_seen}");
    eprintln!("PermissionRequest count : {}", perm_requests.len());
    eprintln!("PermissionResolved count: {}", perm_resolveds.len());
    eprintln!("RunComplete seen        : {run_complete_seen}");
    eprintln!("Drain timeout           : {}", drain_result.is_err());

    // Print permission details.
    if !perm_resolveds.is_empty() {
        eprintln!("PermissionResolved detail:");
        for (d, s) in &perm_resolveds {
            eprintln!("  decision={d} source={s}");
        }
    }

    // 15. Assertions — flexible; Copilot's exact tool sequence varies.

    // The outer 3-min timeout must not have fired.
    assert!(
        drain_result.is_ok(),
        "smoke test timed out after 180s — Copilot subprocess hung or pipeline stalled"
    );

    // At least one tool call must have been observed on the bus.
    assert!(
        tool_use_starts > 0,
        "expected ≥1 ToolUseStart from Copilot backend; got 0 — \
         check that the bus is wired correctly and CopilotCliBackend emits events"
    );

    // The gate must have resolved at least one permission (allowlist hit for bash/ls/view).
    assert!(
        !perm_resolveds.is_empty(),
        "expected ≥1 PermissionResolved envelope; the gate did not fire — \
         check that AsyncGate is wired into PipelineOrchestrator"
    );

    // At least one resolution should come from AllowlistConfig (bash/ls/view are in the allowlist).
    let allowlist_count = perm_resolveds
        .iter()
        .filter(|(_, s)| s.contains("AllowlistConfig"))
        .count();
    assert!(
        allowlist_count >= 1,
        "expected ≥1 PermissionResolved with source=AllowlistConfig; \
         got {allowlist_count} — allowlist patterns may not be matching"
    );

    // The pipeline must have completed without a panic.
    match pipeline_result {
        Ok(Ok(())) => eprintln!("pipeline completed successfully"),
        Ok(Err(e)) => eprintln!("pipeline returned Err (non-fatal for smoke): {e}"),
        Err(e) => panic!("pipeline task panicked: {e}"),
    }

    eprintln!("\nPASSED");
}
