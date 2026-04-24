//! Parser for the Copilot CLI line-delimited JSON event stream.
//!
//! The Copilot CLI emits a stream of newline-delimited JSON objects.
//! Each object has an outer `type` field that identifies the envelope kind:
//!
//! - `session.*` — session lifecycle; ignored.
//! - `user.message` — user turn; ignored.
//! - `assistant.turn_start` / `assistant.turn_end` — lifecycle; ignored.
//! - `assistant.message_delta` — streaming text chunk → `Event::TextDelta`.
//! - `assistant.message` — complete turn; accumulates outputTokens and emits
//!   `Event::ToolUseStart` for each entry in `toolRequests`.
//! - `tool.execution_start` — redundant (ToolUseStart already emitted); ignored.
//! - `tool.execution_complete` → `Event::ToolUseEnd`.
//! - `result` — backend synthesises outcome; parser ignores.
//! - Unknown types — logged at debug level, silently ignored.
//! - Malformed JSON — one `Event::Error { code: "protocol_error" }` emitted,
//!   parsing continues.

use serde::Deserialize;
use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use crate::TokenUsage;
use crate::backends::EventSink;
use crate::error::Result;
use crate::events::{CURRENT_SCHEMA_VERSION, Event, EventEnvelope};
use crate::time::now_ms;

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/// Final outcome returned by [`parse_stream`] once the stream is exhausted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParseOutcome {
    /// Accumulated token usage for this message.
    pub token_usage: TokenUsage,
    /// `true` if at least one `Event::Error { recoverable: false }` was observed.
    pub saw_unrecoverable_error: bool,
    /// The message from the first non-recoverable error seen, if any.
    pub error_message: Option<String>,
}

/// Parse a line-delimited Copilot CLI event stream.
///
/// Reads lines from `reader`, translates each JSON object into zero or more
/// core [`Event`]s that are sent via `sink`, and returns a [`ParseOutcome`]
/// with aggregated token usage on completion.
pub async fn parse_stream<R: AsyncBufRead + Unpin>(
    reader: R,
    sink: EventSink,
    run_id: String,
    step_id: Option<String>,
) -> Result<ParseOutcome> {
    let mut token_usage = TokenUsage::default();
    let mut saw_unrecoverable_error = false;
    let mut error_message: Option<String> = None;
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let outer: std::result::Result<Value, _> = serde_json::from_str(line.trim());
        match outer {
            Ok(json) => {
                let events = dispatch_envelope(&json, &mut token_usage);
                for event in events {
                    if let Event::Error {
                        recoverable: false,
                        ref message,
                        ..
                    } = event
                        && !saw_unrecoverable_error
                    {
                        saw_unrecoverable_error = true;
                        error_message = Some(message.clone());
                    }
                    send_event(&sink, &run_id, &step_id, event);
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    line_preview = %line.chars().take(200).collect::<String>(),
                    "copilot parser: malformed line"
                );
                send_event(
                    &sink,
                    &run_id,
                    &step_id,
                    Event::Error {
                        code: "protocol_error".to_string(),
                        message: format!("parse: {e}"),
                        recoverable: true,
                        retry_after_ms: None,
                    },
                );
            }
        }
    }

    Ok(ParseOutcome {
        token_usage,
        saw_unrecoverable_error,
        error_message,
    })
}

// ---------------------------------------------------------------------------
// Wire types — used for sub-object deserialization after outer dispatch
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
struct AssistantMessageDeltaData {
    #[serde(rename = "deltaContent")]
    delta_content: String,
}

#[derive(Deserialize, Debug)]
struct AssistantMessageData {
    #[serde(default, rename = "toolRequests")]
    tool_requests: Vec<CopilotToolRequest>,
    #[serde(default, rename = "outputTokens")]
    output_tokens: u64,
}

#[derive(Deserialize, Debug)]
struct CopilotToolRequest {
    #[serde(rename = "toolCallId")]
    tool_call_id: String,
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Deserialize, Debug)]
struct ToolExecutionCompleteData {
    #[serde(rename = "toolCallId")]
    tool_call_id: String,
    #[serde(default)]
    success: bool,
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Dispatch a parsed JSON object to the appropriate handler.
///
/// We first extract the outer `type` string from the `Value`, then
/// deserialise the relevant sub-objects with full type safety.
fn dispatch_envelope(json: &Value, token_usage: &mut TokenUsage) -> Vec<Event> {
    let outer_type = match json["type"].as_str() {
        Some(t) => t,
        None => {
            tracing::debug!("copilot parser: envelope missing 'type' field");
            return vec![];
        }
    };

    // Ignored lifecycle categories — silent.
    if outer_type.starts_with("session.") {
        return vec![];
    }

    match outer_type {
        "user.message" => vec![],

        "assistant.turn_start" | "assistant.turn_end" => vec![],

        "assistant.message_delta" => {
            let data: AssistantMessageDeltaData = match serde_json::from_value(json["data"].clone())
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "copilot parser: failed to parse message_delta data");
                    return vec![];
                }
            };
            vec![Event::TextDelta {
                content: data.delta_content,
            }]
        }

        "assistant.message" => {
            let data: AssistantMessageData = match serde_json::from_value(json["data"].clone()) {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "copilot parser: failed to parse assistant.message data");
                    return vec![];
                }
            };

            token_usage.output_tokens += data.output_tokens;

            // Only emit ToolUseStart events; text was already delivered via deltas.
            data.tool_requests
                .into_iter()
                .map(|req| Event::ToolUseStart {
                    tool_call_id: req.tool_call_id,
                    tool_name: req.name,
                    input: req.arguments,
                })
                .collect()
        }

        "tool.execution_start" => {
            // Redundant — ToolUseStart already emitted from assistant.message.toolRequests.
            vec![]
        }

        "tool.execution_complete" => {
            let data: ToolExecutionCompleteData = match serde_json::from_value(json["data"].clone())
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::warn!(error = %e, "copilot parser: failed to parse tool.execution_complete data");
                    return vec![];
                }
            };
            vec![Event::ToolUseEnd {
                tool_call_id: data.tool_call_id,
                exit_code: Some(if data.success { 0 } else { 1 }),
                duration_ms: 0,
            }]
        }

        "result" => {
            tracing::debug!("copilot parser: result envelope, ignoring");
            vec![]
        }

        other => {
            tracing::debug!(
                type_ = other,
                line_preview = %json.to_string().chars().take(120).collect::<String>(),
                "copilot parser: unknown envelope type, skipping"
            );
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn send_event(sink: &EventSink, run_id: &str, step_id: &Option<String>, event: Event) {
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: ulid::Ulid::new().to_string(),
        run_id: run_id.to_string(),
        step_id: step_id.clone(),
        timestamp_ms: now_ms(),
        event,
    };
    // Best-effort send; ignore errors (no receivers).
    let _ = sink.send(envelope);
}
