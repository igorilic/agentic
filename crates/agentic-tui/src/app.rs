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

/// Which pane currently receives input. Pure state — the renderer reads
/// it to decorate the focused pane's title; future steps (12.5 chat) will
/// route key events to the focused pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Cockpit,
    Chat,
}

/// High-level events the bin maps key presses onto. Keeping this an enum
/// (rather than calling state mutators directly) lets us add a TUI test
/// that exercises a sequence without cooking up real `crossterm::Event`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    ToggleFocus,
    /// `]` — give the cockpit one resize-step more.
    WidenCockpit,
    /// `[` — give the cockpit one resize-step less.
    NarrowCockpit,
}

/// Step size when resizing — 0.10 means each `]` / `[` shifts 10% of the
/// horizontal width. Spec §7.2 calls for a "noticeable but not jarring"
/// shift; 10% feels right at 80-column terminals.
const RESIZE_STEP: f32 = 0.10;
const RATIO_MIN: f32 = 0.20;
const RATIO_MAX: f32 = 0.80;

#[derive(Debug, Clone)]
pub struct AppState {
    pub focus: Pane,
    /// Fraction of the horizontal width occupied by the cockpit pane.
    /// Clamped to [RATIO_MIN, RATIO_MAX] on every mutation.
    pub cockpit_ratio: f32,
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
    /// Elapsed wall-clock seconds since the current run started.
    /// Formatted as `MM:SS` in the issue header pill.
    pub run_elapsed_secs: u64,
    /// Toggled by the render loop on every frame to drive the `●` pulse
    /// in the issue-header pill (spec §4.3). `false` = "on" phase (BLUE);
    /// `true` = "off" phase (DIM). Defaults to `false` (start lit).
    pub frame_parity: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Pane::Cockpit,
            cockpit_ratio: 0.50,
            run: RunState::default(),
            mode: Mode::Normal,
            findings: FindingsState::default(),
            last_status: None,
            current_diff: None,
            diff_scroll_offset: 0,
            run_label: None,
            run_title: None,
            run_elapsed_secs: 0,
            frame_parity: false,
        }
    }
}

impl AppState {
    pub fn handle(&mut self, event: AppEvent) {
        match event {
            AppEvent::ToggleFocus => {
                self.focus = match self.focus {
                    Pane::Cockpit => Pane::Chat,
                    Pane::Chat => Pane::Cockpit,
                };
            }
            AppEvent::WidenCockpit => {
                self.cockpit_ratio = (self.cockpit_ratio + RESIZE_STEP).clamp(RATIO_MIN, RATIO_MAX);
            }
            AppEvent::NarrowCockpit => {
                self.cockpit_ratio = (self.cockpit_ratio - RESIZE_STEP).clamp(RATIO_MIN, RATIO_MAX);
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
    /// - `Normal`: `:` enters command mode; `Tab` toggles focus; `[`/`]`
    ///   resize. Other keys are no-ops (chat input lands in 12.5).
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
                    KeyCode::Char(']') => self.handle(AppEvent::WidenCockpit),
                    KeyCode::Char('[') => self.handle(AppEvent::NarrowCockpit),
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
