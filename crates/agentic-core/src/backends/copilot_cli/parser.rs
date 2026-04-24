//! Parser for the Copilot CLI line-delimited JSON event stream.
//!
//! The Copilot CLI emits a stream of newline-delimited JSON objects.
//! Each object has an outer `type` field that identifies the envelope kind.

use serde::Deserialize;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

use crate::TokenUsage;
use crate::backends::EventSink;
use crate::error::Result;

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
    _reader: R,
    _sink: EventSink,
    _run_id: String,
    _step_id: Option<String>,
) -> Result<ParseOutcome> {
    todo!("copilot parser not yet implemented")
}

// ---------------------------------------------------------------------------
// Wire types — used for sub-object deserialization after outer dispatch
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct EnvelopeHeader {
    #[serde(rename = "type")]
    ty: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct AssistantMessageDeltaData {
    #[serde(rename = "deltaContent")]
    delta_content: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct AssistantMessageData {
    #[serde(default, rename = "toolRequests")]
    tool_requests: Vec<CopilotToolRequest>,
    #[serde(default, rename = "outputTokens")]
    output_tokens: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct CopilotToolRequest {
    #[serde(rename = "toolCallId")]
    tool_call_id: String,
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct ToolExecutionCompleteData {
    #[serde(rename = "toolCallId")]
    tool_call_id: String,
    #[serde(default)]
    success: bool,
}
