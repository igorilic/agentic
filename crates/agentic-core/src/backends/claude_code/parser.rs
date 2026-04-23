//! Parser for the Claude Agent SDK line-delimited JSON event stream.
//!
//! The Claude SDK uses the same streaming protocol as the Anthropic Messages
//! API: each line is a JSON object with a `type` field that identifies the
//! event kind.  This module consumes that stream and translates it into the
//! core `Event` variants.
//!
//! ## Design choices
//!
//! - Tool-use input is accumulated across `input_json_delta` lines and only
//!   parsed as JSON once `content_block_stop` is received. `ToolUseStart` is
//!   emitted at block_stop so the full `input` value is available.
//! - `message_start` initialises the input-token counter from the usage hint
//!   embedded in the message object but emits NO core `Event`.
//! - `message_delta` merges output/cache token counts into the accumulator.
//! - Malformed JSON lines produce one `Event::Error { code: "protocol_error" }`
//!   and parsing continues on the next line.

use serde_json::Value;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, Lines};

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
}

/// Parse a line-delimited Claude SDK event stream.
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
    let mut state = ParserState::new(run_id, step_id);
    let mut lines: Lines<R> = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(trimmed) {
            Ok(json) => {
                let events = state.process_line(&json);
                for event in events {
                    let envelope = EventEnvelope {
                        schema_version: CURRENT_SCHEMA_VERSION,
                        event_id: ulid::Ulid::new().to_string(),
                        run_id: state.run_id.clone(),
                        step_id: state.step_id.clone(),
                        timestamp_ms: now_ms(),
                        event,
                    };
                    // Best-effort send; ignore errors (no receivers).
                    let _ = sink.send(envelope);
                }
            }
            Err(err) => {
                let envelope = EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    event_id: ulid::Ulid::new().to_string(),
                    run_id: state.run_id.clone(),
                    step_id: state.step_id.clone(),
                    timestamp_ms: now_ms(),
                    event: Event::Error {
                        code: "protocol_error".to_string(),
                        message: err.to_string(),
                        recoverable: true,
                        retry_after_ms: None,
                    },
                };
                let _ = sink.send(envelope);
                // Continue parsing the next line.
            }
        }
    }

    Ok(ParseOutcome {
        token_usage: state.token_acc.finalize(),
    })
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// Per-block state for an in-progress `tool_use` content block.
struct ToolUseBlock {
    id: String,
    name: String,
    /// Accumulates partial `input_json_delta` strings.
    input_json_buf: String,
}

/// Accumulates token usage fields across `message_start` and `message_delta`
/// lines and produces a final `TokenUsage` on completion.
#[derive(Debug, Default)]
struct TokenAccumulator {
    inner: TokenUsage,
}

impl TokenAccumulator {
    /// Merge a `usage` JSON object into the running totals.
    /// Present fields overwrite; absent fields are left unchanged.
    fn absorb(&mut self, usage: &serde_json::Map<String, Value>) {
        if let Some(n) = usage.get("input_tokens").and_then(Value::as_u64) {
            self.inner.input_tokens = n;
        }
        if let Some(n) = usage.get("output_tokens").and_then(Value::as_u64) {
            self.inner.output_tokens = n;
        }
        if let Some(n) = usage.get("cache_read_input_tokens").and_then(Value::as_u64) {
            self.inner.cache_read_input_tokens = n;
        }
        if let Some(n) = usage
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64)
        {
            self.inner.cache_creation_input_tokens = n;
        }
    }

    /// Consume the accumulator and return the final `TokenUsage`.
    fn finalize(self) -> TokenUsage {
        self.inner
    }
}

/// Mutable parser state threaded through all line-dispatch calls.
struct ParserState {
    run_id: String,
    step_id: Option<String>,
    token_acc: TokenAccumulator,
    /// Present when we are inside a `tool_use` content block.
    current_tool: Option<ToolUseBlock>,
}

impl ParserState {
    fn new(run_id: String, step_id: Option<String>) -> Self {
        Self {
            run_id,
            step_id,
            token_acc: TokenAccumulator::default(),
            current_tool: None,
        }
    }

    /// Dispatch one parsed JSON object and return the resulting events (0-N).
    fn process_line(&mut self, json: &Value) -> Vec<Event> {
        let event_type = match json["type"].as_str() {
            Some(t) => t,
            None => return vec![],
        };

        match event_type {
            "message_start" => self.handle_message_start(json),
            "content_block_start" => self.handle_content_block_start(json),
            "content_block_delta" => self.handle_content_block_delta(json),
            "content_block_stop" => self.handle_content_block_stop(),
            "message_delta" => self.handle_message_delta(json),
            "message_stop" | "ping" => vec![],
            "error" => Self::handle_upstream_error(json),
            _ => vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Handlers
    // -----------------------------------------------------------------------

    fn handle_message_start(&mut self, json: &Value) -> Vec<Event> {
        // Absorb the initial usage hint from message_start.
        if let Some(usage) = json["message"]["usage"].as_object() {
            self.token_acc.absorb(usage);
        }
        // No core event emitted for message_start.
        vec![]
    }

    fn handle_content_block_start(&mut self, json: &Value) -> Vec<Event> {
        let block = &json["content_block"];
        match block["type"].as_str() {
            Some("tool_use") => {
                let id = string_field(block, "id");
                let name = string_field(block, "name");
                self.current_tool = Some(ToolUseBlock {
                    id,
                    name,
                    input_json_buf: String::new(),
                });
                vec![]
            }
            _ => {
                // text or thinking block — nothing to track specially
                vec![]
            }
        }
    }

    fn handle_content_block_delta(&mut self, json: &Value) -> Vec<Event> {
        let delta = &json["delta"];
        match delta["type"].as_str() {
            Some("text_delta") => {
                let text = string_field(delta, "text");
                vec![Event::TextDelta { content: text }]
            }
            Some("thinking_delta") => {
                let text = string_field(delta, "thinking");
                vec![Event::ThinkingDelta { content: text }]
            }
            Some("input_json_delta") => {
                if let (Some(tool), Some(partial)) =
                    (&mut self.current_tool, delta["partial_json"].as_str())
                {
                    tool.input_json_buf.push_str(partial);
                }
                vec![]
            }
            _ => vec![],
        }
    }

    fn handle_content_block_stop(&mut self) -> Vec<Event> {
        // If we were accumulating a tool_use block, emit ToolUseStart now
        // that the full input JSON is known.
        if let Some(tool) = self.current_tool.take() {
            let input: Value = serde_json::from_str(&tool.input_json_buf)
                .unwrap_or(Value::Object(serde_json::Map::new()));
            return vec![Event::ToolUseStart {
                tool_call_id: tool.id,
                tool_name: tool.name,
                input,
            }];
        }
        vec![]
    }

    fn handle_message_delta(&mut self, json: &Value) -> Vec<Event> {
        // message_delta carries output token counts in the top-level `usage`
        // field (NOT nested under `delta`).
        if let Some(usage) = json["usage"].as_object() {
            self.token_acc.absorb(usage);
        }
        vec![]
    }

    /// Handle an Anthropic-level `{"type":"error","error":{...}}` payload.
    ///
    /// These are well-formed JSON objects but represent upstream API errors
    /// (overload, rate-limit, auth failure, etc.).  Parsing continues after
    /// emitting the error event.
    fn handle_upstream_error(json: &Value) -> Vec<Event> {
        let error_obj = match json.get("error") {
            Some(obj) => obj,
            None => {
                return vec![Event::Error {
                    code: "upstream_error".to_string(),
                    message: "error event missing body".to_string(),
                    recoverable: false,
                    retry_after_ms: None,
                }];
            }
        };

        let code = error_obj["type"]
            .as_str()
            .unwrap_or("upstream_error")
            .to_string();
        let message = error_obj["message"].as_str().unwrap_or("").to_string();
        let retry_after_ms = error_obj["retry_after"].as_u64().map(|secs| secs * 1000);

        let recoverable = matches!(code.as_str(), "overloaded_error" | "rate_limit_error");

        vec![Event::Error {
            code,
            message,
            recoverable,
            retry_after_ms,
        }]
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract a string field from a JSON object, defaulting to empty string.
fn string_field(obj: &Value, key: &str) -> String {
    obj[key].as_str().unwrap_or("").to_string()
}
