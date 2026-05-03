/// Integration tests for P.2.4 — PipelineOrchestrator permission gate wiring.
///
/// These tests verify that the orchestrator's bus-consuming loop intercepts
/// `Event::ToolUseStart` envelopes and delegates them to the `AsyncGate`,
/// emitting `PermissionResolved` for allowlist/denylist hits and
/// `PermissionRequest` for the prompt path.
///
/// The gate is observational (approach Q3.c): the tool call has already
/// executed by the time we see the `ToolUseStart` envelope. Denylist hits
/// log a `tracing::warn!` advisory.
use std::sync::Arc;
use std::time::Duration;

use agentic_core::permissions::config::{OnTimeout, PermissionRule, PermissionsConfig, PermissionsSettings};
use agentic_core::permissions::gate_async::AsyncGate;
use agentic_core::{
    Db, Event, EventBus, EventEnvelope, Paths, PipelineOrchestrator, RunRepo, StepRepo,
};
use agentic_core::events::{PermissionDecision, PermissionSource};
use rusqlite::params;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

/// Create a minimal in-process test environment: Db, RunRepo, StepRepo, EventBus.
fn test_setup() -> (tempfile::TempDir, Db, RunRepo, StepRepo, EventBus) {
    let tmp = tempfile::tempdir().unwrap();
    let paths = Paths::for_tests(tmp.path());
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).expect("Db::open");
    let runs = RunRepo::new(&db);
    let steps = StepRepo::new(&db);
    // Seed a workspace row required by the FK on runs.
    let conn = db.conn().unwrap();
    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES (?1, 'test', '/tmp/test', 'test', 0, 0)",
        params!["ws-perm-test"],
    )
    .unwrap();
    let bus = EventBus::new();
    (tmp, db, runs, steps, bus)
}

/// Build a `PermissionsConfig` with explicit allow/deny pattern lists.
fn make_config(allow: &[&str], deny: &[&str]) -> PermissionsConfig {
    PermissionsConfig {
        allowlist: allow
            .iter()
            .map(|s| PermissionRule { pattern: s.to_string() })
            .collect(),
        denylist: deny
            .iter()
            .map(|s| PermissionRule { pattern: s.to_string() })
            .collect(),
        settings: PermissionsSettings {
            default_on_timeout: OnTimeout::Deny,
        },
    }
}

/// Spawn an orchestrator with the given gate and return the subscriber + handle.
/// The caller owns the bus; drop it to shut down the orchestrator.
fn spawn_orch(
    bus: &EventBus,
    runs: &RunRepo,
    steps: &StepRepo,
    gate: Arc<AsyncGate>,
) -> tokio::task::JoinHandle<()> {
    PipelineOrchestrator::spawn(bus.clone(), runs.clone(), steps.clone(), gate)
}

/// Collect the next Permission* envelope from `rx`, timing out after 500 ms.
/// Returns `None` on timeout (no event within window) or `Some(event)` on match.
async fn next_permission_event(
    rx: &mut tokio::sync::broadcast::Receiver<EventEnvelope>,
) -> Option<Event> {
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            let env = rx.recv().await.expect("bus closed unexpectedly");
            if matches!(
                &env.event,
                Event::PermissionRequest { .. } | Event::PermissionResolved { .. }
            ) {
                return env.event;
            }
        }
    })
    .await
    .ok()
}

/// Assert no permission-related envelope arrives within 150 ms.
async fn assert_no_permission_event(rx: &mut tokio::sync::broadcast::Receiver<EventEnvelope>) {
    let nothing = tokio::time::timeout(Duration::from_millis(150), async {
        loop {
            let env = rx.recv().await.expect("bus closed");
            if matches!(
                &env.event,
                Event::PermissionRequest { .. } | Event::PermissionResolved { .. }
            ) {
                return true;
            }
        }
    })
    .await;
    assert!(
        nothing.is_err(),
        "expected no permission envelope, but one arrived"
    );
}

// ---------------------------------------------------------------------------
// Test 1: allowlist hit emits PermissionResolved(AllowOnce, AllowlistConfig)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_use_start_with_allowlist_hit_emits_permission_resolved() {
    let (_tmp, _db, runs, steps, bus) = test_setup();

    let gate = Arc::new(AsyncGate::new(
        make_config(&["Read(*)"], &[]),
        bus.clone(),
        Duration::from_secs(60),
        "test-agent".to_string(),
    ));

    let mut rx = bus.subscribe();
    let _handle = spawn_orch(&bus, &runs, &steps, gate);

    bus.publish(EventEnvelope::now(
        "run-allow".to_string(),
        Some("step-1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "tc-1".to_string(),
            tool_name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/x"}),
        },
    ));

    let event = next_permission_event(&mut rx).await.expect(
        "expected PermissionResolved for allowlist hit, but timed out waiting",
    );

    match event {
        Event::PermissionResolved { decision, source, request_id } => {
            assert_eq!(
                decision,
                PermissionDecision::AllowOnce,
                "allowlist hit must produce AllowOnce decision"
            );
            assert_eq!(
                source,
                PermissionSource::AllowlistConfig,
                "allowlist hit must have AllowlistConfig source"
            );
            assert!(!request_id.is_empty(), "request_id must not be empty");
        }
        other => panic!("expected PermissionResolved, got {other:?}"),
    }

    // Verify no PermissionRequest was published (allowlist short-circuits prompt).
    assert_no_permission_event(&mut rx).await;
}

// ---------------------------------------------------------------------------
// Test 2: denylist hit emits PermissionResolved(Deny, DenylistConfig) + warn log
// ---------------------------------------------------------------------------

#[tokio::test]
#[tracing_test::traced_test]
async fn tool_use_start_with_denylist_hit_emits_permission_resolved_deny_plus_warn_log() {
    let (_tmp, _db, runs, steps, bus) = test_setup();

    let gate = Arc::new(AsyncGate::new(
        make_config(&[], &["Bash(rm -rf *)"]),
        bus.clone(),
        Duration::from_secs(60),
        "test-agent".to_string(),
    ));

    let mut rx = bus.subscribe();
    let _handle = spawn_orch(&bus, &runs, &steps, gate);

    bus.publish(EventEnvelope::now(
        "run-deny".to_string(),
        Some("step-2".to_string()),
        Event::ToolUseStart {
            tool_call_id: "tc-2".to_string(),
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "rm -rf foo"}),
        },
    ));

    let event = next_permission_event(&mut rx).await.expect(
        "expected PermissionResolved for denylist hit, but timed out waiting",
    );

    match event {
        Event::PermissionResolved { decision, source, .. } => {
            assert_eq!(
                decision,
                PermissionDecision::Deny,
                "denylist hit must produce Deny decision"
            );
            assert_eq!(
                source,
                PermissionSource::DenylistConfig,
                "denylist hit must have DenylistConfig source"
            );
        }
        other => panic!("expected PermissionResolved(Deny), got {other:?}"),
    }

    // Verify advisory warn log was emitted.
    assert!(
        logs_contain("permission gate denied"),
        "expected warn log containing 'permission gate denied'"
    );
}

// ---------------------------------------------------------------------------
// Test 3: no match emits PermissionRequest (prompt path — stays pending)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_use_start_with_no_match_emits_permission_request() {
    let (_tmp, _db, runs, steps, bus) = test_setup();

    let gate = Arc::new(AsyncGate::new(
        make_config(&[], &[]),
        bus.clone(),
        Duration::from_secs(60), // long timeout — we don't wait for resolution
        "test-agent".to_string(),
    ));

    let mut rx = bus.subscribe();
    let _handle = spawn_orch(&bus, &runs, &steps, gate);

    bus.publish(EventEnvelope::now(
        "run-prompt".to_string(),
        Some("step-3".to_string()),
        Event::ToolUseStart {
            tool_call_id: "tc-3".to_string(),
            tool_name: "CustomTool".to_string(),
            input: serde_json::json!({"x": "y"}),
        },
    ));

    let event = next_permission_event(&mut rx).await.expect(
        "expected PermissionRequest for unmatched tool, but timed out waiting",
    );

    match event {
        Event::PermissionRequest { tool, agent, .. } => {
            assert_eq!(tool, "CustomTool");
            assert_eq!(agent, "test-agent");
        }
        other => panic!("expected PermissionRequest, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Test 4: non-ToolUseStart events pass through without gate interaction
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_tool_use_events_pass_through() {
    let (_tmp, _db, runs, steps, bus) = test_setup();

    let gate = Arc::new(AsyncGate::new(
        make_config(&[], &[]),
        bus.clone(),
        Duration::from_secs(60),
        "test-agent".to_string(),
    ));

    let mut rx = bus.subscribe();
    let _handle = spawn_orch(&bus, &runs, &steps, gate);

    bus.publish(EventEnvelope::now(
        "run-passthrough".to_string(),
        None,
        Event::TextDelta {
            content: "hello from the LLM".to_string(),
        },
    ));

    // The TextDelta should arrive for the subscriber but no permission events.
    // Drain the TextDelta first.
    let text_env = tokio::time::timeout(Duration::from_millis(200), rx.recv())
        .await
        .expect("timed out waiting for TextDelta")
        .expect("bus closed");
    assert!(
        matches!(text_env.event, Event::TextDelta { .. }),
        "expected TextDelta, got: {:?}",
        text_env.event
    );

    // Now assert no permission event follows within the window.
    assert_no_permission_event(&mut rx).await;
}

// ---------------------------------------------------------------------------
// Bonus test: Bash arg extraction uses `command` field, not whole JSON
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bash_arg_extraction_uses_command_field() {
    let (_tmp, _db, runs, steps, bus) = test_setup();

    // Pattern matches the extracted command string, not the full JSON blob.
    let gate = Arc::new(AsyncGate::new(
        make_config(&[], &["Bash(rm -rf *)"]),
        bus.clone(),
        Duration::from_secs(60),
        "test-agent".to_string(),
    ));

    let mut rx = bus.subscribe();
    let _handle = spawn_orch(&bus, &runs, &steps, gate);

    // Input has `command` field — extractor should pull that value.
    bus.publish(EventEnvelope::now(
        "run-bash-arg".to_string(),
        None,
        Event::ToolUseStart {
            tool_call_id: "tc-bash".to_string(),
            tool_name: "Bash".to_string(),
            input: serde_json::json!({"command": "rm -rf node_modules"}),
        },
    ));

    let event = next_permission_event(&mut rx).await.expect(
        "expected PermissionResolved(Deny) for Bash denylist hit, timed out",
    );

    match event {
        Event::PermissionResolved { decision, source, .. } => {
            assert_eq!(
                decision,
                PermissionDecision::Deny,
                "Bash(rm -rf node_modules) must match denylist Bash(rm -rf *) via command field extraction"
            );
            assert_eq!(source, PermissionSource::DenylistConfig);
        }
        other => panic!("expected PermissionResolved(Deny), got {other:?}"),
    }
}
