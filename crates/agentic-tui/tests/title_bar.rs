//! Step T.10.2: title bar — fixed 1-row band at the top of the frame.
//! Spec §4.2: traffic lights at left, centered text in DIM.

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn renders_three_traffic_light_dots_in_top_row() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer();

    // Top row (y=0): expect ● at cols 0, 2, 4.
    let dot = "●";
    assert_eq!(
        buffer.cell((0, 0)).unwrap().symbol(),
        dot,
        "expected ● at (col=0, row=0)"
    );
    assert_eq!(
        buffer.cell((2, 0)).unwrap().symbol(),
        dot,
        "expected ● at (col=2, row=0)"
    );
    assert_eq!(
        buffer.cell((4, 0)).unwrap().symbol(),
        dot,
        "expected ● at (col=4, row=0)"
    );
}

#[test]
fn renders_centered_title_text_with_dimensions() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer();

    // Concatenate the top row's symbols.
    let row: String = (0..80)
        .map(|x| buffer.cell((x, 0)).unwrap().symbol())
        .collect();

    // Should contain "agentic" and "80×30" somewhere in the row.
    assert!(
        row.contains("agentic"),
        "expected 'agentic' in top row, got: {row:?}"
    );
    assert!(
        row.contains("80×30") || row.contains("80x30"),
        "expected dimensions '80×30' in top row, got: {row:?}"
    );
}

#[test]
fn title_bar_does_not_clobber_lower_rows() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buffer = terminal.backend().buffer();

    // Row 1 should still render normal cockpit/chat content (not just HEADER_BG).
    // Spot-check: not every cell on row 1 is HEADER_BG (0x16, 0x17, 0x1b).
    let header_bg = ratatui::style::Color::Rgb(0x16, 0x17, 0x1b);
    let all_header: bool = (0..80u16)
        .map(|x| buffer.cell((x, 1)).unwrap())
        .all(|cell| cell.bg == header_bg);
    assert!(
        !all_header,
        "row 1 should render below the title bar (not all HEADER_BG)"
    );
}
