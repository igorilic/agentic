//! Step T.11.1: issue header strip — spec §4.3.
//!
//! The issue header is a full-width 1-row strip between the title bar and the
//! two-pane body, per the design hand-off layout:
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
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Add multi-tenant rate limiting".into()),
        run_elapsed_secs: 154, // 02:34
        ..Default::default()
    };

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
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Add multi-tenant rate limiting".into()),
        run_elapsed_secs: 154, // 02:34
        ..Default::default()
    };

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
    // We build per-row cell strings and compare char-by-char to find the
    // column index (not byte index) because some cells like '▰' are
    // multi-byte in UTF-8 but occupy a single column in the terminal.
    let found = (0..30u16).find_map(|y| {
        // Collect each cell symbol as its own entry so we can index by column.
        let cells: Vec<String> = (0..80u16)
            .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
            .collect();
        // Build a joined string to search in.
        let row: String = cells.join("");
        row.find(needle).map(|byte_offset| {
            // Convert byte offset to cell (column) index by counting how many
            // cells' bytes come before byte_offset.
            let mut bytes_so_far = 0usize;
            let mut col = 0u16;
            for (i, sym) in cells.iter().enumerate() {
                if bytes_so_far == byte_offset {
                    col = i as u16;
                    break;
                }
                bytes_so_far += sym.len();
            }
            (y, col)
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
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Test issue".into()),
        run_elapsed_secs: 154,
        ..Default::default()
    };

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
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Some title".into()),
        run_elapsed_secs: 0,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let row0 = row_string(&buffer, 0, 80);
    // Title bar row should still show "agentic" (centered title text).
    assert!(
        row0.contains("agentic"),
        "expected title bar 'agentic' still in row 0, got:\n{row0}"
    );
}

// ── F-1: pill_width must use chars().count(), not byte len ────────────────

/// `● running 02:34` is 15 chars (● = 1 char, 3 bytes). With a 80-col
/// terminal the pill must start at column 80 − 15 = 65. The old code
/// used `.len()` which gives 17 (because `●` is 3 UTF-8 bytes), placing
/// the pill start at col 63.
#[test]
fn pill_starts_at_correct_column_using_char_count() {
    let width = 80u16;
    let backend = TestBackend::new(width, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("T".into()),
        run_elapsed_secs: 154, // 02:34
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // Issue header is on row 1. Find the column of 'r' in "running".
    let cells: Vec<String> = (0..width)
        .map(|x| buffer.cell((x, 1)).unwrap().symbol().to_string())
        .collect();
    let row_str: String = cells.join("");

    // "● running 02:34" — 15 chars total.
    // 'r' of "running" is 2 chars into the pill, so expected col = 65 + 2 = 67.
    // But we find "running" in the joined string; byte-offset in joined
    // string equals col offset only when all preceding cells are single bytes.
    // The `●` cell occupies exactly 1 column (col 65) but its symbol is 3
    // UTF-8 bytes, so `row_str.find("running")` gives byte offset 65 + 3 + 1 =
    // 69, not column 67. Instead we search cell-by-cell.
    let pill_text = "● running 02:34";
    let pill_char_count = pill_text.chars().count() as u16; // 15
    let expected_dot_col = width - pill_char_count; // 65

    // The `●` glyph is a wide-ish Unicode char. ratatui writes it into one
    // cell, but the cell symbol string is "●" (3 bytes). So we check
    // cell(expected_dot_col, 1).symbol() == "●".
    let dot_cell = buffer.cell((expected_dot_col, 1)).unwrap();
    assert_eq!(
        dot_cell.symbol(),
        "●",
        "expected '●' at column {expected_dot_col} (width {width} - chars {pill_char_count}), got {:?}",
        dot_cell.symbol()
    );
}

// ── F-2: both issue-header branches must paint HEADER_BG ─────────────────

/// Active-run branch: at least one cell on the issue-header row (y=1)
/// must have bg == HEADER_BG.
#[test]
fn issue_header_active_run_has_header_bg() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("Some title".into()),
        run_elapsed_secs: 0,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let header_bg = agentic_tui::theme::HEADER_BG;
    let any_header_bg = (0..80u16)
        .map(|x| buffer.cell((x, 1)).unwrap())
        .any(|cell| cell.style().bg == Some(header_bg));

    assert!(
        any_header_bg,
        "expected at least one cell on row 1 to have bg=HEADER_BG ({header_bg:?})"
    );
}

/// Blank (no-run) branch: at least one cell on the issue-header row (y=1)
/// must have bg == HEADER_BG.
#[test]
fn issue_header_blank_has_header_bg() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default(); // no run active

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let header_bg = agentic_tui::theme::HEADER_BG;
    let any_header_bg = (0..80u16)
        .map(|x| buffer.cell((x, 1)).unwrap())
        .any(|cell| cell.style().bg == Some(header_bg));

    assert!(
        any_header_bg,
        "expected at least one cell on row 1 to have bg=HEADER_BG ({header_bg:?}) when no run is active"
    );
}

// ── F-3: run-state dot pulses via frame_parity ────────────────────────────

/// When frame_parity is false the dot `●` must be rendered in BLUE
/// (the "on" phase of the pulse).
#[test]
fn pill_dot_is_blue_when_parity_false() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("T".into()),
        run_elapsed_secs: 0,
        frame_parity: false,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // The `●` cell is at col `80 - 15 = 65` on row 1.
    let dot_col = 80u16 - 15;
    let cell = buffer.cell((dot_col, 1)).unwrap();
    assert_eq!(
        cell.symbol(),
        "●",
        "expected '●' at ({dot_col}, 1), got {:?}",
        cell.symbol()
    );
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::BLUE),
        "expected dot to be BLUE when frame_parity=false, got {:?}",
        cell.style().fg
    );
}

/// When frame_parity is true the dot `●` must be rendered in DIM
/// (the "off" phase of the pulse — color dims rather than disappears).
#[test]
fn pill_dot_is_dim_when_parity_true() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        run_label: Some("AGT-204".into()),
        run_title: Some("T".into()),
        run_elapsed_secs: 0,
        frame_parity: true,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let dot_col = 80u16 - 15;
    let cell = buffer.cell((dot_col, 1)).unwrap();
    assert_eq!(
        cell.symbol(),
        "●",
        "expected '●' at ({dot_col}, 1), got {:?}",
        cell.symbol()
    );
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected dot to be DIM when frame_parity=true, got {:?}",
        cell.style().fg
    );
}
