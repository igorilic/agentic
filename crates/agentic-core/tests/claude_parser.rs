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
    let outcome = parse_stream(
        reader,
        sink,
        "run-1".to_string(),
        Some("step-1".to_string()),
    )
    .await
    .expect("parse_stream must not return Err");

    let mut events = Vec::new();
    while let Ok(env) = rx.try_recv() {
        events.push(env.event);
    }
    (events, outcome)
}

// ---------------------------------------------------------------------------
// assistant_text — should emit exactly one TextDelta
// ---------------------------------------------------------------------------

#[tokio::test]
async fn assistant_text_emits_one_text_delta() {
    let bytes = fixture("assistant_text.jsonl");
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
        1,
        "expected exactly 1 TextDelta event, got: {events:?}"
    );
    assert_eq!(text_deltas[0], "Hello! How can I help you today?");
}

// ---------------------------------------------------------------------------
// assistant_text — token usage is accumulated into ParseOutcome
// ---------------------------------------------------------------------------

#[tokio::test]
async fn assistant_usage_is_accumulated_into_parse_outcome() {
    let bytes = fixture("assistant_text.jsonl");
    let (_events, outcome) = run_parser(bytes).await;

    let expected = TokenUsage {
        input_tokens: 5,
        output_tokens: 5,
        cache_creation_input_tokens: 37438,
        cache_read_input_tokens: 0,
    };
    assert_eq!(
        outcome.token_usage, expected,
        "token usage mismatch: got {:?}",
        outcome.token_usage
    );
}

// ---------------------------------------------------------------------------
// assistant_with_tool_use — should emit exactly one ToolUseStart
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_use_block_emits_tool_use_start() {
    let bytes = fixture("assistant_with_tool_use.jsonl");
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
        "expected exactly 1 ToolUseStart event, got: {events:?}"
    );
    let (id, name, input) = &tool_starts[0];
    assert_eq!(id, "toolu_01A09q90qw90lq917835lq9");
    assert_eq!(name, "Read");
    assert_eq!(input["file_path"], "/etc/hosts");
}

// ---------------------------------------------------------------------------
// tool_use_roundtrip — ToolUseStart then ToolUseEnd for the same tool_call_id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn tool_result_emits_tool_use_end() {
    let bytes = fixture("tool_use_roundtrip.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    let start_ids: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let Event::ToolUseStart { tool_call_id, .. } = e {
                Some(tool_call_id.as_str())
            } else {
                None
            }
        })
        .collect();

    let end_ids: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let Event::ToolUseEnd { tool_call_id, .. } = e {
                Some(tool_call_id.as_str())
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        start_ids.len(),
        1,
        "expected exactly 1 ToolUseStart, got: {events:?}"
    );
    assert_eq!(
        end_ids.len(),
        1,
        "expected exactly 1 ToolUseEnd, got: {events:?}"
    );
    assert_eq!(
        start_ids[0], end_ids[0],
        "ToolUseStart and ToolUseEnd must share the same tool_call_id"
    );
    assert_eq!(start_ids[0], "toolu_roundtrip_01");
}

// ---------------------------------------------------------------------------
// hooks_noise — hook lines are silently ignored; only assistant events appear
// ---------------------------------------------------------------------------

#[tokio::test]
async fn system_hooks_are_ignored() {
    let bytes = fixture("hooks_noise.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    // No Error events should be emitted for hook lines
    let errors: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::Error { .. }))
        .collect();
    assert!(
        errors.is_empty(),
        "hook lines must not produce Error events, got: {errors:?}"
    );

    // The assistant text block should still be emitted
    let text_deltas: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::TextDelta { .. }))
        .collect();
    assert_eq!(
        text_deltas.len(),
        1,
        "expected exactly 1 TextDelta (from assistant, not hooks), got: {events:?}"
    );
}

// ---------------------------------------------------------------------------
// rate_limit — emits one recoverable Error with code "rate_limit_event"
// ---------------------------------------------------------------------------

#[tokio::test]
async fn rate_limit_emits_recoverable_error() {
    let bytes = fixture("rate_limit.jsonl");
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
                Some((code.clone(), *recoverable))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        errors.len(),
        1,
        "expected exactly 1 Error event from rate_limit, got: {events:?}"
    );
    assert_eq!(errors[0].0, "rate_limit_event");
    assert!(
        errors[0].1,
        "rate_limit_event must be recoverable"
    );
}

// ---------------------------------------------------------------------------
// bad_json — parser emits exactly one protocol_error and then continues
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bad_json_line_emits_protocol_error_and_continues() {
    let bytes = fixture("bad_json.jsonl");
    let (events, _outcome) = run_parser(bytes).await;

    let errors: Vec<_> = events
        .iter()
        .filter_map(|e| {
            if let Event::Error {
                code, recoverable, ..
            } = e
            {
                Some((code.clone(), *recoverable))
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
    assert!(
        errors[0].1,
        "protocol_error should be recoverable (parse hiccup, stream continues)"
    );

    // Parser must have continued past the bad line — the assistant TextDelta
    // that follows in the fixture must also be present.
    let text_deltas: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::TextDelta { .. }))
        .collect();
    assert_eq!(
        text_deltas.len(),
        1,
        "expected a TextDelta after the bad line (parser must continue), got: {events:?}"
    );
}

// ---------------------------------------------------------------------------
// bad_json — parser returns Ok even after encountering a bad line
// ---------------------------------------------------------------------------

#[tokio::test]
async fn bad_json_parser_continues_after_error() {
    let bytes = fixture("bad_json.jsonl");
    let (sink, _rx) = tokio::sync::broadcast::channel(64);
    let reader = BufReader::new(Cursor::new(bytes));
    parse_stream(reader, sink, "run-x".to_string(), None)
        .await
        .expect("parse_stream should return Ok even when a line is bad JSON");
}
