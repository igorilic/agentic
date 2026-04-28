//! Step 12.5: TUI-side findings list.
//!
//! Shape mirrors `db::findings::FindingRow` minus the persistence
//! columns we don't render (run_id, step_id, suggestion, triaged_at,
//! created_at). The DB persists from the Tauri side; the TUI just
//! consumes `Event::Finding` envelopes off the bus and shows them.

use agentic_core::events::{Event, EventEnvelope, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Triage {
    Fix,
    TechDebt,
    Ignore,
}

impl Triage {
    /// Wire-compatible label — matches `db::findings::ALLOWED_TRIAGE`
    /// so the badge text in the TUI looks identical to the Tauri UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Fix => "fix",
            Self::TechDebt => "tech-debt",
            Self::Ignore => "ignore",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub message: String,
    pub triage: Option<Triage>,
}

#[derive(Debug, Clone, Default)]
pub struct FindingsState {
    pub items: Vec<Finding>,
    pub cursor: usize,
}

impl FindingsState {
    pub fn cursor_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let last = self.items.len() - 1;
        if self.cursor < last {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        // saturating_sub keeps us at 0 without wrapping.
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn triage_selected(&mut self, t: Triage) {
        if let Some(row) = self.items.get_mut(self.cursor) {
            row.triage = Some(t);
        }
    }

    /// Append a `Event::Finding` envelope to the list. Called from
    /// `AppState::apply_envelope` for every Finding event the bus emits.
    pub fn ingest(&mut self, envelope: &EventEnvelope) {
        if let Event::Finding {
            finding_id,
            severity,
            file,
            line,
            message,
            ..
        } = &envelope.event
        {
            self.items.push(Finding {
                id: finding_id.clone(),
                severity: *severity,
                file: file.as_ref().map(|p| p.display().to_string()),
                line: *line,
                message: message.clone(),
                triage: None,
            });
        }
    }
}
