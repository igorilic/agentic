use async_trait::async_trait;

use super::{
    Ticket, TicketComment, TicketSource, TicketSourceError, github::parse_acceptance_criteria,
};
use crate::events::{TicketKind, TicketRef};

#[derive(Debug, Clone, Copy)]
pub enum GitlabAuth {
    PrivateToken,
    Bearer,
}

pub struct GitlabTicketSource {
    /// Base URL. Defaults to "https://gitlab.com/api/v4".
    /// For self-hosted: "https://gitlab.example.com/api/v4".
    base_url: String,
    /// PAT or OAuth token. Empty disables auth (limited to public projects).
    token: String,
    /// Whether `token` is a PAT (PRIVATE-TOKEN header) or OAuth (Bearer).
    auth_kind: GitlabAuth,
    client: reqwest::Client,
}

impl GitlabTicketSource {
    pub fn new(
        base_url: impl Into<String>,
        token: impl Into<String>,
        auth_kind: GitlabAuth,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            auth_kind,
            client: super::http::shared_client(),
        }
    }

    /// Convenience: gitlab.com with the given token.
    pub fn gitlab_com(token: impl Into<String>, auth_kind: GitlabAuth) -> Self {
        Self::new("https://gitlab.com/api/v4", token, auth_kind)
    }

    async fn send_authed(&self, url: &str) -> Result<reqwest::Response, TicketSourceError> {
        let mut req = self.client.get(url);
        if !self.token.is_empty() {
            req = match self.auth_kind {
                GitlabAuth::PrivateToken => req.header("PRIVATE-TOKEN", &self.token),
                GitlabAuth::Bearer => req.bearer_auth(&self.token),
            };
        }
        req.send().await.map_err(|e| TicketSourceError::Transport {
            source: Box::new(e),
        })
    }
}

fn parse_ref(reference: &str) -> Result<(String, u64), TicketSourceError> {
    let (path, num_str) = reference
        .rsplit_once('#')
        .ok_or_else(|| TicketSourceError::Parse {
            reason: format!("expected group/project#N, got: {reference}"),
        })?;
    if !path.contains('/') {
        return Err(TicketSourceError::Parse {
            reason: format!("expected group/project#N, got: {reference}"),
        });
    }
    let iid: u64 = num_str.parse().map_err(|_| TicketSourceError::Parse {
        reason: format!("expected numeric iid, got: {num_str}"),
    })?;
    Ok((path.to_string(), iid))
}

fn url_encode_path(path: &str) -> String {
    path.replace('/', "%2F")
}

#[derive(serde::Deserialize)]
struct GitlabIssueResponse {
    title: String,
    /// `description` may be null for empty issues; defaults to "".
    description: Option<String>,
    web_url: String,
}

#[derive(serde::Deserialize)]
struct GitlabNoteResponse {
    body: String,
    author: GitlabAuthor,
    created_at: String,
    #[serde(default)]
    system: bool,
}

#[derive(serde::Deserialize)]
struct GitlabAuthor {
    username: String,
}

#[async_trait]
impl TicketSource for GitlabTicketSource {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError> {
        if !matches!(reference.kind, TicketKind::GitlabIssue) {
            return Err(TicketSourceError::KindMismatch {
                expected: "GitlabIssue",
                actual: reference.kind,
            });
        }
        let (project_path, iid) = parse_ref(&reference.reference)?;
        let project_id = url_encode_path(&project_path);

        let issue_url = format!("{}/projects/{}/issues/{}", self.base_url, project_id, iid);
        let resp = self.send_authed(&issue_url).await?;

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

        let issue: GitlabIssueResponse =
            resp.json().await.map_err(|e| TicketSourceError::Parse {
                reason: format!("issue body json: {e}"),
            })?;

        let notes_url = format!(
            "{}/projects/{}/issues/{}/notes",
            self.base_url, project_id, iid
        );
        let comments: Vec<TicketComment> = match self.send_authed(&notes_url).await {
            Ok(r) if r.status().is_success() => {
                let raw: Vec<GitlabNoteResponse> = r.json().await.unwrap_or_default();
                raw.into_iter()
                    .filter(|n| !n.system)
                    .map(|n| TicketComment {
                        author: n.author.username,
                        body: n.body,
                        created_at: chrono::DateTime::parse_from_rfc3339(&n.created_at)
                            .map(|d| d.timestamp_millis())
                            .unwrap_or(0),
                    })
                    .collect()
            }
            _ => Vec::new(),
        };

        let description = issue.description.unwrap_or_default();
        let ac_field = parse_acceptance_criteria(&description);

        Ok(Ticket {
            title: issue.title,
            body: description,
            comments,
            ac_field,
            url: Some(issue.web_url),
        })
    }
}
