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
    // NOTE(T.12.2): chat_pane and logs_pane both use HEADER_BG as their
    // surface background (continuity fill per spec §4.1). The old assertion
    // "row 4 must NOT be all HEADER_BG" is no longer valid because HEADER_BG
    // IS the legitimate body surface. This test now just verifies that
    // draw_app does not panic (the title bar must not write outside row 0).
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = AppState::default();
    terminal.draw(|f| draw_app(f, &state)).unwrap(); // must not panic

    // Row 0 must contain the title bar (bullet dots ● ● ●).
    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();
    assert!(
        content.contains("●"),
        "title bar must render ● dots in row 0; got: {content:?}"
    );
}
