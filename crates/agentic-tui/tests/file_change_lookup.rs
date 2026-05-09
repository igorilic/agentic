//! GH #77: `AppState::apply_envelope` must populate `current_diff` by looking
//! up the `diff` BLOB from `file_changes` when an `Event::FileChange` envelope
//! arrives and `state.db` is set.
//!
//! Tests here exercise the state-machine only (no rendering). The lookup is
//! synchronous — `Db` uses rusqlite + r2d2, so no async runtime is needed.

use std::path::PathBuf;
use std::sync::Arc;

use agentic_core::Db;
use agentic_core::events::{CURRENT_SCHEMA_VERSION, Event, EventEnvelope};
use agentic_tui::app::AppState;

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Build a minimal in-memory `Db` and seed the FK chain:
/// workspace → run → step → `file_changes` row with the given `diff` text.
///
/// Returns `(Db, run_id, step_id)` so callers can build matching envelopes.
fn seed_file_change(
    diff_text: &str,
    before_hash: &str,
    after_hash: &str,
) -> (Db, String, String) {
    let db = Db::open_in_memory().expect("open in-memory db");
    let conn = db.conn().unwrap();

    conn.execute(
        "INSERT INTO workspaces (id, name, root_path, profile, created_at, last_opened) \
         VALUES ('ws1', 'test', '/tmp/test', 'github', 100, 100)",
        [],
    )
    .unwrap();

    let run_id = "run-test-001".to_string();
    conn.execute(
        &format!(
            "INSERT INTO runs \
             (id, workspace_id, pipeline_name, status, backend, model, started_at) \
             VALUES ('{run_id}', 'ws1', 'default', 'running', 'claude-code', 'claude-opus-4-7', 200)"
        ),
        [],
    )
    .unwrap();

    let step_id = "step-test-001".to_string();
    conn.execute(
        &format!(
            "INSERT INTO run_steps (id, run_id, seq, agent_name, status) \
             VALUES ('{step_id}', '{run_id}', 1, 'tdd-developer', 'running')"
        ),
        [],
    )
    .unwrap();

    // Insert file_changes row. diff is stored as TEXT/BLOB.
    conn.execute(
        "INSERT INTO file_changes (id, run_id, step_id, path, before_hash, after_hash, diff, created_at) \
         VALUES ('fc1', ?1, ?2, 'src/foo.rs', ?3, ?4, ?5, 300)",
        rusqlite::params![run_id, step_id, before_hash, after_hash, diff_text.as_bytes()],
    )
    .unwrap();

    (db, run_id, step_id)
}

/// Build an `EventEnvelope` carrying `Event::FileChange` with the given fields.
fn file_change_envelope(
    run_id: &str,
    step_id: Option<&str>,
    path: &str,
    before_hash: &str,
    after_hash: &str,
) -> EventEnvelope {
    EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: "evt-fc-001".into(),
        run_id: run_id.into(),
        step_id: step_id.map(Into::into),
        timestamp_ms: 0,
        event: Event::FileChange {
            path: PathBuf::from(path),
            before_hash: before_hash.into(),
            after_hash: after_hash.into(),
        },
    }
}

// ── Test 1: happy path — diff BLOB is looked up and set ───────────────────────

/// Applying a `FileChange` envelope when `state.db` is set and a matching row
/// exists must populate `state.current_diff` with the diff text from the DB.
#[test]
fn apply_file_change_envelope_populates_current_diff_from_db() {
    let diff_text = "--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,3 @@\n-old\n+new";
    let (db, run_id, step_id) = seed_file_change(diff_text, "hash-before", "hash-after");

    let mut state = AppState::default();
    state.db = Some(Arc::new(db));

    let env = file_change_envelope(
        &run_id,
        Some(&step_id),
        "src/foo.rs",
        "hash-before",
        "hash-after",
    );
    state.apply_envelope(&env);

    assert_eq!(
        state.current_diff,
        Some(diff_text.to_string()),
        "current_diff should be populated from the file_changes BLOB"
    );
}

// ── Test 2: no-op when db is None ─────────────────────────────────────────────

/// When `state.db` is `None`, applying a `FileChange` envelope must leave
/// `state.current_diff` unchanged (no panic, no modification).
#[test]
fn apply_file_change_envelope_with_no_db_is_noop() {
    let mut state = AppState::default();
    assert!(state.db.is_none(), "db must default to None");

    let env = file_change_envelope(
        "run-1",
        None,
        "src/bar.rs",
        "hash-before",
        "hash-after",
    );
    state.apply_envelope(&env);

    assert!(
        state.current_diff.is_none(),
        "current_diff should remain None when db is None"
    );
}

// ── Test 3: no matching row leaves current_diff unchanged ─────────────────────

/// When `state.db` is set but no matching `file_changes` row exists,
/// `state.current_diff` must remain unchanged (no panic, no override to None).
#[test]
fn apply_file_change_envelope_with_no_matching_row_leaves_current_diff_unchanged() {
    let diff_text = "sentinel";
    let (db, run_id, step_id) =
        seed_file_change(diff_text, "hash-before", "hash-after-inserted");

    let mut state = AppState::default();
    state.current_diff = Some("existing-diff".to_string());
    state.db = Some(Arc::new(db));

    // Use a *different* after_hash so no row is found.
    let env = file_change_envelope(
        &run_id,
        Some(&step_id),
        "src/foo.rs",
        "hash-before",
        "hash-after-no-match",
    );
    state.apply_envelope(&env);

    assert_eq!(
        state.current_diff,
        Some("existing-diff".to_string()),
        "current_diff should remain unchanged when no matching row exists"
    );
}

// ── Test 4: existing diff is replaced by matching lookup ──────────────────────

/// When `state.current_diff` is already `Some(...)`, a matching `FileChange`
/// envelope must replace it with the new diff text from the DB.
#[test]
fn apply_file_change_envelope_replaces_previous_diff() {
    let new_diff = "--- a/bar.rs\n+++ b/bar.rs\n@@ -0,0 +1 @@\n+added";
    let (db, run_id, step_id) = seed_file_change(new_diff, "bh", "ah");

    let mut state = AppState::default();
    state.current_diff = Some("old diff text".to_string());
    state.db = Some(Arc::new(db));

    let env = file_change_envelope(&run_id, Some(&step_id), "src/bar.rs", "bh", "ah");
    state.apply_envelope(&env);

    assert_eq!(
        state.current_diff,
        Some(new_diff.to_string()),
        "current_diff should be replaced with the new diff from the DB"
    );
}

// ── Test 5: set_diff resets diff_scroll_offset ────────────────────────────────

/// Regression: when the new lookup path calls `set_diff`, the scroll offset
/// must be reset to 0 (existing `set_diff` invariant preserved).
#[test]
fn set_diff_resets_scroll_offset() {
    let diff_text = "--- a/reset.rs\n+++ b/reset.rs";
    let (db, run_id, step_id) = seed_file_change(diff_text, "bh", "ah");

    let mut state = AppState::default();
    state.diff_scroll_offset = 42; // pre-set to a non-zero value
    state.db = Some(Arc::new(db));

    let env = file_change_envelope(&run_id, Some(&step_id), "src/reset.rs", "bh", "ah");
    state.apply_envelope(&env);

    assert_eq!(
        state.diff_scroll_offset, 0,
        "diff_scroll_offset must be reset to 0 when set_diff is called via FileChange lookup"
    );
    assert_eq!(state.current_diff, Some(diff_text.to_string()));
}

// ── Test 6: db field defaults to None ─────────────────────────────────────────

/// `AppState::default()` must produce `db: None`.
#[test]
fn db_field_default_none() {
    assert!(
        AppState::default().db.is_none(),
        "AppState::default().db must be None"
    );
}
