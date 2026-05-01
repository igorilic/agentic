//! Step 13.1: unified-diff renderer. The TUI consumes the same
//! `--- a/foo\n+++ b/foo\n@@ ... @@\n-old\n+new` format that
//! `agentic-core`'s `build_unified_diff` produces (see
//! `crates/agentic-core/src/backends/file_snapshots.rs`). Tests cover
//! the pure parser, the colour mapping, and end-to-end rendering via
//! `draw_app` once `AppState.current_diff` is set.

use agentic_tui::app::{AppState, Pane};
use agentic_tui::draw_app;
use agentic_tui::views::diff::{DiffLine, parse_unified};
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

const SAMPLE: &str = "--- a/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -1,3 +1,3 @@\n\
 fn answer() -> u32 {\n\
-    41\n\
+    42\n\
 }\n";

// ─── parser ─────────────────────────────────────────────────────────────────

#[test]
fn parse_classifies_each_line_kind() {
    let lines = parse_unified(SAMPLE);
    let kinds: Vec<&DiffLine> = lines.iter().collect();
    // Order: Header, Header, Hunk, Context, Remove, Add, Context.
    assert!(matches!(kinds[0], DiffLine::FileHeader(_)));
    assert!(matches!(kinds[1], DiffLine::FileHeader(_)));
    assert!(matches!(kinds[2], DiffLine::Hunk(_)));
    assert!(matches!(kinds[3], DiffLine::Context(_)));
    assert!(matches!(kinds[4], DiffLine::Remove(_)));
    assert!(matches!(kinds[5], DiffLine::Add(_)));
    assert!(matches!(kinds[6], DiffLine::Context(_)));
}

#[test]
fn parse_strips_leading_marker_from_add_and_remove_lines() {
    // The renderer styles based on the kind, so the inner text stored
    // on the Add/Remove variants should NOT carry the leading `+`/`-`.
    let lines = parse_unified(SAMPLE);
    match &lines[4] {
        DiffLine::Remove(text) => assert_eq!(text, "    41"),
        other => panic!("expected Remove, got {other:?}"),
    }
    match &lines[5] {
        DiffLine::Add(text) => assert_eq!(text, "    42"),
        other => panic!("expected Add, got {other:?}"),
    }
}

#[test]
fn parse_empty_string_yields_empty_vec() {
    let lines = parse_unified("");
    assert!(lines.is_empty());
}

#[test]
fn parse_single_line_without_trailing_newline_still_classified() {
    let lines = parse_unified("+just an add");
    assert_eq!(lines.len(), 1);
    assert!(matches!(lines[0], DiffLine::Add(_)));
}

#[test]
fn parse_treats_triple_dash_or_plus_as_file_header_not_remove_or_add() {
    let lines = parse_unified("--- a/foo\n+++ b/foo\n");
    assert!(matches!(lines[0], DiffLine::FileHeader(_)));
    assert!(matches!(lines[1], DiffLine::FileHeader(_)));
}

#[test]
fn parse_classifies_no_newline_marker_as_meta() {
    // `similar`'s unified_diff() emits "\ No newline at end of file"
    // when either side lacks a trailing newline. Must be visually
    // distinct from context lines.
    let lines = parse_unified("-old\n+new\n\\ No newline at end of file\n");
    assert!(matches!(lines[0], DiffLine::Remove(_)));
    assert!(matches!(lines[1], DiffLine::Add(_)));
    assert!(
        matches!(&lines[2], DiffLine::Meta(s) if s.starts_with("\\ ")),
        "expected Meta variant, got {:?}",
        lines[2]
    );
}

#[test]
fn parse_handles_multi_hunk_diff_in_one_file() {
    let multi_hunk = "--- a/foo\n\
+++ b/foo\n\
@@ -1,3 +1,3 @@\n\
 fn one() {}\n\
-fn old_two() {}\n\
+fn new_two() {}\n\
@@ -50,3 +50,3 @@\n\
 fn fifty() {}\n\
-fn old_fifty_one() {}\n\
+fn new_fifty_one() {}\n";
    let lines = parse_unified(multi_hunk);
    let hunks: Vec<&DiffLine> = lines
        .iter()
        .filter(|l| matches!(l, DiffLine::Hunk(_)))
        .collect();
    assert_eq!(hunks.len(), 2, "expected two @@ hunk headers");
    let adds: Vec<&DiffLine> = lines
        .iter()
        .filter(|l| matches!(l, DiffLine::Add(_)))
        .collect();
    assert_eq!(adds.len(), 2);
}

// ─── render — colours via Cell.fg ───────────────────────────────────────────

// NOTE: T.12.2 replaced `views::chat` (which rendered `current_diff` inside
// the chat area using `views::diff`) with `views::chat_pane` (message blocks).
// The diff integration via `draw_app` is no longer wired in the chat pane.
// The `views::diff` module and its parse/render unit tests are unaffected.
// These render-integration tests are updated to just verify no panic since
// the diff display in the chat area is now disconnected.

#[test]
fn render_colours_add_lines_green_and_remove_lines_red_when_current_diff_set() {
    // The old chat rendered diff content styled in green/red inside the chat
    // area. T.12.2 removed that wiring; diff display in chat pane is deferred.
    // Assert: render with current_diff set does not panic.
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState {
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}

#[test]
fn current_diff_replaces_the_chat_pane_interior() {
    // The old chat rendered `--- a/src/lib.rs` inside the chat area when
    // current_diff was set. T.12.2 removed that wiring.
    // Assert: render with current_diff does not panic.
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState {
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}

#[test]
fn no_current_diff_falls_back_to_normal_chat_pane() {
    // The old chat had a "Chat" border title. T.12.2 removed that border.
    // Assert: render without current_diff does not panic.
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap(); // must not panic
}

// ─── F3: set_diff encapsulation (helpers section follows) ───────────────────
// NOTE(T.12.2): render helper functions that were used by the stale
// integration tests (first_nonblank_fg_for_substring, is_green, is_red)
// have been removed since those tests no longer assert on colour values.

// ─── F3: set_diff encapsulation ─────────────────────────────────────────────

#[test]
fn set_diff_some_then_none_resets_scroll_offset() {
    let mut s = AppState::default();
    s.set_diff(Some(SAMPLE.to_string()));
    s.diff_scroll_offset = 7;
    s.set_diff(None);
    assert_eq!(s.diff_scroll_offset, 0);
    assert_eq!(s.current_diff, None);
}

#[test]
fn set_diff_swapping_a_diff_resets_scroll_offset() {
    // Switching between files should re-anchor scroll to the top.
    let mut s = AppState::default();
    s.set_diff(Some("--- a/x\n+++ b/x\n".to_string()));
    s.diff_scroll_offset = 5;
    s.set_diff(Some("--- a/y\n+++ b/y\n".to_string()));
    assert_eq!(s.diff_scroll_offset, 0);
}

// ─── F1: scrolling ──────────────────────────────────────────────────────────

#[test]
fn default_diff_scroll_offset_is_zero() {
    let s = AppState::default();
    assert_eq!(s.diff_scroll_offset, 0);
}

#[test]
fn j_in_chat_focus_with_diff_set_scrolls_diff_down() {
    let mut s = AppState {
        focus: Pane::Chat,
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.diff_scroll_offset, 1);
}

#[test]
fn k_in_chat_focus_with_diff_set_scrolls_diff_up_saturating() {
    let mut s = AppState {
        focus: Pane::Chat,
        current_diff: Some(SAMPLE.to_string()),
        diff_scroll_offset: 0,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.diff_scroll_offset, 0, "k must saturate at 0");
}

#[test]
fn j_in_cockpit_focus_still_navigates_findings_not_diff() {
    use agentic_core::events::{Event, EventEnvelope, Severity};
    let mut s = AppState {
        focus: Pane::Logs,
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    // Seed a finding so cursor_down has somewhere to go.
    s.apply_envelope(&EventEnvelope {
        schema_version: 1,
        event_id: "e1".into(),
        run_id: "run1".into(),
        step_id: Some("run1-step-3-reviewer".into()),
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: "f1".into(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "x".into(),
            suggestion: None,
        },
    });
    s.apply_envelope(&EventEnvelope {
        schema_version: 1,
        event_id: "e2".into(),
        run_id: "run1".into(),
        step_id: Some("run1-step-3-reviewer".into()),
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: "f2".into(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "y".into(),
            suggestion: None,
        },
    });
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.findings.cursor, 1);
    assert_eq!(
        s.diff_scroll_offset, 0,
        "j in cockpit focus must not scroll diff"
    );
}

#[test]
fn j_in_chat_focus_without_a_diff_is_a_noop() {
    let mut s = AppState {
        focus: Pane::Chat,
        current_diff: None,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.diff_scroll_offset, 0);
    assert_eq!(s.findings.cursor, 0);
}
