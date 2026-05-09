//! Application state for the TUI shell.
//!
//! Step 12.2 added focus + resize. Step 12.3 adds a `RunState` that
//! the cockpit pane renders. Bus envelopes flow in via
//! [`AppState::apply_envelope`]; key presses still flow in via
//! [`AppState::handle`]. Both surfaces are pure mutators so the bin
//! and tests share them.

use std::cell::Cell;
use std::time::{Duration, Instant};

use std::sync::Arc;

use agentic_core::events::{Event, EventBus, EventEnvelope, PermissionDecision, PermissionSource};
use crossterm::event::KeyCode;

/// Re-export the wire `PermissionRisk` so callers that previously used the
/// local `app::PermissionRisk` continue to work without import changes.
pub use agentic_core::events::PermissionRisk;

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

/// One-line flash message shown in the status line (spec §4.8 / §4.10).
///
/// T.13.4 will add `expires_at: Instant` additively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flash {
    pub text: String,
}

/// A single permission request pending user approval (spec §4.7).
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Unique identifier for this request; used to match against
    /// `Event::PermissionResolved` to remove the entry (P.5.1).
    pub request_id: String,
    /// Agent requesting the permission, e.g. `"developer"`.
    pub agent: String,
    /// Shell command or action requiring approval, e.g. `"rm -rf node_modules"`.
    ///
    /// Mapped from the wire envelope's `arg` field. The `tool` field is
    /// intentionally dropped here: the TUI perm card doesn't render the tool
    /// name separately, and for Bash the command string already carries the
    /// tool context. Pretty-printing structured tool args (Write, Edit, etc.)
    /// is deferred to a later layout step.
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

#[derive(Clone)]
pub struct AppState {
    pub focus: Pane,
    /// Pipeline run state — renders as the cockpit stepper.
    pub run: RunState,
    /// Normal vs. Command — see `modes.rs`.
    pub mode: Mode,
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
    /// Vertical scroll offset for the logs pane: number of rows scrolled
    /// past the top. 0 = top of log visible. Reset when sticky tail
    /// clamps at render time. (GH #100)
    pub log_scroll: usize,
    /// When true the logs pane auto-follows new entries (sticky tail).
    /// Set to false when the user scrolls up; restored when they scroll
    /// back to the bottom. Defaults to true. (GH #100)
    pub log_sticky_tail: bool,
    /// Last render-time height of the logs pane body area in rows.
    /// Written by `views/logs_pane::render` on every frame via interior
    /// mutability so the j/Down handler can compute the true bottom
    /// without needing a mutable reference to AppState. Defaults to 0
    /// (means "unknown"; the handler treats 0 as height=1 for safety).
    /// (GH #100 fix-loop 1)
    pub last_known_log_height: Cell<usize>,
    /// Chat messages rendered in the chat pane (spec §4.6).
    /// Populated by the runner wiring in T.13.x; empty by default.
    pub chat: Vec<ChatMessage>,
    /// Pending permission requests (spec §4.7). When non-empty and the logs
    /// pane is focused, the first entry renders as an inline permission card
    /// after the last log row. Keys y/s/n resolve it (T.13.2).
    pub pending_perms: Vec<PermissionRequest>,
    /// One-line flash message shown in the status line in place of the hint
    /// for ~1.6 s (spec §4.8). Set by perm resolution keys (T.13.2).
    pub flash: Option<Flash>,
    /// Timestamp when `flash` was last set. Used by `tick()` to determine
    /// when the flash lifetime (~1.6 s) has expired and the message should
    /// be cleared (T.13.4).
    pub flash_set_at: Option<Instant>,
    /// Whether the help overlay is currently displayed (spec §4.9).
    /// Toggled by `?` in Normal mode; closed by Esc (takes precedence over
    /// all other Esc handling).
    pub help_open: bool,
    /// Bus handle for publishing `PermissionResolved` envelopes back to the
    /// orchestrator when the user presses y/s/n. `None` in tests that don't
    /// need bus egress (existing tests construct `AppState::default()`).
    /// Set by `run.rs` from the runtime bus after `EventBus::subscribe`.
    pub bus: Option<Arc<EventBus>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // bus is excluded — EventBus does not implement Debug.
        f.debug_struct("AppState")
            .field("focus", &self.focus)
            .field("mode", &self.mode)
            .field("pending_perms", &self.pending_perms)
            .field("flash", &self.flash)
            .field("help_open", &self.help_open)
            .field("bus", &self.bus.as_ref().map(|_| "<EventBus>"))
            .finish_non_exhaustive()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Pane::Logs,
            run: RunState::default(),
            mode: Mode::Normal,
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
            log_scroll: 0,
            log_sticky_tail: true,
            last_known_log_height: Cell::new(0),
            chat: vec![],
            pending_perms: vec![],
            flash: None,
            flash_set_at: None,
            help_open: false,
            bus: None,
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
    /// Also handles `Event::PermissionRequest` (push to `pending_perms`) and
    /// `Event::PermissionResolved` (remove by `request_id`). P.5.1.
    ///
    /// The bin's main loop will call this for every envelope yielded by
    /// `EventBus::subscribe`.
    pub fn apply_envelope(&mut self, envelope: &EventEnvelope) {
        self.run.apply_envelope(envelope);

        match &envelope.event {
            Event::Finding { message, .. } => {
                self.log.push(LogEntry {
                    timestamp: format_hms(envelope.timestamp_ms),
                    agent: agent_from_step_id(envelope.step_id.as_deref()),
                    level: LogLevel::Warn,
                    message: message.clone(),
                });
            }
            Event::PermissionRequest {
                request_id,
                agent,
                tool: _,
                arg,
                scope,
                risk,
                reason,
            } => {
                self.pending_perms.push(PermissionRequest {
                    request_id: request_id.clone(),
                    agent: agent.clone(),
                    command: arg.clone(),
                    reason: reason.clone(),
                    scope: scope.clone(),
                    risk: *risk,
                });
            }
            Event::PermissionResolved { request_id, .. } => {
                self.pending_perms.retain(|p| &p.request_id != request_id);
            }
            _ => {}
        }
    }

    /// Replace the currently-viewed diff text and re-anchor scroll to
    /// the top. Use this from the runtime path so swapping files
    /// can't leave a stale scroll offset visible.
    pub fn set_diff(&mut self, text: Option<String>) {
        self.current_diff = text;
        self.diff_scroll_offset = 0;
    }

    /// Called once per render iteration. Clears expired flash messages
    /// (lifetime ~1.6 s per spec §4.8).
    pub fn tick(&mut self) {
        const FLASH_LIFETIME: Duration = Duration::from_millis(1600);
        if let Some(t) = self.flash_set_at {
            if t.elapsed() >= FLASH_LIFETIME {
                self.flash = None;
                self.flash_set_at = None;
            }
        } else if self.flash.is_some() {
            // flash_set_at missing — clear defensively rather than displaying forever.
            self.flash = None;
        }
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
        // Esc closes the help overlay first, regardless of mode.
        if self.help_open && key == KeyCode::Esc {
            self.help_open = false;
            return None;
        }

        match &mut self.mode {
            Mode::Normal => {
                match key {
                    KeyCode::Char(':') => {
                        self.mode = Mode::Command {
                            buffer: String::new(),
                        };
                    }
                    KeyCode::Tab => self.handle(AppEvent::ToggleFocus),
                    // Scroll logs down when Logs pane is focused. (GH #100)
                    KeyCode::Char('j') | KeyCode::Down
                        if self.focus == Pane::Logs && !self.log.is_empty() =>
                    {
                        // Clamp at log.len()-1 so scroll never exceeds content.
                        self.log_scroll = (self.log_scroll.saturating_add(1))
                            .min(self.log.len().saturating_sub(1));
                        // Re-enable sticky tail when the user scrolls back to
                        // the bottom. max_scroll mirrors the renderer's formula:
                        //   len - (visible_height - 1)   [when indicator present]
                        // We use last_known_log_height; treat 0 as 1 (safe floor).
                        let h = self.last_known_log_height.get().max(1);
                        let max_scroll = self.log.len().saturating_sub(h.saturating_sub(1));
                        self.log_sticky_tail = self.log_scroll >= max_scroll;
                    }
                    // Scroll logs up when Logs pane is focused. (GH #100)
                    KeyCode::Char('k') | KeyCode::Up if self.focus == Pane::Logs => {
                        self.log_scroll = self.log_scroll.saturating_sub(1);
                        self.log_sticky_tail = false;
                    }
                    // Scroll diff down when Chat+diff is active; no-op otherwise
                    // (findings navigation removed, refs #99).
                    KeyCode::Char('j')
                        if self.focus == Pane::Chat && self.current_diff.is_some() =>
                    {
                        self.diff_scroll_offset = self.diff_scroll_offset.saturating_add(1);
                    }
                    // Scroll diff up when Chat+diff is active; no-op otherwise.
                    KeyCode::Char('k')
                        if self.focus == Pane::Chat && self.current_diff.is_some() =>
                    {
                        self.diff_scroll_offset = self.diff_scroll_offset.saturating_sub(1);
                    }
                    // Enter Insert mode from Logs/Chat; Issue pane no-op
                    // (triage UI removed, refs #99).
                    KeyCode::Char('i')
                        if (self.focus == Pane::Logs || self.focus == Pane::Chat) =>
                    {
                        self.mode = Mode::Insert;
                    }
                    KeyCode::Char('1') => self.focus = Pane::Logs,
                    KeyCode::Char('2') => self.focus = Pane::Chat,
                    KeyCode::Char('3') => self.focus = Pane::Issue,
                    KeyCode::Char('y') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        let prefix = perm.scope.split('.').next().unwrap_or("shell");
                        self.flash = Some(Flash {
                            text: format!("✓ once: {} \"{}\"", prefix, perm.command),
                        });
                        self.flash_set_at = Some(Instant::now());
                        self.publish_resolution(perm.request_id, PermissionDecision::AllowOnce);
                    }
                    KeyCode::Char('s') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        let prefix = perm.scope.split('.').next().unwrap_or("shell");
                        self.flash = Some(Flash {
                            text: format!("✓ session: {} \"{}\"", prefix, perm.command),
                        });
                        self.flash_set_at = Some(Instant::now());
                        self.publish_resolution(perm.request_id, PermissionDecision::AllowSession);
                    }
                    KeyCode::Char('n') if !self.pending_perms.is_empty() => {
                        let perm = self.pending_perms.remove(0);
                        let prefix = perm.scope.split('.').next().unwrap_or("shell");
                        self.flash = Some(Flash {
                            text: format!("✗ denied: {} \"{}\"", prefix, perm.command),
                        });
                        self.flash_set_at = Some(Instant::now());
                        self.publish_resolution(perm.request_id, PermissionDecision::Deny);
                    }
                    KeyCode::Char('?') => {
                        self.help_open = !self.help_open;
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
            // T.13.6: Insert mode key routing.
            // Esc returns to Normal. Other keys are no-ops until the
            // chat-compose widget lands in a later step.
            Mode::Insert => {
                if key == KeyCode::Esc {
                    self.mode = Mode::Normal;
                }
                None
            }
        }
    }

    /// Publish a `PermissionResolved` envelope to the bus when the user
    /// resolves a pending permission with y/s/n. No-op when `self.bus` is
    /// `None` (test mode or not yet wired to the runtime bus).
    fn publish_resolution(&self, request_id: String, decision: PermissionDecision) {
        if let Some(bus) = &self.bus {
            let run_id = self
                .run_label
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            let envelope = EventEnvelope::now(
                run_id,
                None,
                Event::PermissionResolved {
                    request_id,
                    decision,
                    source: PermissionSource::User,
                },
            );
            bus.publish(envelope);
        }
    }
}

/// Format a millisecond timestamp as `HH:MM:SS`.
///
/// Uses the time-of-day portion of the value modulo 24 hours so that
/// the column always shows a clock-style string regardless of the epoch.
pub(crate) fn format_hms(timestamp_ms: i64) -> String {
    let total_secs = (timestamp_ms.unsigned_abs() / 1000) % 86_400;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Derive the agent name from a `step_id` string of the form
/// `"{run_id}-step-{agent}"`.  Returns the portion after the last
/// `-step-` separator, or an empty string when the pattern is absent.
pub(crate) fn agent_from_step_id(step_id: Option<&str>) -> String {
    step_id
        .and_then(|s| s.rsplit_once("-step-"))
        .map(|(_, agent)| agent.to_string())
        .unwrap_or_default()
}
