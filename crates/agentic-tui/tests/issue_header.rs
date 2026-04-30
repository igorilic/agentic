//! Step T.11.1: issue header strip — spec §4.3.
//!
//! The issue header is a 1-row strip at the top of the chat (right) pane:
//!   `▰ agentic │ AGT-204 Add multi-tenant rate limiting   ● running 02:34`
//!
//! Left side layout (spec §4.3):
//!   • `▰ agentic` — ACCENT bold
//!   • ` │ ` — DIM
//!   • `AGT-204` — FG
//!   • ` <title>` — DIM
//!
//! Right side: `● running 02:34` in BLUE.
//! When no run is active (run_label = None), the row renders blank (no panic).

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Color;

/// Collect every symbol in a given row into a single string.
fn row_string(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    (0..width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Collect every symbol in the entire buffer (all rows) into a single string.
fn buffer_string(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

#[test]
fn renders_issue_id_and_title_in_header() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.run_label = Some("AGT-204".into());
    state.run_title = Some("Add multi-tenant rate limiting".into());
    state.run_elapsed_secs = 154; // 02:34

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 80, 30);

    assert!(
        full.contains("▰ agentic │ AGT-204 Add multi-tenant rate limiting"),
        "expected '▰ agentic │ AGT-204 Add multi-tenant rate limiting' in buffer, got:\n{full}"
    );
}

#[test]
fn renders_running_pill_in_blue() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.run_label = Some("AGT-204".into());
    state.run_title = Some("Add multi-tenant rate limiting".into());
    state.run_elapsed_secs = 154; // 02:34

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 80, 30);

    assert!(
        full.contains("running 02:34"),
        "expected 'running 02:34' in buffer, got:\n{full}"
    );

    // Verify the "running 02:34" text cells are styled in BLUE.
    let blue = agentic_tui::theme::BLUE;
    let needle = "running 02:34";

    // Find which row and column the needle starts in.
    let found = (0..30u16).find_map(|y| {
        let row = row_string(&buffer, y, 80);
        row.find(needle).map(|byte_offset| {
            // Map byte offset → character (cell) offset. Each cell is one char
            // in the test backend, so byte and char index coincide for ASCII.
            // 'running 02:34' is pure ASCII so this is safe.
            (y, byte_offset as u16)
        })
    });

    let (row_y, col_start) = found.expect("'running 02:34' not found in any row");

    // Check that every cell of "running 02:34" has fg == BLUE.
    // The needle is ASCII so char_count == byte_count.
    for col in col_start..col_start + needle.len() as u16 {
        let cell = buffer.cell((col, row_y)).unwrap();
        // ratatui stores Color as the actual colour value; fg is compared directly.
        assert_eq!(
            cell.style().fg,
            Some(blue),
            "expected cell ({col}, {row_y}) = {:?} to have fg=BLUE ({blue:?}), got {:?}",
            cell.symbol(),
            cell.style().fg,
        );
    }
}

#[test]
fn elapsed_formats_as_mm_ss() {
    // 154 seconds = 2 min 34 sec → "02:34"
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.run_label = Some("AGT-204".into());
    state.run_title = Some("Test issue".into());
    state.run_elapsed_secs = 154;

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 80, 30);

    assert!(
        full.contains("02:34"),
        "expected '02:34' in buffer for 154 secs, got:\n{full}"
    );
}

#[test]
fn no_run_renders_blank_without_panic() {
    // When run_label is None, header must not panic and must render blank.
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default(); // run_label = None

    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let full = buffer_string(&buffer, 80, 30);

    // Must not contain any run-specific text.
    assert!(
        !full.contains("running"),
        "expected no 'running' in buffer when no run is active, got:\n{full}"
    );
}

#[test]
fn title_bar_still_at_row_zero() {
    // Issue header must NOT overwrite the title bar in row 0.
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = AppState::default();
    state.run_label = Some("AGT-204".into());
    state.run_title = Some("Some title".into());
    state.run_elapsed_secs = 0;

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row0 = row_string(&buffer, 0, 80);
    // Title bar row should still show "agentic" (centered title text).
    assert!(
        row0.contains("agentic"),
        "expected title bar 'agentic' still in row 0, got:\n{row0}"
    );
}
