//! Application state for the TUI shell.
//!
//! Step 12.2: focus + resize. The `AppState` is intentionally small —
//! a `Pane` enum and a `cockpit_ratio: f32`. All state transitions go
//! through `handle(AppEvent)` so the bin's key-loop and integration
//! tests share the same dispatch surface.

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

#[derive(Debug, Clone, Copy)]
pub struct AppState {
    pub focus: Pane,
    /// Fraction of the horizontal width occupied by the cockpit pane.
    /// Clamped to [RATIO_MIN, RATIO_MAX] on every mutation.
    pub cockpit_ratio: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: Pane::Cockpit,
            cockpit_ratio: 0.50,
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
}
