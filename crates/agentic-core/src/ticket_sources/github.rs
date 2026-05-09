use std::path::PathBuf;

use async_trait::async_trait;

use super::{Ticket, TicketComment, TicketSource, TicketSourceError, cli::run_cli};
use crate::events::{TicketKind, TicketRef};

pub struct GithubTicketSource {
    /// Path to the `gh` binary. Defaults to `"gh"` (resolved via PATH).
    /// Override via `with_binary_path` for tests using a fake-gh shell script.
    binary_path: PathBuf,
}

impl GithubTicketSource {
    /// Create a source that uses `gh` resolved from `$PATH`.
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("gh"),
        }
    }

    /// Create a source that uses the given binary path (for testing).
    pub fn with_binary_path(path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: path.into(),
        }
    }
}

impl Default for GithubTicketSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a `owner/repo#N` reference into `(owner, repo, number)`.
fn parse_ref(reference: &str) -> Result<(&str, &str, u64), TicketSourceError> {
    let (owner_repo, num_str) =
        reference
            .rsplit_once('#')
            .ok_or_else(|| TicketSourceError::Parse {
                reason: format!("expected owner/repo#N, got: {reference}"),
            })?;
    let (owner, repo) = owner_repo
        .split_once('/')
        .ok_or_else(|| TicketSourceError::Parse {
            reason: format!("expected owner/repo#N, got: {reference}"),
        })?;
    let num: u64 = num_str.parse().map_err(|_| TicketSourceError::Parse {
        reason: format!("expected numeric issue id, got: {num_str}"),
    })?;
    Ok((owner, repo, num))
}

pub(crate) fn parse_acceptance_criteria(body: &str) -> Option<String> {
    let lines = body.lines();
    let mut found = false;
    let mut buf = String::new();
    for line in lines {
        if !found {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#')
                && line
                    .trim_start_matches('#')
                    .trim()
                    .eq_ignore_ascii_case("Acceptance Criteria")
            {
                found = true;
            }
            continue;
        }
        let t = line.trim_start();
        if t.starts_with("## ") || t.starts_with("# ") {
            break;
        }
        buf.push_str(line);
        buf.push('\n');
    }
    if found && !buf.trim().is_empty() {
        Some(buf.trim().to_string())
    } else {
        None
    }
}

/// Shape of `gh issue view --json title,body,labels,state,url,comments`
#[derive(serde::Deserialize)]
struct GhIssueView {
    title: String,
    body: Option<String>,
    url: Option<String>,
    #[serde(default)]
    comments: Vec<GhComment>,
}

#[derive(serde::Deserialize)]
struct GhComment {
    author: GhAuthor,
    body: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(serde::Deserialize)]
struct GhAuthor {
    login: String,
}

fn parse_iso8601(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

#[async_trait]
impl TicketSource for GithubTicketSource {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError> {
        if !matches!(reference.kind, TicketKind::GithubIssue) {
            return Err(TicketSourceError::KindMismatch {
                expected: "GithubIssue",
                actual: reference.kind,
            });
        }
        let (owner, repo, num) = parse_ref(&reference.reference)?;
        let owner_repo = format!("{owner}/{repo}");
        let num_str = num.to_string();

        let stdout = run_cli(
            &self.binary_path,
            &[
                "issue",
                "view",
                &num_str,
                "--repo",
                &owner_repo,
                "--json",
                "title,body,labels,state,url,comments",
            ],
            &reference.reference,
        )
        .await?;

        let issue: GhIssueView =
            serde_json::from_str(&stdout).map_err(|e| TicketSourceError::Parse {
                reason: format!("gh issue view json: {e}"),
            })?;

        let body = issue.body.unwrap_or_default();
        let ac_field = parse_acceptance_criteria(&body);
        let comments = issue
            .comments
            .into_iter()
            .map(|c| TicketComment {
                author: c.author.login,
                body: c.body,
                created_at: parse_iso8601(&c.created_at),
            })
            .collect();

        Ok(Ticket {
            title: issue.title,
            body,
            comments,
            ac_field,
            url: issue.url,
        })
    }
}
