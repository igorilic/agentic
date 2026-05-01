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

use agentic_tui::app::{AppState, LogEntry, LogLevel, Pane};
use agentic_tui::draw_app;
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
