//! Step T.12.1: Logs pane — column-aligned rows.
//!
//! Spec §4.6: rows of `HH:MM:SS  agent      LEVEL  message`.
//! Columns: 8 (time) / 16 (agent) / 8 (level) / rest (message).
//! Two-space gaps between columns: agent starts at col 10, level at col 28,
//! message at col 38 (relative to the pane's left edge).
//!
//! Colour contracts:
//!   - Time: DIM
//!   - Agent: agent-specific accent (architect=BLUE, developer=GREEN,
//!     qa=PURPLE, reviewer=YELLOW, unknown=DIM)
//!   - Level Info: DIM; Warn: YELLOW; Error: RED; Tool: BLUE
//!   - Message: FG
//!   - Tool call: tool_name BLUE, result DIM
//!
//! GH #100: vertical scroll fields and key handling for the logs pane.

use std::cell::Cell;

use agentic_tui::app::{AppState, LogEntry, LogLevel, Pane};
use agentic_tui::draw_app;
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Render draw_app at 140×40 and return a cloned buffer.
fn render_state(state: &AppState) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, state)).unwrap();
    terminal.backend().buffer().clone()
}

/// Build a default AppState with focus=Logs, no pipeline, and the given log.
fn state_with_log(log: Vec<LogEntry>) -> AppState {
    AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log,
        ..Default::default()
    }
}

/// Find the first occurrence of `needle` by scanning cell-by-cell.
/// Returns (col, row) of the first character or None.
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

// ── Entry constructors ────────────────────────────────────────────────────────

fn info_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:13".to_string(),
        agent: "architect".to_string(),
        level: LogLevel::Info,
        message: "Analyzing ticket".to_string(),
    }
}

fn tool_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:14".to_string(),
        agent: "developer".to_string(),
        level: LogLevel::Tool {
            name: "edit_file".to_string(),
            arg: "src/foo.rs".to_string(),
            result: "ok".to_string(),
        },
        message: String::new(),
    }
}

fn warn_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:15".to_string(),
        agent: "qa".to_string(),
        level: LogLevel::Warn,
        message: "Coverage below threshold".to_string(),
    }
}

fn error_entry() -> LogEntry {
    LogEntry {
        timestamp: "10:42:16".to_string(),
        agent: "reviewer".to_string(),
        level: LogLevel::Error,
        message: "Compilation failed".to_string(),
    }
}

// ── Test 1: time column is DIM ────────────────────────────────────────────────

/// The time field "10:42:13" must be rendered in DIM colour.
#[test]
fn logs_pane_renders_time_in_dim() {
    let buffer = render_state(&state_with_log(vec![info_entry()]));
    let (col, row) =
        find_in_buffer(&buffer, "10:42:13", 140, 40).expect("'10:42:13' not found in buffer");
    // '1' is the first character of the timestamp.
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected time '1' at ({col}, {row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );
}

// ── Test 2: agent column uses agent colour ────────────────────────────────────

/// The agent field "architect" must render in BLUE (architect accent).
#[test]
fn logs_pane_renders_agent_in_agent_color() {
    let buffer = render_state(&state_with_log(vec![info_entry()]));
    let (col, row) =
        find_in_buffer(&buffer, "architect", 140, 40).expect("'architect' not found in buffer");
    // 'a' is the first character of "architect".
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::BLUE),
        "expected agent 'a' at ({col}, {row}) to have fg=BLUE (architect accent), got {:?}",
        cell.style().fg
    );
}

// ── Test 3: level column uses level colour ────────────────────────────────────

/// Info level → DIM, Warn level → YELLOW, Error level → RED.
#[test]
fn logs_pane_renders_level_in_level_color() {
    let buffer = render_state(&state_with_log(vec![
        info_entry(),
        warn_entry(),
        error_entry(),
    ]));

    // Info row: "INFO" → DIM
    let (info_col, info_row) =
        find_in_buffer(&buffer, "INFO", 140, 40).expect("'INFO' not found in buffer");
    let info_cell = buffer.cell((info_col, info_row)).unwrap();
    assert_eq!(
        info_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected 'I' of INFO at ({info_col}, {info_row}) to have fg=DIM, got {:?}",
        info_cell.style().fg
    );

    // Warn row: "WARN" → YELLOW
    let (warn_col, warn_row) =
        find_in_buffer(&buffer, "WARN", 140, 40).expect("'WARN' not found in buffer");
    let warn_cell = buffer.cell((warn_col, warn_row)).unwrap();
    assert_eq!(
        warn_cell.style().fg,
        Some(agentic_tui::theme::YELLOW),
        "expected 'W' of WARN at ({warn_col}, {warn_row}) to have fg=YELLOW, got {:?}",
        warn_cell.style().fg
    );

    // Error row: "ERROR" → RED
    let (err_col, err_row) =
        find_in_buffer(&buffer, "ERROR", 140, 40).expect("'ERROR' not found in buffer");
    let err_cell = buffer.cell((err_col, err_row)).unwrap();
    assert_eq!(
        err_cell.style().fg,
        Some(agentic_tui::theme::RED),
        "expected 'E' of ERROR at ({err_col}, {err_row}) to have fg=RED, got {:?}",
        err_cell.style().fg
    );
}

// ── Test 4: message column is FG ──────────────────────────────────────────────

/// The message text "Analyzing ticket" must render in FG.
#[test]
fn logs_pane_renders_message_in_fg() {
    let buffer = render_state(&state_with_log(vec![info_entry()]));
    let (col, row) =
        find_in_buffer(&buffer, "Analyzing", 140, 40).expect("'Analyzing' not found in buffer");
    // 'A' is the first character.
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::FG),
        "expected 'A' of 'Analyzing' at ({col}, {row}) to have fg=FG, got {:?}",
        cell.style().fg
    );
}

// ── Test 5: tool call — name BLUE, result DIM ─────────────────────────────────

/// Tool entry renders `edit_file("src/foo.rs") → ok` with:
///   - "edit_file" in BLUE
///   - "ok" (after "→ ") in DIM
#[test]
fn logs_pane_renders_tool_call_with_blue_name_and_dim_result() {
    let buffer = render_state(&state_with_log(vec![tool_entry()]));

    // Find "edit_file" → 'e' must be BLUE.
    let (ef_col, ef_row) =
        find_in_buffer(&buffer, "edit_file", 140, 40).expect("'edit_file' not found in buffer");
    let ef_cell = buffer.cell((ef_col, ef_row)).unwrap();
    assert_eq!(
        ef_cell.style().fg,
        Some(agentic_tui::theme::BLUE),
        "expected 'e' of 'edit_file' at ({ef_col}, {ef_row}) to have fg=BLUE, got {:?}",
        ef_cell.style().fg
    );

    // Find " → ok" and check 'o' is DIM.
    // We search for "ok" and then verify the cell. Note: there may be more text
    // around "ok", so we need to find it in the tool row specifically.
    let (ok_col, ok_row) =
        find_in_buffer(&buffer, "→ ok", 140, 40).expect("'→ ok' not found in buffer");
    // '→' is 1 char wide; the next char is ' ', then 'o'. So "→ ok" starts with '→' at ok_col.
    // We want the 'o' of "ok" which is at ok_col + 2 (→ + space).
    let o_col = ok_col + 2;
    let o_cell = buffer.cell((o_col, ok_row)).unwrap();
    assert_eq!(
        o_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected 'o' of 'ok' at ({o_col}, {ok_row}) to have fg=DIM, got {:?}",
        o_cell.style().fg
    );
}

// ── Test 6: columns aligned 8/16/8/rest ──────────────────────────────────────

/// Column boundaries (relative to the pane left edge):
///   - Col 0: time starts (width 8)
///   - Col 10: agent starts (8 + 2-space gap)
///   - Col 28: level starts (10 + 16 + 2-space gap)
///   - Col 38: message starts (28 + 8 + 2-space gap)
///
/// We find the rendered row by locating "10:42:13" and then check that the
/// character at (pane_x + 10) on that row matches 'a' (start of "architect").
#[test]
fn logs_pane_columns_aligned_8_16_8_rest() {
    let buffer = render_state(&state_with_log(vec![info_entry()]));

    // Locate the time field to get the absolute row and pane-left-edge column.
    let (time_col, row) =
        find_in_buffer(&buffer, "10:42:13", 140, 40).expect("'10:42:13' not found in buffer");

    // Time occupies cols [time_col .. time_col+8].
    // Agent should start at time_col + 10 (8 chars + 2-space gap).
    let agent_col = time_col + 10;
    let agent_cell = buffer.cell((agent_col, row)).unwrap();
    assert_eq!(
        agent_cell.symbol(),
        "a",
        "expected 'a' (start of 'architect') at col {agent_col}, row {row}, got {:?}",
        agent_cell.symbol()
    );

    // Level should start at time_col + 10 + 16 + 2 = time_col + 28.
    let level_col = time_col + 28;
    let level_cell = buffer.cell((level_col, row)).unwrap();
    assert_eq!(
        level_cell.symbol(),
        "I",
        "expected 'I' (start of 'INFO') at col {level_col}, row {row}, got {:?}",
        level_cell.symbol()
    );

    // Message should start at time_col + 28 + 8 + 2 = time_col + 38.
    let msg_col = time_col + 38;
    let msg_cell = buffer.cell((msg_col, row)).unwrap();
    assert_eq!(
        msg_cell.symbol(),
        "A",
        "expected 'A' (start of 'Analyzing ticket') at col {msg_col}, row {row}, got {:?}",
        msg_cell.symbol()
    );
}

// ── Test 7: empty log renders without panic ───────────────────────────────────

/// An empty log vec must not panic and must produce a valid buffer.
#[test]
fn logs_pane_handles_empty_log_gracefully() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = state_with_log(vec![]);
    // Must not panic.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
}

// ── Test 8: long messages truncate without overflow ───────────────────────────

/// A message that exceeds available width must not overflow or panic.
#[test]
fn logs_pane_truncates_long_messages_at_area_width() {
    let long_msg = "A".repeat(200);
    let entry = LogEntry {
        timestamp: "10:42:17".to_string(),
        agent: "architect".to_string(),
        level: LogLevel::Info,
        message: long_msg,
    };
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let state = state_with_log(vec![entry]);
    // Must not panic; must not overflow.
    terminal.draw(|f| draw_app(f, &state)).unwrap();
}

// ── Test 9 (F1): tool-call quote chars are FG; parens are DIM ────────────────

/// Per tui-view.jsx:322-326, the entire `"arg"` block (including both quote
/// characters) must be styled FG. Only the outer parentheses `(` and `)` are
/// DIM.
///
/// Layout for `edit_file("src/foo.rs") → ok` at the message column:
///   e d i t _ f i l e (  "  s  r  c  /  f  o  o  .  r  s  "  )  ...
///   0 1 2 3 4 5 6 7 8 9 10 11 ...                             21 22
///
/// `(` is at offset 9 from start of "edit_file", `"` (opening) at offset 10,
/// `"` (closing) at offset 9 + 1 + len("src/foo.rs") + 1 = 21,
/// `)` at offset 22 from start of "edit_file".
#[test]
fn logs_pane_tool_call_quote_chars_are_fg_parens_are_dim() {
    let buffer = render_state(&state_with_log(vec![tool_entry()]));

    // Find the start of "edit_file" in the buffer.
    let (ef_col, ef_row) =
        find_in_buffer(&buffer, "edit_file", 140, 40).expect("'edit_file' not found in buffer");

    // `(` is at ef_col + 9 — must be DIM.
    let open_paren_col = ef_col + 9;
    let open_paren_cell = buffer.cell((open_paren_col, ef_row)).unwrap();
    assert_eq!(
        open_paren_cell.symbol(),
        "(",
        "expected '(' at col {open_paren_col}, row {ef_row}"
    );
    assert_eq!(
        open_paren_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected '(' at ({open_paren_col}, {ef_row}) to have fg=DIM, got {:?}",
        open_paren_cell.style().fg
    );

    // Opening `"` is at ef_col + 10 — must be FG.
    let open_quote_col = ef_col + 10;
    let open_quote_cell = buffer.cell((open_quote_col, ef_row)).unwrap();
    assert_eq!(
        open_quote_cell.symbol(),
        "\"",
        "expected '\"' at col {open_quote_col}, row {ef_row}"
    );
    assert_eq!(
        open_quote_cell.style().fg,
        Some(agentic_tui::theme::FG),
        "expected opening '\"' at ({open_quote_col}, {ef_row}) to have fg=FG, got {:?}",
        open_quote_cell.style().fg
    );

    // Closing `"` is at ef_col + 10 + len("src/foo.rs") + 1 = ef_col + 21 — must be FG.
    // "src/foo.rs" has 10 chars, so: open_quote + 1 (opening ") + 10 (arg) = closing " at +11.
    let arg_len = "src/foo.rs".chars().count() as u16;
    let close_quote_col = open_quote_col + 1 + arg_len;
    let close_quote_cell = buffer.cell((close_quote_col, ef_row)).unwrap();
    assert_eq!(
        close_quote_cell.symbol(),
        "\"",
        "expected closing '\"' at col {close_quote_col}, row {ef_row}"
    );
    assert_eq!(
        close_quote_cell.style().fg,
        Some(agentic_tui::theme::FG),
        "expected closing '\"' at ({close_quote_col}, {ef_row}) to have fg=FG, got {:?}",
        close_quote_cell.style().fg
    );

    // `)` is at ef_col + 10 + 1 + len("src/foo.rs") + 1 = ef_col + 22 — must be DIM.
    let close_paren_col = close_quote_col + 1;
    let close_paren_cell = buffer.cell((close_paren_col, ef_row)).unwrap();
    assert_eq!(
        close_paren_cell.symbol(),
        ")",
        "expected ')' at col {close_paren_col}, row {ef_row}"
    );
    assert_eq!(
        close_paren_cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected ')' at ({close_paren_col}, {ef_row}) to have fg=DIM, got {:?}",
        close_paren_cell.style().fg
    );
}

// ── Test 10 (S3): unknown agent falls back to DIM ─────────────────────────────

/// Any agent name not in the canonical set must render in DIM.
#[test]
fn logs_pane_unknown_agent_falls_back_to_dim() {
    let entry = LogEntry {
        timestamp: "10:42:18".to_string(),
        agent: "unknown-agent".to_string(),
        level: LogLevel::Info,
        message: "hello".to_string(),
    };
    let buffer = render_state(&state_with_log(vec![entry]));

    let (col, row) = find_in_buffer(&buffer, "unknown-agent", 140, 40)
        .expect("'unknown-agent' not found in buffer");
    // 'u' is the first char of "unknown-agent".
    let cell = buffer.cell((col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::DIM),
        "expected 'u' of 'unknown-agent' at ({col}, {row}) to have fg=DIM, got {:?}",
        cell.style().fg
    );
}

// ── Test 11 (S4): each canonical agent renders in its own color ───────────────

/// Per tui-view.jsx:290-292:
///   architect → BLUE, developer → GREEN, qa → PURPLE, reviewer → YELLOW.
#[test]
fn logs_pane_renders_each_agent_in_its_own_color() {
    let entries = vec![
        LogEntry {
            timestamp: "10:42:19".to_string(),
            agent: "architect".to_string(),
            level: LogLevel::Info,
            message: "arch-msg".to_string(),
        },
        LogEntry {
            timestamp: "10:42:20".to_string(),
            agent: "developer".to_string(),
            level: LogLevel::Info,
            message: "dev-msg".to_string(),
        },
        LogEntry {
            timestamp: "10:42:21".to_string(),
            agent: "qa".to_string(),
            level: LogLevel::Info,
            message: "qa-msg".to_string(),
        },
        LogEntry {
            timestamp: "10:42:22".to_string(),
            agent: "reviewer".to_string(),
            level: LogLevel::Info,
            message: "rev-msg".to_string(),
        },
    ];
    let buffer = render_state(&state_with_log(entries));

    // architect → BLUE
    let (_, row) = find_in_buffer(&buffer, "arch-msg", 140, 40).expect("'arch-msg' not found");
    // Find "architect" on the same row: start of agent column is time_col + 10.
    // Use the row from arch-msg and look for 'a' of "architect".
    // We can search for "architect" restricted to that row by checking the buffer directly.
    let arch_col = find_in_buffer(&buffer, "architect", 140, 40)
        .map(|(c, r)| {
            assert_eq!(r, row, "architect row mismatch");
            c
        })
        .expect("'architect' not found");
    let cell = buffer.cell((arch_col, row)).unwrap();
    assert_eq!(
        cell.style().fg,
        Some(agentic_tui::theme::BLUE),
        "architect: expected BLUE at ({arch_col}, {row}), got {:?}",
        cell.style().fg
    );

    // developer → GREEN
    let (_, dev_row) = find_in_buffer(&buffer, "dev-msg", 140, 40).expect("'dev-msg' not found");
    let (dev_col, _) =
        find_in_buffer(&buffer, "developer", 140, 40).expect("'developer' not found");
    let dev_cell = buffer.cell((dev_col, dev_row)).unwrap();
    assert_eq!(
        dev_cell.style().fg,
        Some(agentic_tui::theme::GREEN),
        "developer: expected GREEN at ({dev_col}, {dev_row}), got {:?}",
        dev_cell.style().fg
    );

    // qa → PURPLE
    let (_, qa_row) = find_in_buffer(&buffer, "qa-msg", 140, 40).expect("'qa-msg' not found");
    // Search for "qa" starting at the agent column on qa_row.
    // Since "qa" is short we find it by scanning.
    let qa_col = (0..140_u16)
        .find(|&x| {
            buffer
                .cell((x, qa_row))
                .map(|c| c.symbol() == "q")
                .unwrap_or(false)
                && buffer
                    .cell((x + 1, qa_row))
                    .map(|c| c.symbol() == "a")
                    .unwrap_or(false)
        })
        .expect("'qa' not found in qa row");
    let qa_cell = buffer.cell((qa_col, qa_row)).unwrap();
    assert_eq!(
        qa_cell.style().fg,
        Some(agentic_tui::theme::PURPLE),
        "qa: expected PURPLE at ({qa_col}, {qa_row}), got {:?}",
        qa_cell.style().fg
    );

    // reviewer → YELLOW
    let (_, rev_row) = find_in_buffer(&buffer, "rev-msg", 140, 40).expect("'rev-msg' not found");
    let (rev_col, _) = find_in_buffer(&buffer, "reviewer", 140, 40).expect("'reviewer' not found");
    let rev_cell = buffer.cell((rev_col, rev_row)).unwrap();
    assert_eq!(
        rev_cell.style().fg,
        Some(agentic_tui::theme::YELLOW),
        "reviewer: expected YELLOW at ({rev_col}, {rev_row}), got {:?}",
        rev_cell.style().fg
    );
}

// ── GH #100: Scroll field defaults ───────────────────────────────────────────

/// `AppState::default()` must initialise `log_scroll = 0` and
/// `log_sticky_tail = true` (auto-follow on by default).
#[test]
fn log_scroll_field_default_zero_and_sticky_tail_true() {
    let s = AppState::default();
    assert_eq!(s.log_scroll, 0, "log_scroll must default to 0");
    assert!(s.log_sticky_tail, "log_sticky_tail must default to true");
}

// ── GH #100: j key increments scroll in Logs pane ────────────────────────────

/// Pressing `j` while focus=Logs and Normal mode must increment `log_scroll`
/// by 1 and set `log_sticky_tail = false` (user scrolled away from bottom).
#[test]
fn j_key_in_logs_pane_increments_scroll() {
    let mut s = AppState {
        focus: Pane::Logs,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 0,
        log_sticky_tail: true,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.log_scroll, 1, "j must increment log_scroll by 1");
    assert!(
        !s.log_sticky_tail,
        "j must set log_sticky_tail = false (not at bottom yet)"
    );
}

// ── GH #100: k key decrements scroll, saturating ─────────────────────────────

/// Pressing `k` when `log_scroll = 0` must leave it at 0 (saturating sub).
#[test]
fn k_key_in_logs_pane_decrements_scroll_saturating() {
    let mut s = AppState {
        focus: Pane::Logs,
        log_scroll: 0,
        log_sticky_tail: false,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(
        s.log_scroll, 0,
        "k must not underflow below 0 (saturating sub)"
    );
}

// ── GH #100: k decrements when > 0 ──────────────────────────────────────────

/// Pressing `k` when `log_scroll > 0` must decrement it by 1 and set
/// `log_sticky_tail = false`.
#[test]
fn k_key_in_logs_pane_decrements_scroll_when_nonzero() {
    let mut s = AppState {
        focus: Pane::Logs,
        log_scroll: 3,
        log_sticky_tail: false,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.log_scroll, 2, "k must decrement log_scroll by 1");
    assert!(
        !s.log_sticky_tail,
        "log_sticky_tail must stay false after k"
    );
}

// ── GH #100: j/k do not affect log_scroll when focus = Chat ─────────────────

/// When focus is Chat, `j` must not touch `log_scroll`.
#[test]
fn j_in_chat_pane_does_not_affect_log_scroll() {
    let mut s = AppState {
        focus: Pane::Chat,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 0,
        log_sticky_tail: true,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.log_scroll, 0, "j in Chat pane must not change log_scroll");
}

// ── GH #100: render shows +N earlier indicator when log_scroll > 0 ───────────

/// Build a TestBackend tall enough for 5 rows (4 log rows + 1 indicator).
/// With 10 entries and log_scroll=3, the top row must contain "+3 earlier"
/// and the visible rows must be entries 3..7 (not 0..4).
#[test]
fn render_with_scroll_offset_skips_top_rows_and_shows_indicator() {
    // 10 log entries, each with a unique distinguishable message.
    let log: Vec<LogEntry> = (0..10)
        .map(|i| LogEntry {
            timestamp: "00:00:00".to_string(),
            agent: "architect".to_string(),
            level: LogLevel::Info,
            message: format!("entry-{i:02}"),
        })
        .collect();

    // Terminal: 140 wide, 9 high (5 body rows + 4 chrome rows).
    // draw_app uses rows: 1 title + 1 issue-header + 0 pipeline + 2 tab-bar = 4 chrome.
    // So body height = 9 - 4 - 1(status) = 4 body rows.
    // With log_scroll=3: row 0 = "+3 earlier", rows 1-3 = entries 3,4,5.
    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log,
        log_scroll: 3,
        log_sticky_tail: false,
        ..Default::default()
    };

    let backend = TestBackend::new(140, 9);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // The indicator "+3 earlier" must appear somewhere in the buffer.
    let found_indicator = find_in_buffer(&buf, "+3 earlier", 140, 9).is_some();
    assert!(
        found_indicator,
        "buffer must contain '+3 earlier' when log_scroll=3"
    );

    // entry-00, entry-01, entry-02 must NOT be visible.
    let found_entry_00 = find_in_buffer(&buf, "entry-00", 140, 9).is_some();
    assert!(
        !found_entry_00,
        "entry-00 must not be visible when scrolled past it"
    );

    // entry-03 must be visible (first visible log row after indicator).
    let found_entry_03 = find_in_buffer(&buf, "entry-03", 140, 9).is_some();
    assert!(
        found_entry_03,
        "entry-03 must be visible at scroll offset 3"
    );
}

// ── GH #100: render shows no indicator when log_scroll = 0 ──────────────────

/// When `log_scroll = 0` the "+N earlier" indicator must NOT appear.
#[test]
fn render_with_no_scroll_does_not_show_indicator() {
    let log: Vec<LogEntry> = (0..5)
        .map(|i| LogEntry {
            timestamp: "00:00:00".to_string(),
            agent: "architect".to_string(),
            level: LogLevel::Info,
            message: format!("entry-{i}"),
        })
        .collect();

    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log,
        log_scroll: 0,
        log_sticky_tail: true,
        ..Default::default()
    };

    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect();

    assert!(
        !content.contains("earlier"),
        "no 'earlier' indicator should appear when log_scroll = 0"
    );
}

// ── GH #100: sticky tail keeps scroll at bottom after new entries ─────────────

/// When `log_sticky_tail = true` and more entries exist than fit the visible
/// area, the render must clamp `log_scroll` to keep the bottom row visible.
/// We verify by asserting the last entry (entry-09) is visible after render
/// with sticky tail on and many entries pushed.
#[test]
fn sticky_tail_clamps_scroll_to_keep_bottom_visible() {
    // 20 entries, terminal body height = 4 (9-row terminal, 4 chrome + 1 status = 5 overhead).
    let log: Vec<LogEntry> = (0..20)
        .map(|i| LogEntry {
            timestamp: "00:00:00".to_string(),
            agent: "architect".to_string(),
            level: LogLevel::Info,
            message: format!("line-{i:02}"),
        })
        .collect();

    let state = AppState {
        focus: Pane::Logs,
        pipeline: vec![],
        log,
        log_scroll: 0,         // starts at top
        log_sticky_tail: true, // sticky tail on
        ..Default::default()
    };

    let backend = TestBackend::new(140, 9);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_app(f, &state)).unwrap();
    let buf = terminal.backend().buffer().clone();

    // With sticky tail = true, the last entry ("line-19") must be visible.
    let found_last = find_in_buffer(&buf, "line-19", 140, 9).is_some();
    assert!(
        found_last,
        "sticky_tail=true must keep the last log entry visible after render"
    );
}

// ── GH #100: Down arrow in Logs pane increments scroll ───────────────────────

/// Down arrow in Logs pane behaves identically to `j`.
#[test]
fn down_arrow_in_logs_pane_increments_scroll() {
    let mut s = AppState {
        focus: Pane::Logs,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 0,
        log_sticky_tail: true,
        ..Default::default()
    };
    s.handle_key(KeyCode::Down);
    assert_eq!(s.log_scroll, 1, "Down arrow must increment log_scroll by 1");
}

// ── GH #100: Up arrow in Logs pane decrements scroll ─────────────────────────

/// Up arrow in Logs pane behaves identically to `k`.
#[test]
fn up_arrow_in_logs_pane_decrements_scroll_saturating() {
    let mut s = AppState {
        focus: Pane::Logs,
        log_scroll: 0,
        log_sticky_tail: false,
        ..Default::default()
    };
    s.handle_key(KeyCode::Up);
    assert_eq!(s.log_scroll, 0, "Up arrow must not underflow log_scroll");
}

// ── GH #100 fix-loop 1: k in Chat pane does not affect log_scroll ─────────────

/// Pressing `k` when focus=Chat must not touch `log_scroll` or `log_sticky_tail`.
#[test]
fn k_in_chat_pane_does_not_affect_log_scroll() {
    let mut s = AppState {
        focus: Pane::Chat,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 2,
        log_sticky_tail: false,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('k'));
    assert_eq!(s.log_scroll, 2, "k in Chat pane must not change log_scroll");
    assert!(
        !s.log_sticky_tail,
        "k in Chat pane must not change log_sticky_tail"
    );
}

// ── GH #100 fix-loop 1: j is a no-op on empty log ────────────────────────────

/// Pressing `j` when the log is empty must leave `log_scroll=0` and
/// `log_sticky_tail=true` unchanged.
#[test]
fn j_is_noop_on_empty_log() {
    let mut s = AppState {
        focus: Pane::Logs,
        log: vec![],
        log_scroll: 0,
        log_sticky_tail: true,
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(s.log_scroll, 0, "j on empty log must not change log_scroll");
    assert!(
        s.log_sticky_tail,
        "j on empty log must not disable log_sticky_tail"
    );
}

// ── GH #100 fix-loop 1: j clamps at log.len()-1 ──────────────────────────────

/// Pressing `j` when already at the bottom (log_scroll == log.len()-1) must
/// not increment further, and must re-enable sticky tail.
#[test]
fn j_clamps_at_log_len_minus_one() {
    // 5 entries: valid scroll range is 0..=4.
    let mut s = AppState {
        focus: Pane::Logs,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 4,
        log_sticky_tail: false,
        last_known_log_height: Cell::new(1), // height=1 → max_scroll = 5-1 = 4, at bottom
        ..Default::default()
    };
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(
        s.log_scroll, 4,
        "j at bottom must not increment log_scroll beyond log.len()-1"
    );
}

// ── GH #100 fix-loop 1: scrolling to bottom re-enables sticky tail ────────────

/// After scrolling up (sticky=false), pressing j enough times to reach the
/// bottom must re-enable `log_sticky_tail = true`.
///
/// Setup: 5 entries, last_known_log_height = 2 (so max_scroll = 5 - (2-1) = 4).
/// Start at log_scroll=1 (sticky=false), press j 3 times → log_scroll=4 == max_scroll.
/// Expect: log_sticky_tail becomes true again.
#[test]
fn scrolling_to_bottom_re_enables_sticky_tail() {
    let mut s = AppState {
        focus: Pane::Logs,
        log: (0..5).map(make_log_entry).collect(),
        log_scroll: 1,
        log_sticky_tail: false,
        last_known_log_height: Cell::new(2),
        ..Default::default()
    };
    // Press j three times: 1→2→3→4 (== max_scroll = 5 - (2-1) = 4).
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('j'));
    s.handle_key(KeyCode::Char('j'));
    assert_eq!(
        s.log_scroll, 4,
        "log_scroll should be 4 after three j presses from 1"
    );
    assert!(
        s.log_sticky_tail,
        "reaching the bottom must re-enable log_sticky_tail"
    );
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn make_log_entry(i: usize) -> LogEntry {
    LogEntry {
        timestamp: "00:00:00".to_string(),
        agent: "architect".to_string(),
        level: LogLevel::Info,
        message: format!("msg-{i}"),
    }
}
