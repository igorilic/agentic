use std::io::Cursor;

use agentic_core::backends::claude_code::parser::{ParseOutcome, parse_stream};
use agentic_core::{Event, TokenUsage};
use tokio::io::BufReader;

/// Load a fixture file and return its bytes.
fn fixture(name: &str) -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/claude")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {name}: {e}"))
}

/// Run `parse_stream` on the given bytes and return all emitted events plus the
/// final `ParseOutcome`.
async fn run_parser(bytes: Vec<u8>) -> (Vec<Event>, ParseOutcome) {
    let (sink, mut rx) = tokio::sync::broadcast::channel(64);
    let reader = BufReader::new(Cursor::new(bytes));
    let outcome = parse_stream(reader, sink, "run-1".to_string(), Some("step-1".to_string()))
        .await
        .expect("parse_stream must not return Err");

    let mut events = Vec::new();
    while let Ok(env) = rx.try_recv() {
        events.push(env.event);
    }
    (events, outcome)
}

// ---------------------------------------------------------------------------
// message_start — should produce NO core events
// ---------------------------------------------------------------------------
#[tokio::test]
async fn message_start_emits_no_events() {
    let bytes = fixture("message_start.jsonl");
    let (events, _outcome) = run_parser(bytes).await;
    assert!(
        events.is_empty(),
        "expected no events for message_start, got: {events:?}"
    );
}

// ---------------------------------------------------------------------------
// text_delta — should emit 2× TextDelta
// ---------------------------------------------------------------------------
#[tokio::test]
async fn text_delta_emits_two_text_delta_events() {
    let bytes = fixture("text_delta.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    let text_deltas: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let Event::TextDelta { content } = e {
                Some(content.as_str())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        text_deltas.len(),
        2,
        "expected 2 TextDelta events, got: {events:?}"
    );
    assert_eq!(text_deltas[0], "Hello, ");
    assert_eq!(text_deltas[1], "world!");
}

// ---------------------------------------------------------------------------
// tool_use — should emit 1× ToolUseStart with correct id, name, and input
// ---------------------------------------------------------------------------
#[tokio::test]
async fn tool_use_emits_tool_use_start() {
    let bytes = fixture("tool_use.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    let tool_starts: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::ToolUseStart {
                tool_call_id,
                tool_name,
                input,
            } = e
            {
                Some((tool_call_id.clone(), tool_name.clone(), input.clone()))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        tool_starts.len(),
        1,
        "expected 1 ToolUseStart event, got: {events:?}"
    );
    let (id, name, input) = &tool_starts[0];
    assert_eq!(id, "toolu_01A09q90qw90lq917835lq9");
    assert_eq!(name, "Read");
    assert_eq!(input["file_path"], "/etc/hosts");
}

// ---------------------------------------------------------------------------
// message_delta_usage — token accumulator reflects usage from message_start
// and message_delta combined
// ---------------------------------------------------------------------------
#[tokio::test]
async fn message_delta_accumulates_token_usage() {
    let bytes = fixture("message_delta_usage.jsonl");
    let (_events, outcome) = run_parser(bytes).await;

    // message_start gave input_tokens=100
    // message_delta gave output_tokens=42, cache_read=20, cache_creation=5
    let expected = TokenUsage {
        input_tokens: 100,
        output_tokens: 42,
        cache_read_input_tokens: 20,
        cache_creation_input_tokens: 5,
    };
    assert_eq!(
        outcome.token_usage, expected,
        "token usage mismatch: got {:?}",
        outcome.token_usage
    );
}

// ---------------------------------------------------------------------------
// bad_json — parser emits exactly one Error event and then continues
// (the final message_stop line still processes without panic)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn bad_json_line_emits_protocol_error_and_continues() {
    let bytes = fixture("bad_json.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    let errors: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Error {
                code,
                recoverable,
                ..
            } = e
            {
                Some((code.clone(), recoverable))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        errors.len(),
        1,
        "expected exactly 1 Error event from bad json, got: {events:?}"
    );
    assert_eq!(errors[0].0, "protocol_error");
    assert!(!errors[0].1, "error should be non-recoverable");
}

// ---------------------------------------------------------------------------
// bad_json — parser continues after the bad line (no panic, outcome ok)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn bad_json_parser_continues_after_error() {
    let bytes = fixture("bad_json.jsonl");
    // Just verifying parse_stream returns Ok (no panic/Err) after bad JSON.
    let (sink, _rx) = tokio::sync::broadcast::channel(64);
    let reader = BufReader::new(Cursor::new(bytes));
    parse_stream(reader, sink, "run-x".to_string(), None)
        .await
        .expect("parse_stream should return Ok even when a line is bad JSON");
}
