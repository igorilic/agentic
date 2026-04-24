//! Integration tests for the Copilot CLI stream parser.
//!
//! Each test loads a real fixture from tests/fixtures/copilot/ and asserts
//! the correct core Events are emitted.

use std::io::Cursor;

use agentic_core::backends::copilot_cli::parser::{ParseOutcome, parse_stream};
use agentic_core::events::{Event, EventEnvelope};
use tokio::io::BufReader;
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/copilot")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

async fn run_parser(fixture_name: &str) -> (Vec<EventEnvelope>, ParseOutcome) {
    let bytes = fixture(fixture_name);
    let reader = BufReader::new(Cursor::new(bytes));
    let (tx, mut rx) = broadcast::channel::<EventEnvelope>(256);

    let outcome = parse_stream(reader, tx, "run-test".to_string(), Some("step-test".to_string()))
        .await
        .expect("parse_stream returned Err");

    let mut events = Vec::new();
    while let Ok(env) = rx.try_recv() {
        events.push(env);
    }

    (events, outcome)
}

async fn run_parser_str(jsonl: &str) -> (Vec<EventEnvelope>, ParseOutcome) {
    let reader = BufReader::new(Cursor::new(jsonl.to_owned()));
    let (tx, mut rx) = broadcast::channel::<EventEnvelope>(256);

    let outcome = parse_stream(reader, tx, "run-test".to_string(), None)
        .await
        .expect("parse_stream returned Err");

    let mut events = Vec::new();
    while let Ok(env) = rx.try_recv() {
        events.push(env);
    }

    (events, outcome)
}

// ---------------------------------------------------------------------------
// Test 1: hello_simple emits text deltas and accumulates tokens
// ---------------------------------------------------------------------------

#[tokio::test]
async fn hello_simple_emits_text_deltas_and_accumulates_tokens() {
    let (events, outcome) = run_parser("hello_simple.jsonl").await;

    // At least one TextDelta emitted.
    let text_deltas: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event, Event::TextDelta { .. }))
        .collect();
    assert!(
        !text_deltas.is_empty(),
        "expected at least one TextDelta, got none. All events: {events:?}"
    );

    // outputTokens = 2 per the fixture assistant.message.
    assert_eq!(
        outcome.token_usage.output_tokens, 2,
        "expected output_tokens = 2 from fixture, got {}",
        outcome.token_usage.output_tokens
    );

    // No ToolUseStart emitted.
    let tool_starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event, Event::ToolUseStart { .. }))
        .collect();
    assert!(
        tool_starts.is_empty(),
        "expected no ToolUseStart, got {tool_starts:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: tool_use_bash emits ToolUseStart and ToolUseEnd for both tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_use_bash_emits_tool_use_start_and_end() {
    let (events, outcome) = run_parser("tool_use_bash.jsonl").await;

    let starts: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event, Event::ToolUseStart { .. }))
        .collect();
    assert_eq!(
        starts.len(),
        2,
        "expected 2 ToolUseStart events (report_intent + bash), got {}: {starts:?}",
        starts.len()
    );

    // Verify tool names: report_intent and bash.
    let tool_names: Vec<&str> = starts
        .iter()
        .map(|e| match &e.event {
            Event::ToolUseStart { tool_name, .. } => tool_name.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert!(
        tool_names.contains(&"report_intent"),
        "expected report_intent in tool names: {tool_names:?}"
    );
    assert!(
        tool_names.contains(&"bash"),
        "expected bash in tool names: {tool_names:?}"
    );

    let ends: Vec<_> = events
        .iter()
        .filter(|e| matches!(e.event, Event::ToolUseEnd { .. }))
        .collect();
    assert_eq!(
        ends.len(),
        2,
        "expected 2 ToolUseEnd events, got {}: {ends:?}",
        ends.len()
    );

    // Verify tool_call_ids match between starts and ends.
    let start_ids: std::collections::HashSet<&str> = starts
        .iter()
        .map(|e| match &e.event {
            Event::ToolUseStart { tool_call_id, .. } => tool_call_id.as_str(),
            _ => unreachable!(),
        })
        .collect();
    let end_ids: std::collections::HashSet<&str> = ends
        .iter()
        .map(|e| match &e.event {
            Event::ToolUseEnd { tool_call_id, .. } => tool_call_id.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        start_ids, end_ids,
        "tool_call_ids do not match between ToolUseStart and ToolUseEnd"
    );

    // outputTokens accumulates from both assistant.message envelopes:
    // fixture has 125 (first) + 63 (second).
    assert_eq!(
        outcome.token_usage.output_tokens,
        125 + 63,
        "expected output_tokens = 188 (125+63), got {}",
        outcome.token_usage.output_tokens
    );
}

// ---------------------------------------------------------------------------
// Test 3: bad_json line emits protocol_error and continues
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bad_json_line_emits_protocol_error_and_continues() {
    let (events, _outcome) = run_parser("bad_json.jsonl").await;

    let errors: Vec<_> = events
        .iter()
        .filter(|e| {
            matches!(
                &e.event,
                Event::Error { code, .. } if code == "protocol_error"
            )
        })
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "expected exactly 1 protocol_error event, got {}: {errors:?}",
        errors.len()
    );

    // Should not have panicked (we're still here).
}

// ---------------------------------------------------------------------------
// Test 4: session and lifecycle events are ignored (no extra events)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_and_lifecycle_events_are_ignored() {
    let (events, _outcome) = run_parser("hello_simple.jsonl").await;

    // The hello_simple fixture produces exactly:
    // - 1 TextDelta (from assistant.message_delta)
    // No session.*, user.message, assistant.turn_start/end, result events.
    for env in &events {
        match &env.event {
            Event::TextDelta { .. } => {} // expected
            other => panic!(
                "unexpected event type in hello_simple fixture: {other:?}"
            ),
        }
    }

    // Exactly 1 event (the single message_delta).
    assert_eq!(
        events.len(),
        1,
        "expected exactly 1 event from hello_simple, got {}: {events:?}",
        events.len()
    );
}

// ---------------------------------------------------------------------------
// Test 5: unknown type does not crash
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_type_does_not_crash() {
    let jsonl = r#"{"type":"future.unknown_event","data":{"foo":"bar"}}"#;
    let (events, outcome) = run_parser_str(jsonl).await;

    // No events emitted for unknown types.
    assert!(
        events.is_empty(),
        "expected no events for unknown type, got: {events:?}"
    );

    // No error accumulated.
    assert!(
        !outcome.saw_unrecoverable_error,
        "unexpected unrecoverable error for unknown type"
    );
}
