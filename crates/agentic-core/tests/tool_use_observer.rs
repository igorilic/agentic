//! Unit tests for [`ToolUseObserver`].
//!
//! Test 1: observer captures an `Edit` ToolUseStart for the right step and
//!         emits a `FileChange` event after finalize.
//! Test 2: observer ignores wrong tool names, wrong step ids, and missing
//!         `file_path` keys.
//! Test 3: observer ignores envelopes from a different run_id.
//! Test 4: observer captures Copilot `create` tool-use and emits FileChange.
//! Test 5: observer captures Copilot `str_replace` tool-use and emits FileChange.
//! Test 6: observer ignores Copilot read-only `view` tool-use.
//! Test 7: observer ignores Copilot `bash` tool-use (shell redirects not supported).

use agentic_core::{Db, Event, EventBus, EventEnvelope, EventPersister, Paths, ToolUseObserver};
use serde_json::json;
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;

/// Yield to the scheduler `n` times to allow spawned tasks to make progress.
/// More deterministic than `tokio::time::sleep` under CI load.
async fn yield_many(n: usize) {
    for _ in 0..n {
        tokio::task::yield_now().await;
    }
}

// ---------------------------------------------------------------------------
// Test 1: captures Edit tool-use and emits FileChange
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_captures_edit_tooluse_and_emits_filechange() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    // Workspace: write foo.txt with initial content.
    let ws_root = base.join("ws");
    std::fs::create_dir_all(&ws_root).unwrap();
    let foo_path = ws_root.join("foo.txt");
    std::fs::write(&foo_path, b"hello\n").unwrap();

    // Paths + DB for EventPersister.
    let paths = Paths::for_tests(base);
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();

    // Diff target.
    let diff_dir = base.join("diffs");
    std::fs::create_dir_all(&diff_dir).unwrap();
    let diff_path = diff_dir.join("file_changes.diff");

    // Set up bus and persister.
    let bus = EventBus::new();
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // Spawn observer for run="r1", step="s1".
    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &bus,
        "r1".to_string(),
        "s1".to_string(),
        ws_root.clone(),
        observer_stop.clone(),
    );

    // Publish ToolUseStart { Edit, foo.txt }.
    bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t1".to_string(),
            tool_name: "Edit".to_string(),
            input: json!({
                "file_path": foo_path.to_string_lossy().as_ref(),
                "old_string": "hello",
                "new_string": "world"
            }),
        },
    ));

    // Give observer task time to process the envelope before mutating disk.
    yield_many(10).await;

    // Simulate Claude's edit: overwrite foo.txt.
    std::fs::write(&foo_path, b"world\n").unwrap();

    // Cancel observer and finalize.
    observer_stop.cancel();
    let report = observer
        .finalize_into(&diff_path, &bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    // Drain bus and wait for persister.
    drop(bus);
    pers_handle.await.unwrap();

    // Assert: changed_paths contains foo.txt.
    assert!(
        report.changed_paths.iter().any(|p| p == &foo_path),
        "changed_paths should include foo.txt; got: {:?}",
        report.changed_paths
    );

    // Assert: diff file exists and contains unified-diff markers.
    assert!(diff_path.exists(), "diff_path should exist after finalize");
    let diff_content = std::fs::read_to_string(&diff_path).unwrap();
    assert!(
        diff_content.contains("---") && diff_content.contains("+++"),
        "diff should have unified-diff headers; got:\n{diff_content}"
    );
    assert!(
        diff_content.contains("+world"),
        "diff should contain +world line; got:\n{diff_content}"
    );

    // Assert: stream_events has a FileChange row with distinct hashes.
    {
        use agentic_core::Event;

        let conn = db.conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stream_events WHERE event_type = 'FileChange'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            count >= 1,
            "stream_events should have at least one FileChange row; got {count}"
        );

        // Payload is MessagePack-encoded Event. Decode and check hashes differ.
        let payload: Vec<u8> = conn
            .query_row(
                "SELECT payload FROM stream_events WHERE event_type = 'FileChange' LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let event: Event = rmp_serde::from_slice(&payload).expect("payload should decode as Event");
        match event {
            Event::FileChange {
                before_hash,
                after_hash,
                ..
            } => {
                assert_ne!(
                    before_hash, after_hash,
                    "before_hash and after_hash should differ"
                );
                assert!(!before_hash.is_empty(), "before_hash should not be empty");
                assert!(!after_hash.is_empty(), "after_hash should not be empty");
            }
            other => panic!("expected FileChange, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Test 2: ignores wrong tool, wrong step, and missing file_path key
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_ignores_non_edit_tools_and_other_steps() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let ws_root = base.join("ws");
    std::fs::create_dir_all(&ws_root).unwrap();
    let bar_path = ws_root.join("bar.txt");
    std::fs::write(&bar_path, b"unchanged\n").unwrap();

    let paths = Paths::for_tests(base);
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();

    let diff_dir = base.join("diffs");
    std::fs::create_dir_all(&diff_dir).unwrap();
    let diff_path = diff_dir.join("file_changes.diff");

    let bus = EventBus::new();
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &bus,
        "r1".to_string(),
        "s1".to_string(),
        ws_root.clone(),
        observer_stop.clone(),
    );

    // Wrong tool name: Bash (should be ignored).
    bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_bash".to_string(),
            tool_name: "Bash".to_string(),
            input: json!({ "file_path": bar_path.to_string_lossy().as_ref() }),
        },
    ));

    // Correct tool name but wrong step id (should be ignored).
    bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s_other".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_other".to_string(),
            tool_name: "Edit".to_string(),
            input: json!({ "file_path": bar_path.to_string_lossy().as_ref() }),
        },
    ));

    // Correct tool + step but missing file_path key (should warn + skip, not crash).
    bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_no_path".to_string(),
            tool_name: "Edit".to_string(),
            input: json!({ "not_file_path": "some_value" }),
        },
    ));

    // Give observer time to process all envelopes.
    yield_many(10).await;

    observer_stop.cancel();
    let report = observer
        .finalize_into(&diff_path, &bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(bus);
    pers_handle.await.unwrap();

    // Assert: no files captured → changed_paths is empty.
    assert!(
        report.changed_paths.is_empty(),
        "changed_paths should be empty when all events are ignored; got: {:?}",
        report.changed_paths
    );

    // Assert: diff file does NOT exist (no changes to write).
    assert!(
        !diff_path.exists(),
        "diff_path should not exist when nothing was captured"
    );

    // Assert: no FileChange rows in stream_events.
    {
        let conn = db.conn().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM stream_events WHERE event_type = 'FileChange'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            count, 0,
            "should be no FileChange rows when all events were ignored"
        );
    }
}

// ---------------------------------------------------------------------------
// Shared setup helper
// ---------------------------------------------------------------------------

struct TestSetup {
    _tmp: TempDir,
    ws_root: std::path::PathBuf,
    diff_path: std::path::PathBuf,
    bus: EventBus,
    db: Db,
}

impl TestSetup {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        let ws_root = base.join("ws");
        std::fs::create_dir_all(&ws_root).unwrap();

        let paths = Paths::for_tests(base);
        paths.ensure_dirs().unwrap();
        let db = Db::open(&paths).unwrap();

        let diff_dir = base.join("diffs");
        std::fs::create_dir_all(&diff_dir).unwrap();
        let diff_path = diff_dir.join("file_changes.diff");

        let bus = EventBus::new();

        TestSetup {
            _tmp: tmp,
            ws_root,
            diff_path,
            bus,
            db,
        }
    }
}

// ---------------------------------------------------------------------------
// Test 3: observer ignores envelopes from a different run_id (F4 guard)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_ignores_envelopes_from_different_run() {
    let tmp = TempDir::new().unwrap();
    let base = tmp.path();

    let ws_root = base.join("ws");
    std::fs::create_dir_all(&ws_root).unwrap();
    let baz_path = ws_root.join("baz.txt");
    std::fs::write(&baz_path, b"original\n").unwrap();

    let paths = Paths::for_tests(base);
    paths.ensure_dirs().unwrap();
    let db = Db::open(&paths).unwrap();

    let diff_dir = base.join("diffs");
    std::fs::create_dir_all(&diff_dir).unwrap();
    let diff_path = diff_dir.join("file_changes.diff");

    let bus = EventBus::new();
    let pers_handle = EventPersister::spawn(bus.subscribe(), db.clone());

    // Observer is for run "r1", step "s1".
    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &bus,
        "r1".to_string(),
        "s1".to_string(),
        ws_root.clone(),
        observer_stop.clone(),
    );

    // Publish ToolUseStart with the correct step_id but a DIFFERENT run_id ("r2").
    // The observer must ignore this envelope.
    bus.publish(EventEnvelope::now(
        "r2".to_string(), // wrong run
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_wrong_run".to_string(),
            tool_name: "Edit".to_string(),
            input: json!({ "file_path": baz_path.to_string_lossy().as_ref() }),
        },
    ));

    // Give observer time to process the envelope.
    yield_many(10).await;

    // Mutate the file — if capture ran, a diff would be produced.
    std::fs::write(&baz_path, b"mutated\n").unwrap();

    observer_stop.cancel();
    let report = observer
        .finalize_into(&diff_path, &bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(bus);
    pers_handle.await.unwrap();

    // Assert: no files captured because the envelope was from run "r2".
    assert!(
        report.changed_paths.is_empty(),
        "changed_paths should be empty when envelope run_id differs; got: {:?}",
        report.changed_paths
    );

    // Assert: no diff file written.
    assert!(
        !diff_path.exists(),
        "diff_path should not exist when envelope was ignored due to run_id mismatch"
    );
}

// ---------------------------------------------------------------------------
// Test 4: observer captures Copilot `create` tool-use and emits FileChange
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_captures_copilot_create_tool_use() {
    let setup = TestSetup::new();
    let foo_path = setup.ws_root.join("foo.txt");
    std::fs::write(&foo_path, b"hello\n").unwrap();

    let pers_handle = EventPersister::spawn(setup.bus.subscribe(), setup.db.clone());

    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &setup.bus,
        "r1".to_string(),
        "s1".to_string(),
        setup.ws_root.clone(),
        observer_stop.clone(),
    );

    // Copilot `create` tool uses `input.path` not `input.file_path`.
    setup.bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_create".to_string(),
            tool_name: "create".to_string(),
            input: json!({
                "path": foo_path.to_string_lossy().as_ref(),
                "file_text": "world"
            }),
        },
    ));

    yield_many(10).await;

    // Simulate Copilot's edit: overwrite foo.txt.
    std::fs::write(&foo_path, b"world\n").unwrap();

    observer_stop.cancel();
    let report = observer
        .finalize_into(&setup.diff_path, &setup.bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(setup.bus);
    pers_handle.await.unwrap();

    assert!(
        report.changed_paths.iter().any(|p| p == &foo_path),
        "changed_paths should include foo.txt for copilot create; got: {:?}",
        report.changed_paths
    );

    // Check FileChange event with distinct hashes.
    let conn = setup.db.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stream_events WHERE event_type = 'FileChange'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 1,
        "stream_events should have at least one FileChange row; got {count}"
    );
}

// ---------------------------------------------------------------------------
// Test 5: observer captures Copilot `str_replace` tool-use and emits FileChange
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_captures_copilot_str_replace_tool_use() {
    let setup = TestSetup::new();
    let bar_path = setup.ws_root.join("bar.txt");
    std::fs::write(&bar_path, b"original content\n").unwrap();

    let pers_handle = EventPersister::spawn(setup.bus.subscribe(), setup.db.clone());

    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &setup.bus,
        "r1".to_string(),
        "s1".to_string(),
        setup.ws_root.clone(),
        observer_stop.clone(),
    );

    // Copilot `str_replace` tool uses `input.path` not `input.file_path`.
    setup.bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_str_replace".to_string(),
            tool_name: "str_replace".to_string(),
            input: json!({
                "path": bar_path.to_string_lossy().as_ref(),
                "old_str": "original content",
                "new_str": "replaced content"
            }),
        },
    ));

    yield_many(10).await;

    // Simulate Copilot's str_replace: overwrite bar.txt.
    std::fs::write(&bar_path, b"replaced content\n").unwrap();

    observer_stop.cancel();
    let report = observer
        .finalize_into(&setup.diff_path, &setup.bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(setup.bus);
    pers_handle.await.unwrap();

    assert!(
        report.changed_paths.iter().any(|p| p == &bar_path),
        "changed_paths should include bar.txt for copilot str_replace; got: {:?}",
        report.changed_paths
    );

    let conn = setup.db.conn().unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM stream_events WHERE event_type = 'FileChange'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 1,
        "stream_events should have at least one FileChange row; got {count}"
    );
}

// ---------------------------------------------------------------------------
// Test 6: observer ignores Copilot read-only `view` tool-use
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_ignores_copilot_view_tool_use() {
    let setup = TestSetup::new();
    let baz_path = setup.ws_root.join("baz.txt");
    std::fs::write(&baz_path, b"some content\n").unwrap();

    let pers_handle = EventPersister::spawn(setup.bus.subscribe(), setup.db.clone());

    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &setup.bus,
        "r1".to_string(),
        "s1".to_string(),
        setup.ws_root.clone(),
        observer_stop.clone(),
    );

    // Copilot `view` tool is read-only — must be ignored.
    setup.bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_view".to_string(),
            tool_name: "view".to_string(),
            input: json!({
                "path": baz_path.to_string_lossy().as_ref()
            }),
        },
    ));

    yield_many(10).await;

    // File is NOT mutated (view is read-only).

    observer_stop.cancel();
    let report = observer
        .finalize_into(&setup.diff_path, &setup.bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(setup.bus);
    pers_handle.await.unwrap();

    // Observer must NOT have captured pre-state for `view` — changed_paths empty.
    assert!(
        report.changed_paths.is_empty(),
        "changed_paths should be empty for copilot view tool; got: {:?}",
        report.changed_paths
    );

    assert!(
        !setup.diff_path.exists(),
        "diff_path should not exist when only view tool was used"
    );
}

// ---------------------------------------------------------------------------
// Test 7: observer ignores Copilot `bash` tool-use (not supported per spec)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn observer_ignores_copilot_bash_tool_use() {
    let setup = TestSetup::new();
    let qux_path = setup.ws_root.join("qux.txt");

    let pers_handle = EventPersister::spawn(setup.bus.subscribe(), setup.db.clone());

    let observer_stop = CancellationToken::new();
    let observer = ToolUseObserver::spawn(
        &setup.bus,
        "r1".to_string(),
        "s1".to_string(),
        setup.ws_root.clone(),
        observer_stop.clone(),
    );

    // Copilot `bash` with shell redirect — intentionally NOT supported.
    setup.bus.publish(EventEnvelope::now(
        "r1".to_string(),
        Some("s1".to_string()),
        Event::ToolUseStart {
            tool_call_id: "t_bash".to_string(),
            tool_name: "bash".to_string(),
            input: json!({
                "command": format!("echo hello > {}", qux_path.to_string_lossy())
            }),
        },
    ));

    yield_many(10).await;

    // Simulate file being created by bash (shell redirect).
    std::fs::write(&qux_path, b"hello\n").unwrap();

    observer_stop.cancel();
    let report = observer
        .finalize_into(&setup.diff_path, &setup.bus.sender(), "r1", "s1")
        .await
        .expect("finalize_into should succeed");

    drop(setup.bus);
    pers_handle.await.unwrap();

    // bash is intentionally not tracked — no FileChange events.
    assert!(
        report.changed_paths.is_empty(),
        "changed_paths should be empty for bash tool (not supported); got: {:?}",
        report.changed_paths
    );

    assert!(
        !setup.diff_path.exists(),
        "diff_path should not exist when only bash tool was used"
    );
}
