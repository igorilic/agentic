//! Spec §4.6 — `Event::Finding` envelopes are ingested as `LogLevel::Warn`
//! rows in `state.log` via `apply_envelope`.
//!
//! The findings sidebar widget is no longer rendered; findings surface
//! exclusively as WARN rows in the unified logs pane (closes #99).

use agentic_core::events::{Event, EventEnvelope, Severity};
use agentic_tui::app::{AppState, LogLevel};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Build an EventEnvelope carrying a Finding event.
/// `timestamp_ms` is set to a known value so the formatted timestamp
/// can be asserted exactly: 3_661_000 ms → 01:01:01.
fn finding_envelope(id: &str, agent: &str, message: &str, timestamp_ms: i64) -> EventEnvelope {
    EventEnvelope {
        schema_version: 1,
        event_id: format!("evt-{id}"),
        run_id: "run1".to_string(),
        step_id: Some(format!("run1-step-{agent}")),
        timestamp_ms,
        event: Event::Finding {
            finding_id: id.to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: message.to_string(),
            suggestion: None,
        },
    }
}

// ── happy path ───────────────────────────────────────────────────────────────

#[test]
fn finding_envelope_pushes_warn_entry_to_state_log() {
    let mut s = AppState::default();
    // 3_661_000 ms = 1 h + 1 min + 1 s → "01:01:01"
    let env = finding_envelope("f1", "reviewer", "missing test", 3_661_000);
    s.apply_envelope(&env);

    assert_eq!(
        s.log.len(),
        1,
        "apply_envelope for Finding must push exactly one LogEntry to state.log"
    );
    let entry = &s.log[0];
    assert_eq!(
        entry.level,
        LogLevel::Warn,
        "Finding must produce a LogLevel::Warn entry"
    );
    assert_eq!(
        entry.agent, "reviewer",
        "agent must be derived from step_id"
    );
    assert_eq!(
        entry.message, "missing test",
        "message must match the Finding message"
    );
    assert_eq!(
        entry.timestamp, "01:01:01",
        "timestamp must be formatted as HH:MM:SS from envelope.timestamp_ms"
    );
}

// ── multiple findings ─────────────────────────────────────────────────────────

#[test]
fn two_finding_envelopes_push_two_warn_entries() {
    let mut s = AppState::default();
    s.apply_envelope(&finding_envelope("f1", "reviewer", "first finding", 0));
    s.apply_envelope(&finding_envelope("f2", "qa", "second finding", 0));

    assert_eq!(
        s.log.len(),
        2,
        "each Finding envelope must produce one log row"
    );
    assert_eq!(s.log[0].message, "first finding");
    assert_eq!(s.log[1].message, "second finding");
    assert!(matches!(s.log[0].level, LogLevel::Warn));
    assert!(matches!(s.log[1].level, LogLevel::Warn));
}

// ── agent derivation ─────────────────────────────────────────────────────────

#[test]
fn agent_falls_back_to_empty_string_when_step_id_is_none() {
    let mut s = AppState::default();
    let env = EventEnvelope {
        schema_version: 1,
        event_id: "evt-x".to_string(),
        run_id: "run1".to_string(),
        step_id: None,
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: "f1".to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "no agent".to_string(),
            suggestion: None,
        },
    };
    s.apply_envelope(&env);

    assert_eq!(s.log.len(), 1);
    assert_eq!(
        s.log[0].agent, "",
        "absent step_id must yield empty agent string"
    );
}

// ── timestamp edge case: zero ms ──────────────────────────────────────────────

#[test]
fn zero_timestamp_ms_formats_as_midnight() {
    let mut s = AppState::default();
    s.apply_envelope(&finding_envelope("f0", "architect", "at midnight", 0));

    assert_eq!(
        s.log[0].timestamp, "00:00:00",
        "0 ms must format as 00:00:00"
    );
}

// ── agent_from_step_id boundary cases ────────────────────────────────────────

/// `step_id` with no `-step-` separator → `agent` field must be empty string.
#[test]
fn agent_from_step_id_no_separator_yields_empty_string() {
    let mut s = AppState::default();
    let env = EventEnvelope {
        schema_version: 1,
        event_id: "evt-b1".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("run1".to_string()), // no "-step-" separator
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: "fb1".to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "no separator".to_string(),
            suggestion: None,
        },
    };
    s.apply_envelope(&env);

    assert_eq!(s.log.len(), 1);
    assert_eq!(
        s.log[0].agent, "",
        "step_id with no '-step-' separator must yield empty agent string"
    );
}

/// `step_id = Some("-step-")` (empty agent portion after separator) →
/// `agent` field must be empty string.
#[test]
fn agent_from_step_id_empty_after_separator_yields_empty_string() {
    let mut s = AppState::default();
    let env = EventEnvelope {
        schema_version: 1,
        event_id: "evt-b2".to_string(),
        run_id: "run1".to_string(),
        step_id: Some("-step-".to_string()), // separator present, agent part is ""
        timestamp_ms: 0,
        event: Event::Finding {
            finding_id: "fb2".to_string(),
            severity: Severity::Warning,
            file: None,
            line: None,
            message: "empty after separator".to_string(),
            suggestion: None,
        },
    };
    s.apply_envelope(&env);

    assert_eq!(s.log.len(), 1);
    assert_eq!(
        s.log[0].agent, "",
        "step_id='-step-' must yield empty agent string (empty portion after separator)"
    );
}
