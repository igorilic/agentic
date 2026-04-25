use async_trait::async_trait;
use reqwest::Client;

use super::{Ticket, TicketComment, TicketSource, TicketSourceError};
use crate::events::{TicketKind, TicketRef};

pub struct GithubTicketSource {
    /// Base URL. Defaults to "https://api.github.com" for github.com.
    /// For GHES set to "https://ghes.example.com/api/v3".
    base_url: String,
    /// Personal access token. Empty string disables auth (anonymous,
    /// limited to public repos).
    token: String,
    client: Client,
}

impl GithubTicketSource {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            client: super::http::shared_client(),
        }
    }

    /// Convenience: github.com with the given token.
    pub fn github_com(token: impl Into<String>) -> Self {
        Self::new("https://api.github.com", token)
    }
}

fn parse_ref(reference: &str) -> Result<(String, String, u64), TicketSourceError> {
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
    Ok((owner.to_string(), repo.to_string(), num))
}

fn parse_acceptance_criteria(body: &str) -> Option<String> {
    let lines = body.lines();
    let mut found = false;
    let mut buf = String::new();
    for line in lines {
        if !found {
            if line.trim_start().starts_with("##")
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

fn parse_iso8601(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(0)
}

#[derive(serde::Deserialize)]
struct GithubIssueResponse {
    title: String,
    body: String,
    html_url: String,
}

#[derive(serde::Deserialize)]
struct GithubCommentResponse {
    user: GithubUser,
    body: String,
    created_at: String,
}

#[derive(serde::Deserialize)]
struct GithubUser {
    login: String,
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

        let url = format!("{}/repos/{}/{}/issues/{}", self.base_url, owner, repo, num);
        let mut req = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github+json");
        if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }
        let resp = req.send().await.map_err(|e| TicketSourceError::Transport {
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

        let issue: GithubIssueResponse =
            resp.json().await.map_err(|e| TicketSourceError::Parse {
                reason: format!("issue body json: {e}"),
            })?;

        let comments_url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, num
        );
        let mut comments_req = self
            .client
            .get(&comments_url)
            .header("Accept", "application/vnd.github+json");
        if !self.token.is_empty() {
            comments_req = comments_req.bearer_auth(&self.token);
        }
        let comments: Vec<TicketComment> = match comments_req.send().await {
            Ok(r) if r.status().is_success() => {
                let raw: Vec<GithubCommentResponse> = r.json().await.unwrap_or_default();
                raw.into_iter()
                    .map(|c| TicketComment {
                        author: c.user.login,
                        body: c.body,
                        created_at: parse_iso8601(&c.created_at),
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        let ac_field = parse_acceptance_criteria(&issue.body);

        Ok(Ticket {
            title: issue.title,
            body: issue.body,
            comments,
            ac_field,
            url: Some(issue.html_url),
        })
    }
}
