use std::path::PathBuf;

use async_trait::async_trait;

use super::cli::run_cli;
use super::{
    Ticket, TicketComment, TicketSource, TicketSourceError, github::parse_acceptance_criteria,
};
use crate::events::{TicketKind, TicketRef};

pub struct GitlabTicketSource {
    /// Path to the `glab` binary. Defaults to `"glab"` (resolved via PATH).
    /// Override via `with_binary_path` for tests using a fake-glab shell script.
    binary_path: PathBuf,
}

impl GitlabTicketSource {
    /// Create a source that uses `glab` resolved from `$PATH`.
    pub fn new() -> Self {
        Self {
            binary_path: PathBuf::from("glab"),
        }
    }

    /// Create a source that uses the given binary path (for testing).
    pub fn with_binary_path(path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: path.into(),
        }
    }
}

impl Default for GitlabTicketSource {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a `group/project#N` reference into `(project_path, iid)`.
fn parse_ref(reference: &str) -> Result<(&str, u64), TicketSourceError> {
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
    Ok((path, iid))
}

fn url_encode_path(path: &str) -> String {
    path.replace('/', "%2F")
}

/// Shape of `glab issue view <iid> --repo <path> --output json`
#[derive(serde::Deserialize)]
struct GlabIssueView {
    title: String,
    description: Option<String>,
    web_url: String,
}

/// Shape of `glab api /projects/<encoded>/issues/<iid>/notes`
#[derive(serde::Deserialize)]
struct GlabNote {
    body: String,
    author: GlabAuthor,
    created_at: String,
    #[serde(default)]
    system: bool,
}

#[derive(serde::Deserialize)]
struct GlabAuthor {
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
        let iid_str = iid.to_string();
        let encoded_path = url_encode_path(project_path);

        // Fetch issue via glab issue view
        let stdout = run_cli(
            &self.binary_path,
            &[
                "issue",
                "view",
                &iid_str,
                "--repo",
                project_path,
                "--output",
                "json",
            ],
            &reference.reference,
        )
        .await?;

        let issue: GlabIssueView =
            serde_json::from_str(&stdout).map_err(|e| TicketSourceError::Parse {
                reason: format!("glab issue view json: {e}"),
            })?;

        // Fetch notes (comments) via glab api
        let notes_path = format!("/projects/{encoded_path}/issues/{iid_str}/notes");
        let notes: Vec<TicketComment> = match run_cli(
            &self.binary_path,
            &["api", &notes_path],
            &reference.reference,
        )
        .await
        {
            Ok(out) => {
                let raw: Vec<GlabNote> = serde_json::from_str(&out).unwrap_or_default();
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
            Err(_) => Vec::new(),
        };

        let description = issue.description.unwrap_or_default();
        let ac_field = parse_acceptance_criteria(&description);

        Ok(Ticket {
            title: issue.title,
            body: description,
            comments: notes,
            ac_field,
            url: Some(issue.web_url),
        })
    }
}
