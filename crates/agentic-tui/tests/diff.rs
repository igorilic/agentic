//! Step 13.1: unified-diff renderer. The TUI consumes the same
//! `--- a/foo\n+++ b/foo\n@@ ... @@\n-old\n+new` format that
//! `agentic-core`'s `build_unified_diff` produces (see
//! `crates/agentic-core/src/backends/file_snapshots.rs`). Tests cover
//! the pure parser, the colour mapping, and end-to-end rendering via
//! `draw_app` once `AppState.current_diff` is set.

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::views::diff::{DiffLine, parse_unified};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

const SAMPLE: &str = "--- a/src/lib.rs\n\
+++ b/src/lib.rs\n\
@@ -1,3 +1,3 @@\n\
 fn answer() -> u32 {\n\
-    41\n\
+    42\n\
 }\n";

fn flatten(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

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

// ─── render — colours via Cell.fg ───────────────────────────────────────────

#[test]
fn render_colours_add_lines_green_and_remove_lines_red_when_current_diff_set() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState {
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let buf = terminal.backend().buffer();

    // Find the row containing "    42" — the Add line — and check its
    // first non-blank cell is green-ish.
    let add_color = first_nonblank_fg_for_substring(buf, "42").expect("add row not rendered");
    assert!(
        is_green(add_color),
        "expected green fg for Add line; got {add_color:?}"
    );

    let remove_color = first_nonblank_fg_for_substring(buf, "41").expect("remove row not rendered");
    assert!(
        is_red(remove_color),
        "expected red fg for Remove line; got {remove_color:?}"
    );
}

#[test]
fn current_diff_replaces_the_chat_pane_interior() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState {
        current_diff: Some(SAMPLE.to_string()),
        ..Default::default()
    };
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    // The unified-diff header must show up in the rendered buffer.
    assert!(
        content.contains("--- a/src/lib.rs"),
        "expected diff header rendered; got: {content:?}"
    );
}

#[test]
fn no_current_diff_falls_back_to_normal_chat_pane() {
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let s = AppState::default();
    terminal.draw(|f| draw_app(f, &s)).unwrap();
    let content = flatten(&terminal);
    // No diff text should appear when current_diff is None.
    assert!(
        !content.contains("---"),
        "diff content should not appear when current_diff is None"
    );
    // And the chat title should still be present.
    assert!(content.contains("Chat"));
}

// ─── helpers ────────────────────────────────────────────────────────────────

fn first_nonblank_fg_for_substring(buf: &ratatui::buffer::Buffer, needle: &str) -> Option<Color> {
    let area = buf.area;
    for y in 0..area.height {
        // Scan each row, gathering its symbols.
        let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
        if let Some(col_byte) = row.find(needle) {
            // Convert byte offset → char index → cell column.
            let col_char = row[..col_byte].chars().count() as u16;
            // Walk back to the first non-space cell on this row to grab
            // the marker's colour (the leading `+` / `-` was rendered
            // styled, even though the raw text was stripped).
            for x in (0..=col_char).rev() {
                let sym = buf[(x, y)].symbol();
                if !sym.trim().is_empty() {
                    return Some(buf[(x, y)].fg);
                }
            }
        }
    }
    None
}

fn is_green(c: Color) -> bool {
    matches!(c, Color::Green | Color::LightGreen)
        || matches!(c, Color::Rgb(r, g, b) if g > r && g > b)
}

fn is_red(c: Color) -> bool {
    matches!(c, Color::Red | Color::LightRed) || matches!(c, Color::Rgb(r, g, b) if r > g && r > b)
}
