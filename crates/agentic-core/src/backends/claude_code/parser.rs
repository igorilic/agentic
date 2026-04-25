//! Parser for the Claude CLI line-delimited JSON event stream.
//!
//! The Claude CLI (`claude -p --output-format stream-json`) emits a stream of
//! newline-delimited JSON objects. Each object has an outer `type` field that
//! identifies the envelope kind:
//!
//! - `system` — session lifecycle and hook events; mostly ignored.
//! - `assistant` — the key event; `message.content[]` holds text/thinking/tool_use blocks.
//! - `user` — tool results returned by the host; `message.content[]` holds `tool_result` blocks.
//! - `result` — final outcome; ignored (backend synthesises `ExecuteOutcome`).
//! - `rate_limit_event` — emitted when the CLI is rate-limited; translated to a recoverable Error.
//! - Unknown types — logged at debug level, silently ignored.
//! - Malformed JSON — one `Event::Error { code: "protocol_error" }` emitted, then parsing continues.
//!
//! # Truncated-stream safety (GH #23)
//!
//! Each line is an **atomic** JSON envelope. If the subprocess is killed mid-line
//! (e.g. SIGTERM, OOM), `AsyncBufReadExt::lines` returns the partial bytes as a
//! line, `serde_json::from_str` fails, and the `Err` branch already emits
//! `Event::Error { code: "protocol_error" }`. There is no "block_start without
//! block_stop" notion at this level — that was an artefact of the old SSE API.
//! GH #23 is therefore obsolete after the Step-6.1 parser rewrite.

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

/// Parse a line-delimited Claude CLI event stream.
///
/// Reads lines from `reader`, translates each JSON object into zero or more
/// core [`Event`]s that are sent via `sink`, and returns a [`ParseOutcome`]
/// with aggregated token usage on completion.
///
/// # Line-length assumption (GH #26)
///
/// Each line is read via [`tokio::io::AsyncBufReadExt::lines`], which allocates
/// each line as a full `String` bounded only by available memory. There is no
/// hard per-line size limit. Very long lines (e.g. base64-encoded file contents
/// inline in a `ToolUse` block) will be fully buffered before dispatch.
/// If this becomes a problem, a streaming NDJSON parser can be introduced at the
/// protocol layer; the public API of this function would remain unchanged.
///
/// # Orphan-delta invariant (GH #25)
///
/// The previous SSE-based parser tracked `block_start` / `block_stop` pairing
/// and had to handle orphaned partial deltas on subprocess kill. That invariant
/// is **obsolete** after the Step-6.1 rewrite: the Claude CLI now emits
/// complete, atomic JSON envelopes per line. A killed subprocess produces a
/// partial line, which `serde_json::from_str` rejects, causing a single
/// `Event::Error { code: "protocol_error" }` to be emitted. There is no delta
/// state to orphan.
pub async fn parse_stream<R: AsyncBufRead + Unpin>(
    reader: R,
    sink: EventSink,
    run_id: String,
    step_id: Option<String>,
) -> Result<ParseOutcome> {
    tracing::info!(
        run_id = %run_id,
        step_id = ?step_id,
        "claude parser: starting stream parse"
    );

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
                    tracing::debug!(
                        event_type = event_type_str(&event),
                        tool_call_id = %tool_call_id_of(&event).unwrap_or(""),
                        "claude parser: emitting event"
                    );
                    send_event(&sink, &run_id, &step_id, event);
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    line_preview = %line.chars().take(200).collect::<String>(),
                    "claude parser: malformed line"
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

#[derive(Deserialize)]
struct AssistantMessage {
    content: Vec<AssistantContentBlock>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AssistantContentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
}

#[derive(Deserialize)]
struct UserMessage {
    content: Vec<UserContentBlock>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UserContentBlock {
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        is_error: bool,
    },
}

#[derive(Deserialize, Default)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

// ---------------------------------------------------------------------------
// Dispatch — two-pass: outer type from Value, inner typed deserialization
// ---------------------------------------------------------------------------

/// Dispatch a parsed JSON object to the appropriate handler.
///
/// We first extract the outer `type` string from the `Value`, then
/// deserialise the relevant sub-objects with full type safety. This avoids
/// fighting serde's internally-tagged enum limitations when unknown fields
/// are present.
fn dispatch_envelope(json: &Value, token_usage: &mut TokenUsage) -> Vec<Event> {
    let outer_type = match json["type"].as_str() {
        Some(t) => t,
        None => {
            tracing::debug!("claude parser: envelope missing 'type' field");
            return vec![];
        }
    };

    match outer_type {
        "system" => {
            let subtype = json["subtype"].as_str().unwrap_or("");
            match subtype {
                "init" => {
                    tracing::debug!(subtype = "init", "claude parser: session init");
                }
                "tools_updated" => {
                    let model = json["model"].as_str().unwrap_or("<unknown>");
                    tracing::info!(
                        model = model,
                        "claude parser: tools_updated — model identified"
                    );
                }
                "hook_started" | "hook_response" => {
                    // Silently ignore hook lifecycle events.
                }
                other => {
                    tracing::debug!(subtype = other, "claude parser: unknown system subtype");
                }
            }
            vec![]
        }

        "assistant" => {
            let msg: AssistantMessage = match serde_json::from_value(json["message"].clone()) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, "claude parser: failed to parse assistant message");
                    return vec![];
                }
            };

            if msg.content.is_empty() {
                tracing::warn!("claude parser: assistant message has empty content[]");
            }

            let mut events = Vec::new();

            if let Some(usage) = msg.usage {
                token_usage.input_tokens += usage.input_tokens;
                token_usage.output_tokens += usage.output_tokens;
                token_usage.cache_creation_input_tokens += usage.cache_creation_input_tokens;
                token_usage.cache_read_input_tokens += usage.cache_read_input_tokens;
            }

            for block in msg.content {
                match block {
                    AssistantContentBlock::Text { text } => {
                        events.push(Event::TextDelta { content: text });
                    }
                    AssistantContentBlock::Thinking { thinking } => {
                        events.push(Event::ThinkingDelta { content: thinking });
                    }
                    AssistantContentBlock::ToolUse { id, name, input } => {
                        events.push(Event::ToolUseStart {
                            tool_call_id: id,
                            tool_name: name,
                            input,
                        });
                    }
                }
            }

            events
        }

        "user" => {
            let msg: UserMessage = match serde_json::from_value(json["message"].clone()) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(error = %e, "claude parser: failed to parse user message");
                    return vec![];
                }
            };

            msg.content
                .into_iter()
                .map(|block| match block {
                    UserContentBlock::ToolResult {
                        tool_use_id,
                        is_error,
                    } => Event::ToolUseEnd {
                        tool_call_id: tool_use_id,
                        exit_code: Some(if is_error { 1 } else { 0 }),
                        duration_ms: 0,
                    },
                })
                .collect()
        }

        "result" => {
            tracing::debug!("claude parser: result envelope, ignoring");
            vec![]
        }

        "rate_limit_event" => {
            let message = json["message"]
                .as_str()
                .unwrap_or("rate limit exceeded")
                .to_string();
            vec![Event::Error {
                code: "rate_limit_event".to_string(),
                message,
                recoverable: true,
                retry_after_ms: None,
            }]
        }

        other => {
            tracing::debug!(
                type_ = other,
                "claude parser: unknown envelope type, skipping"
            );
            vec![]
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return a short string name for the event variant, used in tracing.
fn event_type_str(event: &Event) -> &'static str {
    match event {
        Event::RunStarted { .. } => "RunStarted",
        Event::RunComplete { .. } => "RunComplete",
        Event::StepStarted { .. } => "StepStarted",
        Event::StepComplete { .. } => "StepComplete",
        Event::TextDelta { .. } => "TextDelta",
        Event::ThinkingDelta { .. } => "ThinkingDelta",
        Event::ToolUseStart { .. } => "ToolUseStart",
        Event::ToolUseDelta { .. } => "ToolUseDelta",
        Event::ToolUseEnd { .. } => "ToolUseEnd",
        Event::FileChange { .. } => "FileChange",
        Event::Finding { .. } => "Finding",
        Event::ClarifyingQuestion { .. } => "ClarifyingQuestion",
        Event::RetryStarted { .. } => "RetryStarted",
        Event::Error { .. } => "Error",
        Event::UserActionNeeded { .. } => "UserActionNeeded",
    }
}

/// Return the `tool_call_id` for events that carry one, otherwise `None`.
fn tool_call_id_of(event: &Event) -> Option<&str> {
    match event {
        Event::ToolUseStart { tool_call_id, .. } => Some(tool_call_id.as_str()),
        Event::ToolUseDelta { tool_call_id, .. } => Some(tool_call_id.as_str()),
        Event::ToolUseEnd { tool_call_id, .. } => Some(tool_call_id.as_str()),
        _ => None,
    }
}

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
