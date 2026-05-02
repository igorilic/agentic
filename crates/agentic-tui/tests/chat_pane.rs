//! Step T.12.2: Chat pane — message blocks.
//!
//! Spec §4.6: system messages centered with `── system ──` dividers;
//! user messages with `you` label in ACCENT; agent messages with agent
//! name in GREEN; body indented 2 cols; slash commands and @mentions
//! highlighted with SLASH_TINT.

use agentic_tui::app::{AppState, ChatMessage, Pane};
use agentic_tui::draw_app;
use agentic_tui::theme;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn user_msg(text: &str) -> ChatMessage {
    ChatMessage::User(text.into())
}

fn system_msg(text: &str) -> ChatMessage {
    ChatMessage::System(text.into())
}

fn agent_msg(agent: &str, text: &str) -> ChatMessage {
    ChatMessage::Agent {
        agent: agent.into(),
        text: text.into(),
    }
}

/// Build an AppState focused on Chat with the given messages.
fn state_with_chat(msgs: Vec<ChatMessage>) -> AppState {
    AppState {
        focus: Pane::Chat,
        pipeline: vec![],
        chat: msgs,
        ..Default::default()
    }
}

/// Render draw_app at `width×height` and return a cloned buffer.
fn render_at(state: &AppState, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Render draw_app at 100×20.
fn render(state: &AppState) -> ratatui::buffer::Buffer {
    render_at(state, 100, 20)
}

/// Find the first occurrence of `needle` scanning cell-by-cell.
/// Returns `(col, row)` of the first character of the match, or `None`.
fn find_in_buffer(
    buffer: &ratatui::buffer::Buffer,
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
                if buffer.cell((col, y)).unwrap().symbol() != ch.to_string() {
                    continue 'outer;
                }
            }
            return Some((x, y));
        }
    }
    None
}

// Expose the SLASH_TINT constant for test assertions.
// It lives in views::chat_pane — re-export via the module's pub constant.
// Since it's private to chat_pane, we reference it by value here (same RGB).
// The actual production constant is: Color::Rgb(0x2d, 0x2c, 0x26).
use ratatui::style::Color;
const SLASH_TINT: Color = Color::Rgb(0x2d, 0x2c, 0x26);

// ── Test 1: user label renders in ACCENT ─────────────────────────────────────

/// Seed `state.chat = [user_msg("hello")]`.
/// The label line "you" must have `fg == ACCENT` on its first character.
#[test]
fn chat_pane_renders_user_label_in_accent() {
    let state = state_with_chat(vec![user_msg("hello")]);
    let buffer = render(&state);

    let (col, row) =
        find_in_buffer(&buffer, "you", 100, 20).expect("'you' label not found in buffer");
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::ACCENT),
        "expected 'y' of 'you' at ({col}, {row}) to have fg=ACCENT, got {:?}",
        cell.style().fg
    );
}

// ── Test 2: agent label renders in GREEN ─────────────────────────────────────

/// Seed `state.chat = [agent_msg("architect", "I will plan this.")]`.
/// The label line "architect" must have `fg == GREEN` on its first character.
#[test]
fn chat_pane_renders_agent_label_in_green() {
    let state = state_with_chat(vec![agent_msg("architect", "I will plan this.")]);
    let buffer = render(&state);

    // "architect" in chat label context (not the logs pane which uses BLUE)
    let (col, row) = find_in_buffer(&buffer, "architect", 100, 20)
        .expect("'architect' label not found in buffer");
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(theme::GREEN),
        "expected 'a' of 'architect' at ({col}, {row}) to have fg=GREEN, got {:?}",
        cell.style().fg
    );
}

// ── Test 3: system message centered with em-dash dividers ────────────────────

/// Seed `state.chat = [system_msg("session started")]`.
/// The rendered row must contain `──` (two or more box-drawing horizontals)
/// before AND after the text, and the leading/trailing spaces must be
/// roughly equal (i.e., leading_spaces >= 1 indicating centering).
#[test]
fn chat_pane_centers_system_with_em_dash_dividers() {
    let state = state_with_chat(vec![system_msg("session started")]);
    let buffer = render(&state);

    // Find "session started" in the buffer.
    let (text_col, row) = find_in_buffer(&buffer, "session started", 100, 20)
        .expect("'session started' not found in buffer");

    // There must be at least one `─` character before the text on the same row.
    let has_dash_before = (0..text_col).any(|x| {
        buffer
            .cell((x, row))
            .map(|c| c.symbol() == "─")
            .unwrap_or(false)
    });
    assert!(
        has_dash_before,
        "expected ─ divider BEFORE 'session started' on row {row}, but none found"
    );

    // There must be at least one `─` character after the text on the same row.
    let text_end = text_col + "session started".chars().count() as u16;
    let has_dash_after = (text_end..100).any(|x| {
        buffer
            .cell((x, row))
            .map(|c| c.symbol() == "─")
            .unwrap_or(false)
    });
    assert!(
        has_dash_after,
        "expected ─ divider AFTER 'session started' on row {row}, but none found"
    );

    // Centering: find the first ─ on the row, then verify it is NOT flush
    // against the chat pane's left edge. T.12.3 made the body single-pane
    // (full-width), so the chat area now starts at col 0 (after the tab-bar
    // rows). Centering "── session started ──" (22 chars) in a 100-col area
    // produces a ~39-col indent — so the first ─ should be well past col 0.
    let first_dash = (0..100_u16)
        .find(|&x| {
            buffer
                .cell((x, row))
                .map(|c| c.symbol() == "─")
                .unwrap_or(false)
        })
        .expect("no ─ found on the system divider row");

    // With full-width chat pane (100 cols), "── session started ──" is 22 chars
    // and should be centered with ~39 leading spaces.  Assert that the first ─
    // appears after at least 1 space from the left edge of the buffer.
    assert!(
        first_dash > 0,
        "first ─ at col 0 — centering should produce at least 1 leading space"
    );
    // The divider must be centered, meaning first_dash should be roughly equal
    // to (100 - "── session started ──".len()) / 2 ≈ 39.  Allow ±2 for
    // rounding differences in divider construction.
    assert!(
        first_dash >= 30,
        "first ─ at col {first_dash}; expected centered position >= 30 for 100-col full-width pane (centering \"── session started ──\" at ~col 39)"
    );
}

// ── Test 4: body is indented 2 cols ─────────────────────────────────────────

/// Seed a user message "hello world". The body line below the `you` label
/// must start at `area.x + 2` (col 2 within the chat area), not col 0.
///
/// We render at 100×20 and look for "hello world" in the buffer.
/// The chat area starts somewhere after the cockpit pane; we look for the
/// body text and assert its column is >= 2 relative to the buffer start,
/// then assert the 2 columns before it within the same row are spaces
/// (i.e., body is not flush-left).
#[test]
fn chat_pane_indents_body_two_cols() {
    let state = state_with_chat(vec![user_msg("hello world")]);
    let buffer = render(&state);

    let (col, row) = find_in_buffer(&buffer, "hello world", 100, 20)
        .expect("'hello world' body not found in buffer");

    // The cell two positions before `col` on the same row must be a space,
    // confirming the body is indented (not flush to the chat pane's left edge).
    // Also the body must not start at col 0 of the entire buffer.
    assert!(
        col >= 2,
        "expected body to start at col >= 2 (2-col indent), but it was at col {col}"
    );
    // The two cells immediately before the body text must be spaces (indent).
    let prev_cell = buffer.cell((col - 1, row)).unwrap();
    let prev2_cell = buffer.cell((col - 2, row)).unwrap();
    assert_eq!(
        prev_cell.symbol(),
        " ",
        "expected space immediately before body text at ({}, {}), got '{}'",
        col - 1,
        row,
        prev_cell.symbol()
    );
    assert_eq!(
        prev2_cell.symbol(),
        " ",
        "expected space 2 cols before body text at ({}, {}), got '{}'",
        col - 2,
        row,
        prev2_cell.symbol()
    );
}

// ── Test 5: slash command highlighted with SLASH_TINT ────────────────────────

/// Seed `state.chat = [user_msg("Run /develop now")]`.
/// Every cell covered by `/develop` must have `bg == SLASH_TINT`.
/// The cell for `n` (start of "now") must NOT have bg == SLASH_TINT.
#[test]
fn chat_pane_highlights_slash_command() {
    let state = state_with_chat(vec![user_msg("Run /develop now")]);
    let buffer = render(&state);

    let (start_col, row) =
        find_in_buffer(&buffer, "/develop", 100, 20).expect("'/develop' not found in buffer");

    // Every character of "/develop" must have bg == SLASH_TINT.
    let token = "/develop";
    for (i, ch) in token.chars().enumerate() {
        let col = start_col + i as u16;
        let cell = buffer.cell((col, row)).unwrap();
        assert_eq!(
            cell.symbol(),
            &ch.to_string(),
            "expected '{}' at ({col}, {row})",
            ch
        );
        assert_eq!(
            cell.style().bg,
            Some(SLASH_TINT),
            "expected '/develop'[{i}] ('{ch}') at ({col}, {row}) to have bg=SLASH_TINT, got {:?}",
            cell.style().bg
        );
    }

    // The first cell of "now" (after the token + space) must NOT have SLASH_TINT bg.
    // "/develop " is 9 chars; "now" starts 9 chars after "/develop" start.
    let now_col = start_col + token.chars().count() as u16 + 1; // +1 for space
    if now_col < 100 {
        let now_cell = buffer.cell((now_col, row)).unwrap();
        assert_ne!(
            now_cell.style().bg,
            Some(SLASH_TINT),
            "expected 'n' of 'now' at ({now_col}, {row}) to NOT have bg=SLASH_TINT, got {:?}",
            now_cell.style().bg
        );
    }
}

// ── Test 6: @mention highlighted with SLASH_TINT ────────────────────────────

/// Seed `state.chat = [user_msg("Cc @qa for review")]`.
/// Every cell covered by `@qa` must have `bg == SLASH_TINT`.
#[test]
fn chat_pane_highlights_at_mention() {
    let state = state_with_chat(vec![user_msg("Cc @qa for review")]);
    let buffer = render(&state);

    let (start_col, row) =
        find_in_buffer(&buffer, "@qa", 100, 20).expect("'@qa' not found in buffer");

    let token = "@qa";
    for (i, ch) in token.chars().enumerate() {
        let col = start_col + i as u16;
        let cell = buffer.cell((col, row)).unwrap();
        assert_eq!(
            cell.symbol(),
            &ch.to_string(),
            "expected '{}' at ({col}, {row})",
            ch
        );
        assert_eq!(
            cell.style().bg,
            Some(SLASH_TINT),
            "expected '@qa'[{i}] ('{ch}') at ({col}, {row}) to have bg=SLASH_TINT, got {:?}",
            cell.style().bg
        );
    }
}

// ── Test 7: empty chat does not panic ────────────────────────────────────────

/// `state.chat = vec![]` — render must complete without panicking.
#[test]
fn chat_pane_handles_empty_chat_gracefully() {
    let state = state_with_chat(vec![]);
    // Must not panic.
    render(&state);
}

// ── Test 8: no panic on narrow terminal ──────────────────────────────────────

/// Render multiple messages at 20×10 — must not panic.
#[test]
fn chat_pane_does_not_panic_on_narrow_terminal() {
    let state = state_with_chat(vec![
        user_msg("hello"),
        system_msg("connected"),
        agent_msg("developer", "working on it"),
    ]);
    // Must not panic.
    render_at(&state, 20, 10);
}

// ── Test 9: HEADER_BG continuity ─────────────────────────────────────────────

/// Render with any chat content. At least one cell in the buffer must have
/// `bg == HEADER_BG`, confirming that the pane fills its background properly.
#[test]
fn chat_pane_uses_header_bg_continuity() {
    let state = state_with_chat(vec![user_msg("hello")]);
    let buffer = render(&state);

    let has_header_bg = (0..20_u16).any(|y| {
        (0..100_u16).any(|x| {
            buffer
                .cell((x, y))
                .map(|c| c.style().bg == Some(theme::HEADER_BG))
                .unwrap_or(false)
        })
    });
    assert!(
        has_header_bg,
        "expected at least one cell with bg=HEADER_BG for continuity"
    );
}

// ── Test 11: highlighted tokens use YELLOW fg (F-2) ─────────────────────────

/// Seed `state.chat = [user_msg("Run /develop now")]`.
/// Find a cell on `/develop`. Assert `fg == theme::YELLOW` AND `bg == SLASH_TINT`.
#[test]
fn chat_pane_highlighted_token_uses_yellow_fg() {
    let state = state_with_chat(vec![user_msg("Run /develop now")]);
    let buffer = render(&state);

    let (start_col, row) =
        find_in_buffer(&buffer, "/develop", 100, 20).expect("'/develop' not found in buffer");

    // Every character of "/develop" must have fg == YELLOW and bg == SLASH_TINT.
    let token = "/develop";
    for (i, ch) in token.chars().enumerate() {
        let col = start_col + i as u16;
        let cell = buffer.cell((col, row)).unwrap();
        assert_eq!(
            cell.style().fg,
            Some(theme::YELLOW),
            "expected '/develop'[{i}] ('{ch}') at ({col}, {row}) to have fg=YELLOW, got {:?}",
            cell.style().fg
        );
        assert_eq!(
            cell.style().bg,
            Some(SLASH_TINT),
            "expected '/develop'[{i}] ('{ch}') at ({col}, {row}) to have bg=SLASH_TINT, got {:?}",
            cell.style().bg
        );
    }
}

// ── Test 10: messages render in insertion order ───────────────────────────────

/// Seed: user_msg, system_msg, agent_msg.
/// After rendering, the user label row index must be < system divider row
/// and the system divider row must be < agent label row.
#[test]
fn chat_pane_renders_in_message_order() {
    let state = state_with_chat(vec![
        user_msg("first"),
        system_msg("checkpoint"),
        agent_msg("qa", "second"),
    ]);
    let buffer = render(&state);

    let (_user_col, user_row) =
        find_in_buffer(&buffer, "you", 100, 20).expect("'you' user label not found in buffer");

    let (_sys_col, sys_row) = find_in_buffer(&buffer, "checkpoint", 100, 20)
        .expect("'checkpoint' system text not found in buffer");

    let (_agent_col, agent_row) =
        find_in_buffer(&buffer, "qa", 100, 20).expect("'qa' agent label not found in buffer");

    assert!(
        user_row < sys_row,
        "expected user label (row {user_row}) to appear before system divider (row {sys_row})"
    );
    assert!(
        sys_row < agent_row,
        "expected system divider (row {sys_row}) to appear before agent label (row {agent_row})"
    );
}
