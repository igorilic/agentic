//! Step T.13.5: Help overlay toggled by `?`.
//!
//! Spec §4.9: Centered modal, ACCENT border, HEADER_BG fill.
//! `┌── KEYBINDINGS ──┐` + table of key → description.
//! Esc closes it.

use agentic_tui::app::AppState;
use agentic_tui::draw_app;
use agentic_tui::modes::Mode;
use agentic_tui::theme;
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helpers ───────────────────────────────────────────────────────────────────

const WIDTH: u16 = 100;
const HEIGHT: u16 = 30;

fn render(state: &AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

fn render_sized(state: &AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Find the first occurrence of `needle` by scanning cell-by-cell.
/// Returns `(col, row)` of the first character, or `None`.
fn find_str(
    buf: &ratatui::buffer::Buffer,
    needle: &str,
    width: u16,
    height: u16,
) -> Option<(u16, u16)> {
    let chars: Vec<char> = needle.chars().collect();
    if chars.is_empty() {
        return None;
    }
    for y in 0..height {
        'outer: for x in 0..width {
            for (i, ch) in chars.iter().enumerate() {
                let col = x + i as u16;
                if col >= width {
                    continue 'outer;
                }
                if buf.cell((col, y)).map(|c| c.symbol()) != Some(&ch.to_string()) {
                    continue 'outer;
                }
            }
            return Some((x, y));
        }
    }
    None
}

fn buf_contains(buf: &ratatui::buffer::Buffer, needle: &str, width: u16, height: u16) -> bool {
    find_str(buf, needle, width, height).is_some()
}

// ── State-only tests (no render) ──────────────────────────────────────────────

#[test]
fn pressing_question_mark_opens_help() {
    let mut state = AppState {
        help_open: false,
        ..AppState::default()
    };
    assert_eq!(state.mode, Mode::Normal);
    state.handle_key(KeyCode::Char('?'));
    assert!(
        state.help_open,
        "help_open should be true after pressing '?'"
    );
}

#[test]
fn pressing_question_mark_in_command_mode_does_not_open_help() {
    let mut state = AppState {
        mode: Mode::Command {
            buffer: String::new(),
        },
        help_open: false,
        ..AppState::default()
    };
    state.handle_key(KeyCode::Char('?'));
    assert!(
        !state.help_open,
        "help_open must stay false in Command mode"
    );
    // The '?' should be appended to the command buffer instead.
    assert_eq!(
        state.mode,
        Mode::Command {
            buffer: "?".to_string()
        },
        "'?' should append to the command buffer"
    );
}

#[test]
fn pressing_esc_when_help_open_closes_it() {
    let mut state = AppState {
        help_open: true,
        ..AppState::default()
    };
    state.handle_key(KeyCode::Esc);
    assert!(!state.help_open, "help_open should be false after Esc");
}

#[test]
fn pressing_esc_in_command_mode_with_help_closed_still_exits_command() {
    let mut state = AppState {
        mode: Mode::Command {
            buffer: "plan".to_string(),
        },
        help_open: false,
        ..AppState::default()
    };
    state.handle_key(KeyCode::Esc);
    assert_eq!(
        state.mode,
        Mode::Normal,
        "Esc with help closed must exit command mode"
    );
    assert!(!state.help_open, "help should remain closed when Esc exits command mode");
}

#[test]
fn pressing_question_mark_while_open_closes_help() {
    let mut state = AppState {
        help_open: true,
        mode: Mode::Normal,
        ..AppState::default()
    };
    state.handle_key(KeyCode::Char('?'));
    assert!(
        !state.help_open,
        "help_open should be false after pressing '?' while help is already open (toggle)"
    );
}

// ── Render tests ─────────────────────────────────────────────────────────────

#[test]
fn help_overlay_renders_keybindings_header_when_open() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render(&state);
    assert!(
        buf_contains(&buf, "KEYBINDINGS", WIDTH, HEIGHT),
        "Buffer should contain 'KEYBINDINGS' when help is open"
    );
}

#[test]
fn help_overlay_does_not_render_when_closed() {
    let state = AppState {
        help_open: false,
        ..AppState::default()
    };
    let buf = render(&state);
    assert!(
        !buf_contains(&buf, "KEYBINDINGS", WIDTH, HEIGHT),
        "Buffer must NOT contain 'KEYBINDINGS' when help is closed"
    );
}

#[test]
fn help_overlay_uses_accent_border() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render(&state);

    // Find a corner character on the border.
    let corner_pos = find_str(&buf, "┌", WIDTH, HEIGHT)
        .or_else(|| find_str(&buf, "┐", WIDTH, HEIGHT))
        .expect("Should find a corner char when help is open");

    let cell = buf
        .cell((corner_pos.0, corner_pos.1))
        .expect("Cell must exist");
    assert_eq!(
        cell.fg,
        theme::ACCENT,
        "Border corner must use ACCENT foreground"
    );
}

#[test]
fn help_overlay_uses_header_bg_fill() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render(&state);

    // Find the KEYBINDINGS header and check HEADER_BG on a cell inside the modal.
    let (kx, ky) =
        find_str(&buf, "KEYBINDINGS", WIDTH, HEIGHT).expect("Must find KEYBINDINGS when help open");

    let cell = buf.cell((kx, ky)).expect("Cell must exist");
    assert_eq!(
        cell.bg,
        theme::HEADER_BG,
        "Text inside modal must use HEADER_BG background"
    );
}

#[test]
fn help_overlay_lists_canonical_keybindings() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render(&state);

    for key_label in &["Tab", "1", ":", "?", "y", "Esc"] {
        assert!(
            buf_contains(&buf, key_label, WIDTH, HEIGHT),
            "Buffer must contain canonical keybinding '{}' in help overlay",
            key_label
        );
    }
}

#[test]
fn help_overlay_centered_horizontally() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render(&state);

    let (_kx, ky) =
        find_str(&buf, "KEYBINDINGS", WIDTH, HEIGHT).expect("Must find KEYBINDINGS when help open");

    // Find the leftmost '┌' and '┐' on the KEYBINDINGS row.
    let mut left_col: Option<u16> = None;
    let mut right_col: Option<u16> = None;
    for x in 0..WIDTH {
        if let Some(cell) = buf.cell((x, ky)) {
            if cell.symbol() == "┌" {
                left_col = Some(x);
            }
            if cell.symbol() == "┐" && left_col.is_some() {
                right_col = Some(x);
            }
        }
    }

    let left_col = left_col.expect("Must find '┌' on the KEYBINDINGS row");
    let right_col = right_col.expect("Must find '┐' on the KEYBINDINGS row");
    let modal_width = right_col - left_col + 1;
    let expected_left = (WIDTH - modal_width) / 2;

    // Allow ±3 columns of tolerance for centering.
    let diff = (left_col as i32 - expected_left as i32).unsigned_abs();
    assert!(
        diff <= 3,
        "Modal should be horizontally centered: left_col={left_col}, expected≈{expected_left}"
    );
}

#[test]
fn help_overlay_does_not_panic_on_narrow_terminal() {
    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    // Should not panic — just clip or skip.
    render_sized(&state, 30, 10);
}

#[test]
fn help_overlay_centered_vertically() {
    // MODAL_HEIGHT from views/help_overlay.rs: 2 + 6 + 2 = 10
    // If private, we read it from source; hardcode expected here and document.
    const MODAL_HEIGHT: u16 = 10; // 1 top border + 1 blank + 6 binding rows + 1 blank + 1 bottom border
    const RENDER_HEIGHT: u16 = 40;
    const RENDER_WIDTH: u16 = 100;

    let state = AppState {
        help_open: true,
        ..AppState::default()
    };
    let buf = render_sized(&state, RENDER_WIDTH, RENDER_HEIGHT);

    // Find the '┌' corner — that is the modal's top row.
    let modal_top_row = find_str(&buf, "┌", RENDER_WIDTH, RENDER_HEIGHT)
        .expect("Must find '┌' corner when help is open")
        .1;

    let expected_top = (RENDER_HEIGHT - MODAL_HEIGHT) / 2;
    let diff = (modal_top_row as i32 - expected_top as i32).unsigned_abs();
    assert!(
        diff <= 1,
        "Modal top row {modal_top_row} should be within ±1 of vertically centered row {expected_top}"
    );
}
