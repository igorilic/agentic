//! Application state for the TUI shell.
//!
//! Step 12.2 added focus + resize. Step 12.3 adds a `RunState` that
//! the cockpit pane renders. Bus envelopes flow in via
//! [`AppState::apply_envelope`]; key presses still flow in via
//! [`AppState::handle`]. Both surfaces are pure mutators so the bin
//! and tests share them.

use agentic_core::events::EventEnvelope;
use crossterm::event::KeyCode;

use crate::findings::{FindingsState, Triage};
use crate::modes::{AppCommand, Mode, ParseResult, parse_command};
use crate::run::RunState;

/// Status of a single agent run slot in the pipeline bar (spec §4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunStatus {
    Queued,
    Active,
    Done,
    Failed,
}

/// A single agent card rendered in the pipeline bar.
#[derive(Debug, Clone)]
pub struct AgentInstance {
    /// Zero-padded index + name, e.g. `"01 Architect"`.
    pub label: String,
    pub status: AgentRunStatus,
}

/// A single message in the chat pane (spec §4.6).
///
/// Populated by runner wiring in T.13.x; seeded from tests for T.12.2.
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// Centered divider: `── <text> ──` rendered in DIM.
    System(String),
    /// User message: label `you` in ACCENT, body indented 2 cols.
    User(String),
    /// Agent message: label = agent name in GREEN, body indented 2 cols.
    Agent { agent: String, text: String },
}

/// Level of a log entry, including tool-call variant with structured fields.
///
/// Tool calls carry `name`, `arg`, and `result` separately so the renderer
/// can apply distinct styles (name=BLUE, result=DIM) without string-splitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
    /// Informational row — rendered as "INFO" in DIM.
    Info,
    /// Tool invocation — rendered as `name("arg") → result`.
    Tool {
        name: String,
        arg: String,
        result: String,
    },
    /// Warning row — rendered as "WARN" in YELLOW.
    Warn,
    /// Error row — rendered as "ERROR" in RED.
    Error,
}

/// A single row in the logs pane (spec §4.6).
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// `HH:MM:SS` timestamp string.
    pub timestamp: String,
    /// Agent name, e.g. `"architect"`.
    pub agent: String,
    /// Level and optional tool-call data.
    pub level: LogLevel,
    /// Human-readable message (unused for `LogLevel::Tool`).
    pub message: String,
}

/// Risk level of a pending permission request (spec §4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionRisk {
    Low,
    Medium,
    High,
}

/// A single permission request pending user approval (spec §4.7).
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Agent requesting the permission, e.g. `"developer"`.
    pub agent: String,
    /// Shell command or action requiring approval, e.g. `"rm -rf node_modules"`.
    pub command: String,
    /// Human-readable justification.
    pub reason: String,
    /// Permission scope key, e.g. `"shell.destructive"`.
    pub scope: String,
    /// Risk classification.
    pub risk: PermissionRisk,
}

/// Which pane currently receives input. Pure state — the renderer reads
/// it to decorate the focused pane's title; future steps (12.5 chat) will
/// route key events to the focused pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// Logs / cockpit pane (left side, formerly `Cockpit`).
    Logs,
    Chat,
    /// Issue tracker pane (spec §4.6 issue variant).
    Issue,
}

/// High-level events the bin maps key presses onto. Keeping this an enum
/// (rather than calling state mutators directly) lets us add a TUI test
/// that exercises a sequence without cooking up real `crossterm::Event`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    ToggleFocus,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub focus: Pane,
    /// Pipeline run state — renders as the cockpit stepper.
    pub run: RunState,
    /// Normal vs. Command — see `modes.rs`.
    pub mode: Mode,
    /// Reviewer findings — renders below the stepper, navigated with
    /// `j`/`k`, triaged with `f`/`t`/`i`.
    pub findings: FindingsState,
    /// One-line user-facing status — set when a command parse fails so
    /// the user sees feedback. Cleared when a command succeeds. The
    /// chat pane renders this in place of the hint line.
    pub last_status: Option<String>,
    /// Unified-diff text for the file currently being viewed. When
    /// `Some`, the chat-pane interior is replaced by the diff
    /// renderer. Runtime callers should use `set_diff` to mutate this
    /// (it also resets the scroll offset); tests may write the field
    /// directly when they don't care about scroll state.
    pub current_diff: Option<String>,
    /// Vertical scroll offset for the diff view (number of lines
    /// scrolled past the top). Reset to 0 by `set_diff`.
    pub diff_scroll_offset: u16,
    /// Jira / issue tracker label for the active run, e.g. `"AGT-204"`.
    /// `None` when no run is active (cold-start / idle state).
    pub run_label: Option<String>,
    /// Human-readable issue title for the active run.
    /// `None` when no run is active.
    pub run_title: Option<String>,
    /// Label chips to render in the issue pane (spec §4.6).
    /// Each string is one chip; rendered as `▏<label>▕`.
    pub run_labels: Vec<String>,
    /// Description paragraphs for the issue pane (spec §4.6).
    /// Each entry is one paragraph; rendered with a blank line between.
    pub run_body: Vec<String>,
    /// Acceptance checklist items for the issue pane (spec §4.6).
    /// Each entry is one item; rendered as `[ ] <item>`.
    pub run_acceptance: Vec<String>,
    /// Elapsed wall-clock seconds since the current run started.
    /// Formatted as `MM:SS` in the issue header pill.
    pub run_elapsed_secs: u64,
    /// Toggled by the render loop on every frame to drive the `●` pulse
    /// in the issue-header pill (spec §4.3). `false` = "on" phase (BLUE);
    /// `true` = "off" phase (DIM). Defaults to `false` (start lit).
    pub frame_parity: bool,
    /// Agent pipeline cards rendered in the 4-row pipeline bar (spec §4.4).
    /// When empty, the pipeline bar is not rendered (zero-height constraint).
    pub pipeline: Vec<AgentInstance>,
    /// Log entries rendered in the logs pane (spec §4.6).
    /// Populated by the runner wiring in T.13.x; empty by default.
    pub log: Vec<LogEntry>,
    /// Chat messages rendered in the chat pane (spec §4.6).
    /// Populated by the runner wiring in T.13.x; empty by default.
    pub chat: Vec<ChatMessage>,
    /// Pending permission requests (spec §4.7). When non-empty and the logs
    /// pane is focused, the first entry renders as an inline permission card
    /// after the last log row. Keys y/s/n resolve it (T.13.2).
    pub pending_perms: Vec<PermissionRequest>,
    /// One-line flash message shown in the status line in place of the hint
    /// for ~1.6 s (spec §4.8). Set by perm resolution keys (T.13.2);
    /// the clear timer is a later-phase concern.
    pub flash: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Pane::Logs,
            run: RunState::default(),
            mode: Mode::Normal,
            findings: FindingsState::default(),
            last_status: None,
            current_diff: None,
            diff_scroll_offset: 0,
            run_label: None,
            run_title: None,
            run_labels: vec![],
            run_body: vec![],
            run_acceptance: vec![],
            run_elapsed_secs: 0,
            frame_parity: false,
            pipeline: vec![],
            log: vec![],
            chat: vec![],
            pending_perms: vec![],
            flash: None,
        }
    }
}

impl AppState {
    pub fn handle(&mut self, event: AppEvent) {
        match event {
            AppEvent::ToggleFocus => {
                self.focus = match self.focus {
                    Pane::Logs => Pane::Chat,
                    Pane::Chat => Pane::Issue,
                    Pane::Issue => Pane::Logs,
                };
            }
        }
    }

    /// Forward a bus envelope into both the run-state machine (for
    /// step-status events) and the findings list (for `Event::Finding`).
    /// The bin's main loop will call this for every envelope yielded by
    /// `EventBus::subscribe`.
    pub fn apply_envelope(&mut self, envelope: &EventEnvelope) {
        self.run.apply_envelope(envelope);
        self.findings.ingest(envelope);
    }

    /// Replace the currently-viewed diff text and re-anchor scroll to
    /// the top. Use this from the runtime path so swapping files
    /// can't leave a stale scroll offset visible.
    pub fn set_diff(&mut self, text: Option<String>) {
        self.current_diff = text;
        self.diff_scroll_offset = 0;
    }

    /// Process a key event. The interpretation depends on `self.mode`:
    ///
    /// - `Normal`: `:` enters command mode; `Tab` cycles focus; `1`/`2`/`3`
    ///   jump to logs/chat/issue; `y`/`s`/`n` resolve the first pending perm
    ///   (no-op when `pending_perms` is empty). Unmapped keys are no-ops.
    /// - `Command`: characters append to the buffer; Enter parses and
    ///   may return an `AppCommand`; Esc cancels back to Normal.
    ///
    /// Returns `Some(AppCommand)` when a command should be executed.
    pub fn handle_key(&mut self, key: KeyCode) -> Option<AppCommand> {
        match &mut self.mode {
            Mode::Normal => {
                match key {
                    KeyCode::Char(':') => {
                        self.mode = Mode::Command {
                            buffer: String::new(),
                        };
                    }
                    KeyCode::Tab => self.handle(AppEvent::ToggleFocus),
                    KeyCode::Char('j') => {
                        if self.focus == Pane::Chat && self.current_diff.is_some() {
                            self.diff_scroll_offset = self.diff_scroll_offset.saturating_add(1);
                        } else {
                            self.findings.cursor_down();
                        }
                    }
                    KeyCode::Char('k') => {
                        if self.focus == Pane::Chat && self.current_diff.is_some() {
                            self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(1);
                        } else {
                            self.findings.cursor_up();
                        }
                    }
                    KeyCode::Char('f') => self.findings.triage_selected(Triage::Fix),
                    KeyCode::Char('t') => self.findings.triage_selected(Triage::TechDebt),
                    KeyCode::Char('i') => self.findings.triage_selected(Triage::Ignore),
                    KeyCode::Char('1') => self.focus = Pane::Logs,
                    KeyCode::Char('2') => self.focus = Pane::Chat,
                    KeyCode::Char('3') => self.focus = Pane::Issue,
                    KeyCode::Char('y') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        self.flash = Some(format!("✓ once: {}", perm.command));
                    }
                    KeyCode::Char('s') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        self.flash = Some(format!("✓ session: {}", perm.command));
                    }
                    KeyCode::Char('n') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        self.flash = Some(format!("✗ denied: {}", perm.command));
                    }
                    _ => {}
                }
                None
            }
            Mode::Command { buffer } => match key {
                KeyCode::Char(c) => {
                    buffer.push(c);
                    None
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    None
                }
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    None
                }
                KeyCode::Enter => {
                    let parsed = parse_command(buffer);
                    self.mode = Mode::Normal;
                    match parsed {
                        ParseResult::Empty => None,
                        ParseResult::Cmd(c) => {
                            self.last_status = None;
                            Some(c)
                        }
                        ParseResult::Err(msg) => {
                            self.last_status = Some(msg);
                            None
                        }
                    }
                }
                _ => None,
            },
        }
    }
}
