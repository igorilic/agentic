//! Extract structured findings from reviewer agent text output.
//!
//! Convention: the reviewer agent's system prompt instructs it to end its
//! response with a fenced markdown block tagged `agentic-findings`
//! containing a JSON array. Each entry is a [`FindingDraft`].
//!
//! Example reviewer output:
//!
//! ```text
//! Review summary: 2 issues found.
//!
//! ```agentic-findings
//! [
//!   {"finding_id":"f1","severity":"warning","file":"src/auth.rs","line":42,
//!    "message":"missing rate-limit on /login","suggestion":"add tower-governor"}
//! ]
//! ```
//! ```
//!
//! Robustness: malformed JSON, missing fence, and empty arrays all return
//! an empty `Vec` so callers never insert partial / invalid rows.

use serde::{Deserialize, Serialize};

/// Reviewer-emitted finding before persistence. Mirrors the persistence
/// shape but without DB ids/timestamps which the caller fills in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingDraft {
    pub finding_id: String,
    pub severity: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<u32>,
    pub message: String,
    #[serde(default)]
    pub suggestion: Option<String>,
}

const FENCE_TAG: &str = "agentic-findings";

/// Extract findings from a reviewer agent's accumulated text output.
///
/// Scans for fenced blocks tagged `agentic-findings`. If multiple are
/// present, returns the entries from the LAST block (the agent may have
/// drafted then revised; the final block wins).
pub fn extract_findings(text: &str) -> Vec<FindingDraft> {
    let Some(payload) = last_fenced_block(text, FENCE_TAG) else {
        return vec![];
    };
    match serde_json::from_str::<Vec<FindingDraft>>(payload) {
        Ok(list) => list,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "findings extractor: agentic-findings block had invalid JSON; skipping",
            );
            vec![]
        }
    }
}

/// Return the body of the LAST ```<tag> ... ``` block in `text`, or `None`.
/// Tag match is exact (no leading whitespace tolerance — markdown parsers
/// vary, but our convention is strict).
fn last_fenced_block<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let opener = format!("```{tag}");
    let mut last_body: Option<&str> = None;
    let mut search_from = 0usize;
    while let Some(rel_open) = text[search_from..].find(&opener) {
        let abs_open = search_from + rel_open;
        // Body starts after the opener + the newline that follows it.
        let after_opener = abs_open + opener.len();
        let body_start = match text[after_opener..].find('\n') {
            Some(nl) => after_opener + nl + 1,
            None => return last_body, // malformed: no newline after opener
        };
        // Body ends at the next "```" on its own line (or at EOF).
        let close_rel = text[body_start..].find("\n```");
        let body_end = match close_rel {
            Some(c) => body_start + c,
            None => return last_body, // malformed: no closing fence
        };
        last_body = Some(&text[body_start..body_end]);
        // Continue searching past the close fence.
        search_from = body_end + "\n```".len();
    }
    last_body
}
