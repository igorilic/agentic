//! Step 12.4: command mode (`:plan`, `:status`, `:q`).
//!
//! `Mode` is two-state: `Normal` for the layout/resize keys we already
//! had, and `Command` while the user is typing after `:`. The buffer
//! lives inside the variant rather than a sibling field — that keeps
//! "are we in command mode" and "what's typed so far" inseparable, so
//! we can't accidentally render a prompt for a stale buffer.

/// Pure parse of a command-mode buffer (the text typed *after* the
/// leading `:`). Returns a tri-state so the caller can distinguish a
/// no-op (empty buffer) from a parse failure that should surface a
/// user-facing error in the status line.
pub fn parse_command(buffer: &str) -> ParseResult {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return ParseResult::Empty;
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = match parts.next() {
        Some(c) => c,
        None => return ParseResult::Empty,
    };
    let rest = parts.next().unwrap_or("").trim();
    match cmd {
        "q" | "quit" => ParseResult::Cmd(AppCommand::Quit),
        "status" => ParseResult::Cmd(AppCommand::Status),
        "plan" => {
            if rest.is_empty() {
                ParseResult::Err("Missing argument for :plan: <ticket>".to_string())
            } else {
                ParseResult::Cmd(AppCommand::Plan {
                    ticket: rest.to_string(),
                })
            }
        }
        other => ParseResult::Err(format!("Unknown command: :{other}")),
    }
}

/// What `parse_command` returned: nothing typed, a recognised command,
/// or an error message ready for the chat-pane status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseResult {
    Empty,
    Cmd(AppCommand),
    Err(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Command { buffer: String },
}

/// Commands the user has issued — the binary's main loop turns these
/// into actions (quit, kick off a backend, …). Tests assert on the
/// shape directly without driving the binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    Plan { ticket: String },
    Status,
    Quit,
}
