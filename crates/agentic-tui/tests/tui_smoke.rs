//! Step 12.1: smoke test — the first frame rendered by the TUI must
//! include the word "Agentic" so the user knows they've launched the
//! right binary.

use agentic_tui::draw_first_frame;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

#[test]
fn first_frame_contains_agentic_heading() {
    // 80x24 is the canonical default terminal size; small enough that
    // the assertion is unambiguous.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("Terminal::new");

    terminal.draw(draw_first_frame).expect("draw_first_frame");

    // The TestBackend exposes its internal cell buffer; flatten it to a
    // plain string so we can assert on substrings.
    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();

    assert!(
        content.contains("Agentic"),
        "first frame must include the word 'Agentic'; got: {content:?}"
    );
}
