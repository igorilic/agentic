//! Shared test helpers for TUI integration tests.
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// Flatten the test backend buffer into a single string of cell symbols.
/// Used by integration tests to assert rendered content via substring search.
pub fn flatten(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}
