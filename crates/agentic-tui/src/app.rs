//! Application state for the TUI shell.
//!
//! Step 12.2 added focus + resize. Step 12.3 adds a `RunState` that
//! the cockpit pane renders. Bus envelopes flow in via
//! [`AppState::apply_envelope`]; key presses still flow in via
//! [`AppState::handle`]. Both surfaces are pure mutators so the bin
//! and tests share them.

use agentic_core::events::EventEnvelope;
use crossterm::event::KeyCode;

use crate::modes::{AppCommand, Mode, parse_command};
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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Pane::Cockpit,
            cockpit_ratio: 0.50,
            run: RunState::default(),
            mode: Mode::Normal,
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

    /// Forward a bus envelope into the run-state machine. The bin's main
    /// loop will call this for every envelope yielded by `EventBus::subscribe`.
    pub fn apply_envelope(&mut self, envelope: &EventEnvelope) {
        self.run.apply_envelope(envelope);
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
                    let cmd = parse_command(buffer);
                    self.mode = Mode::Normal;
                    cmd
                }
                _ => None,
            },
        }
    }
}
