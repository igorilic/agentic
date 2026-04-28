//! Step 12.4: command mode (`:plan`, `:status`, `:q`).
//!
//! `Mode` is two-state: `Normal` for the layout/resize keys we already
//! had, and `Command` while the user is typing after `:`. The buffer
//! lives inside the variant rather than a sibling field — that keeps
//! "are we in command mode" and "what's typed so far" inseparable, so
//! we can't accidentally render a prompt for a stale buffer.

/// Pure parse of a command-mode buffer (the text typed *after* the
/// leading `:`). Returns `None` for unknown commands and for `:plan`
/// with no ticket text — both cases should silently exit command mode
/// (a future step adds a status-line for user-facing errors).
pub fn parse_command(buffer: &str) -> Option<AppCommand> {
    let trimmed = buffer.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts.next()?;
    let rest = parts.next().unwrap_or("").trim();
    match cmd {
        "q" | "quit" => Some(AppCommand::Quit),
        "status" => Some(AppCommand::Status),
        "plan" => {
            if rest.is_empty() {
                None
            } else {
                Some(AppCommand::Plan {
                    ticket: rest.to_string(),
                })
            }
        }
        _ => None,
    }
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
