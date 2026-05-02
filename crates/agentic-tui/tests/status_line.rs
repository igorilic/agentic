//! Step T.13.3 — Status line with mode indicator (spec §4.8).
//!
//! Bottom row: single row, HEADER_BG.
//! NORMAL:  left = DIM hint `Press : for command · ? for help · 1/2/3 to switch panes · y/s/n on permission`
//!          right = `NORMAL` in DIM.
//! COMMAND: left = `:` in ACCENT bold + buffer (or placeholder hint if empty)
//!          right = `COMMAND` in YELLOW.
//! INSERT:  left = DIM hint (same as NORMAL for T.13.3; chat compose is T.13.6)
//!          right = `INSERT` in GREEN.

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::modes::Mode;
use agentic_tui::theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Collect every symbol in a given row into a single string.
fn row_string(buffer: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    (0..width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Collect all rows into a flat string (for substring searches).
fn buffer_string(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .map(|(x, y)| buffer.cell((x, y)).unwrap().symbol().to_string())
        .collect()
}

/// Find the first occurrence of `needle` in the buffer, scanning row by row.
/// Returns `(col, row)` of the first character of needle, or `None`.
fn find_in_buffer(
    buffer: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let first_char = needle.chars().next()?;
    let first_str = first_char.to_string();

    for y in 0..height {
        for x in 0..width {
            let cell = buffer.cell((x, y)).unwrap();
            if cell.symbol() == first_str {
                // Check the rest of the chars (char-by-char, one cell per char).
                let matches = needle.chars().enumerate().all(|(i, ch)| {
                    let col = x + i as u16;
                    col < width && buffer.cell((col, y)).unwrap().symbol() == ch.to_string()
                });
                if matches {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

// ── Test 1: Normal mode shows DIM hint on the left ───────────────────────────

#[test]
fn status_line_normal_mode_shows_dim_hint_left() {
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Normal,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let height = 24u16;
    let width = 100u16;
    let full = buffer_string(&buffer, width, height);

    assert!(
        full.contains("Press : for command"),
        "expected 'Press : for command' in buffer for Normal mode, got:\n{full}"
    );

    // Find the ':' in "Press : for command" and assert it's DIM.
    let needle = "Press : for command";
    let (col, row) = find_in_buffer(&buffer, needle, width, height)
        .expect("'Press : for command' not found in buffer");

    // The ':' is at offset 6 within the needle ("Press :" = 7 chars, 0-indexed = 6).
    let colon_col = col + 6;
    let colon_cell = buffer.cell((colon_col, row)).unwrap();
    assert_eq!(
        colon_cell.symbol(),
        ":",
        "expected ':' at col {colon_col}, row {row}, got {:?}",
        colon_cell.symbol()
    );
    assert_eq!(
        colon_cell.style().fg,
        Some(theme::DIM),
        "expected ':' of hint to have fg=DIM in Normal mode, got {:?}",
        colon_cell.style().fg
    );
}

// ── Test 2: Normal mode shows DIM NORMAL label right-aligned ─────────────────

#[test]
fn status_line_normal_mode_shows_dim_normal_label_right() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Normal,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // "NORMAL" must appear somewhere in the buffer.
    let full = buffer_string(&buffer, width, height);
    assert!(
        full.contains("NORMAL"),
        "expected 'NORMAL' in buffer for Normal mode, got:\n{full}"
    );

    // Find "NORMAL" and assert fg == DIM.
    let (col, row) =
        find_in_buffer(&buffer, "NORMAL", width, height).expect("'NORMAL' not found in buffer");

    let n_cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        n_cell.style().fg,
        Some(theme::DIM),
        "expected 'N' of NORMAL to have fg=DIM, got {:?}",
        n_cell.style().fg
    );

    // Verify right-aligned: the last char 'L' of "NORMAL" (6 chars) must be
    // within 2 columns of the right edge of the terminal.
    let last_col = col + 5; // 'L' of NORMAL
    let last_cell = buffer.cell((last_col, row)).unwrap();
    assert_eq!(
        last_cell.symbol(),
        "L",
        "expected 'L' at col {last_col}, row {row}, got {:?}",
        last_cell.symbol()
    );
    // Right-aligned: last_col should be at width - 1 or width - 2 (1-cell padding).
    assert!(
        last_col >= width - 2,
        "expected NORMAL label last char at col >= {}, got col {}",
        width - 2,
        last_col
    );
}

// ── Test 3: Command mode shows ACCENT bold ':' on the left ───────────────────

#[test]
fn status_line_command_mode_shows_accent_colon_left() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Command {
            buffer: String::new(),
        },
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // The status line is the bottom row — row height-1.
    let status_row = height - 1;
    let status_row_str = row_string(&buffer, status_row, width);

    // The first non-space cell on the status row should be ':'.
    // Find ':' on that row.
    let colon_col = (0..width)
        .find(|&x| buffer.cell((x, status_row)).unwrap().symbol() == ":")
        .expect(&format!(
            "':' not found on status row {status_row}; row='{status_row_str}'"
        ));

    let colon_cell = buffer.cell((colon_col, status_row)).unwrap();
    assert_eq!(
        colon_cell.style().fg,
        Some(theme::ACCENT),
        "expected ':' to have fg=ACCENT in Command mode, got {:?}",
        colon_cell.style().fg
    );
    assert!(
        colon_cell
            .style()
            .add_modifier
            .contains(Modifier::BOLD),
        "expected ':' to be BOLD in Command mode, modifiers={:?}",
        colon_cell.style().add_modifier
    );
}

// ── Test 4: Command mode shows YELLOW COMMAND label right ────────────────────

#[test]
fn status_line_command_mode_shows_yellow_command_label_right() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Command {
            buffer: String::new(),
        },
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let full = buffer_string(&buffer, width, height);
    assert!(
        full.contains("COMMAND"),
        "expected 'COMMAND' in buffer for Command mode, got:\n{full}"
    );

    let (col, row) =
        find_in_buffer(&buffer, "COMMAND", width, height).expect("'COMMAND' not found in buffer");

    let c_cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        c_cell.style().fg,
        Some(theme::YELLOW),
        "expected 'C' of COMMAND to have fg=YELLOW, got {:?}",
        c_cell.style().fg
    );
}

// ── Test 5: Command mode shows buffer text after ':' ─────────────────────────

#[test]
fn status_line_command_mode_shows_buffer_after_colon() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Command {
            buffer: "plan hello".to_string(),
        },
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // The bottom row should contain ":plan hello".
    let status_row = height - 1;
    let status_row_str = row_string(&buffer, status_row, width);

    assert!(
        status_row_str.contains(":plan hello"),
        "expected ':plan hello' in status row, got:\n{status_row_str}"
    );
}

// ── Test 6: Command mode shows placeholder when buffer empty ─────────────────

#[test]
fn status_line_command_mode_shows_placeholder_when_buffer_empty() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Command {
            buffer: String::new(),
        },
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let status_row = height - 1;
    let status_row_str = row_string(&buffer, status_row, width);

    assert!(
        status_row_str.contains("add"),
        "expected placeholder hint containing 'add' in status row when buffer is empty, got:\n{status_row_str}"
    );
}

// ── Test 7: Insert mode shows GREEN INSERT label right ───────────────────────

#[test]
fn status_line_insert_mode_shows_green_insert_label_right() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Insert,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let full = buffer_string(&buffer, width, height);
    assert!(
        full.contains("INSERT"),
        "expected 'INSERT' in buffer for Insert mode, got:\n{full}"
    );

    let (col, row) =
        find_in_buffer(&buffer, "INSERT", width, height).expect("'INSERT' not found in buffer");

    let i_cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        i_cell.style().fg,
        Some(theme::GREEN),
        "expected 'I' of INSERT to have fg=GREEN, got {:?}",
        i_cell.style().fg
    );
}

// ── Test 8: Status line cells use HEADER_BG ──────────────────────────────────

#[test]
fn status_line_uses_header_bg() {
    let width = 100u16;
    let height = 24u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Normal,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    let status_row = height - 1;
    let any_header_bg = (0..width)
        .map(|x| buffer.cell((x, status_row)).unwrap())
        .any(|cell| cell.style().bg == Some(theme::HEADER_BG));

    assert!(
        any_header_bg,
        "expected at least one cell on the status row (row {status_row}) to have bg=HEADER_BG"
    );
}

// ── Test 9: Narrow terminal does not panic ────────────────────────────────────

#[test]
fn status_line_does_not_panic_on_narrow_terminal() {
    let backend = TestBackend::new(30, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Normal,
        ..Default::default()
    };
    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
}

// ── Test 10: Status line renders at the bottom row ───────────────────────────

#[test]
fn status_line_renders_at_bottom_row() {
    let width = 100u16;
    let height = 30u16;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState {
        mode: Mode::Normal,
        ..Default::default()
    };

    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();

    // "NORMAL" must appear on the last row.
    let (_, label_row) =
        find_in_buffer(&buffer, "NORMAL", width, height).expect("'NORMAL' not found in buffer");

    assert_eq!(
        label_row,
        height - 1,
        "expected NORMAL label on the last row ({}), found on row {}",
        height - 1,
        label_row
    );
}
