use async_trait::async_trait;
use base64::Engine as _;

use super::{
    Ticket, TicketComment, TicketSource, TicketSourceError, github::parse_acceptance_criteria,
};
use crate::events::{TicketKind, TicketRef};

/// Authentication scheme for the Jira REST API.
///
/// - `Basic`: email + API token, sent as `Authorization: Basic <base64>`.
///   This is what Atlassian Cloud (`*.atlassian.net`) accepts.
/// - `Bearer`: Personal Access Token, sent as `Authorization: Bearer <token>`.
///   This is what self-hosted Jira Server / Data Centre accepts.
#[derive(Clone, Debug)]
pub enum JiraAuth {
    Basic { email: String, token: String },
    Bearer { token: String },
}

pub struct JiraTicketSource {
    /// Base URL e.g. "https://org.atlassian.net/rest/api/3".
    base_url: String,
    /// Authentication scheme (Basic or Bearer).
    auth: JiraAuth,
    /// Optional custom field id holding Acceptance Criteria,
    /// e.g. "customfield_10100". When None, AC is parsed from description.
    ac_custom_field: Option<String>,
    client: reqwest::Client,
}

impl JiraTicketSource {
    pub fn new(
        base_url: impl Into<String>,
        auth: JiraAuth,
        ac_custom_field: Option<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            auth,
            ac_custom_field,
            client: super::http::shared_client(),
        }
    }
}

fn parse_ref(reference: &str) -> Result<&str, TicketSourceError> {
    let mut parts = reference.splitn(2, '-');
    let project = parts.next().unwrap_or("");
    let num = parts.next().unwrap_or("");
    if project.is_empty()
        || num.is_empty()
        || !project.chars().all(|c| c.is_ascii_uppercase())
        || !num.chars().all(|c| c.is_ascii_digit())
    {
        return Err(TicketSourceError::Parse {
            reason: format!("expected PROJECT-NUMBER, got: {reference}"),
        });
    }
    Ok(reference)
}

/// Recursively walk an ADF node tree and extract plain text.
fn adf_to_plain_text(node: &serde_json::Value) -> String {
    let mut out = String::new();
    walk_adf(node, &mut out);
    out
}

fn walk_adf(node: &serde_json::Value, out: &mut String) {
    if let Some(text) = node.get("text").and_then(|v| v.as_str()) {
        out.push_str(text);
    }
    if let Some(content) = node.get("content").and_then(|v| v.as_array()) {
        let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
        // Emit a markdown heading prefix so parse_acceptance_criteria can detect AC headings.
        if node_type == "heading" {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as usize;
            let hashes = "#".repeat(level.min(6));
            out.push_str(&hashes);
            out.push(' ');
        }
        for child in content {
            walk_adf(child, out);
        }
        // Add a newline after paragraph-like containers.
        if matches!(node_type, "paragraph" | "heading" | "listItem") {
            out.push('\n');
        }
    }
}

fn browse_url(base_url: &str, key: &str) -> String {
    let host = base_url
        .trim_end_matches("/rest/api/3")
        .trim_end_matches("/rest/api/2");
    format!("{host}/browse/{key}")
}

fn parse_jira_datetime(s: &str) -> i64 {
    // Jira format: "2026-04-24T10:00:00.000+0000"
    chrono::DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.3f%z")
        .map(|dt| dt.timestamp_millis())
        .unwrap_or_else(|_| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0)
        })
}

#[async_trait]
impl TicketSource for JiraTicketSource {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError> {
        if !matches!(reference.kind, TicketKind::Jira) {
            return Err(TicketSourceError::KindMismatch {
                expected: "JiraIssue",
                actual: reference.kind,
            });
        }
        let key = parse_ref(&reference.reference)?;

        let url = format!("{}/issue/{}", self.base_url, key);

        let auth_header = match &self.auth {
            JiraAuth::Basic { email, token } => {
                let raw = format!("{email}:{token}");
                let b64 = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
                format!("Basic {b64}")
            }
            JiraAuth::Bearer { token } => format!("Bearer {token}"),
        };

        let resp = self
            .client
            .get(&url)
            .header("Authorization", auth_header)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| TicketSourceError::Transport {
                source: Box::new(e),
            })?;

        match resp.status().as_u16() {
            200 => {}
            404 => {
                return Err(TicketSourceError::NotFound {
                    reference: reference.reference.clone(),
                });
            }
            401 | 403 => {
                return Err(TicketSourceError::Auth {
                    reason: format!("HTTP {}", resp.status()),
                });
            }
            other => {
                return Err(TicketSourceError::Transport {
                    source: format!("unexpected HTTP status: {other}").into(),
                });
            }
        }

        let raw: serde_json::Value = resp.json().await.map_err(|e| TicketSourceError::Parse {
            reason: format!("issue body json: {e}"),
        })?;

        let fields = raw.get("fields").ok_or_else(|| TicketSourceError::Parse {
            reason: "missing 'fields' in response".into(),
        })?;

        let title = fields
            .get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let body = fields
            .get("description")
            .map(adf_to_plain_text)
            .unwrap_or_default()
            .trim()
            .to_string();

        // AC: prefer custom field if configured; else parse from description body.
        let ac_field = if let Some(field_id) = &self.ac_custom_field {
            fields
                .get(field_id)
                .map(adf_to_plain_text)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            parse_acceptance_criteria(&body)
        };

        let comments = fields
            .get("comment")
            .and_then(|c| c.get("comments"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|c| TicketComment {
                        author: c
                            .get("author")
                            .and_then(|a| a.get("displayName"))
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        body: c.get("body").map(adf_to_plain_text).unwrap_or_default(),
                        created_at: c
                            .get("created")
                            .and_then(|s| s.as_str())
                            .map(parse_jira_datetime)
                            .unwrap_or(0),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let url_str = Some(browse_url(&self.base_url, key));

        Ok(Ticket {
            title,
            body,
            comments,
            ac_field,
            url: url_str,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_matches_known_inputs() {
        let enc = |s: &[u8]| base64::engine::general_purpose::STANDARD.encode(s);
        assert_eq!(
            enc(b"user@example.com:token"),
            "dXNlckBleGFtcGxlLmNvbTp0b2tlbg=="
        );
        assert_eq!(enc(b"hello"), "aGVsbG8=");
        assert_eq!(enc(b""), "");
    }

    #[test]
    fn adf_walker_extracts_text() {
        let doc = serde_json::json!({
            "type": "doc", "version": 1,
            "content": [
                {"type": "paragraph", "content": [{"type": "text", "text": "Hello"}]},
                {"type": "paragraph", "content": [{"type": "text", "text": "World"}]}
            ]
        });
        let text = adf_to_plain_text(&doc);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn parse_ref_accepts_valid_keys() {
        assert!(parse_ref("PROJ-1").is_ok());
        assert!(parse_ref("ABC-999").is_ok());
        assert!(parse_ref("XY-0").is_ok());
    }

    #[test]
    fn parse_ref_rejects_invalid_keys() {
        for bad in ["", "noproject", "lower-123", "PROJ-", "-123", "PROJ-abc"] {
            assert!(parse_ref(bad).is_err(), "should reject: {bad}");
        }
    }
}
