use async_trait::async_trait;

use crate::events::TicketRef;

mod free_text;
pub use free_text::FreeTextTicketSource;

pub(crate) mod http;

pub(crate) mod cli;

mod github;
pub use github::GithubTicketSource;

mod jira;
pub use jira::{JiraAuth, JiraTicketSource};

/// A fully-fetched ticket with body, comments, optional AC field, and URL.
/// `TicketRef` (in events) is the lightweight pointer; `Ticket` is the
/// content the pipeline consumes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    /// Display title.
    pub title: String,
    /// Main body / description (markdown).
    pub body: String,
    /// Threaded comments in chronological order.
    pub comments: Vec<TicketComment>,
    /// Acceptance criteria parsed from a section in `body` (or a custom
    /// field for Jira). `None` if not present; downstream agents can
    /// fall back to `body`.
    pub ac_field: Option<String>,
    /// Canonical URL for human reference (None for FreeText).
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketComment {
    pub author: String,
    pub body: String,
    /// Unix epoch milliseconds.
    pub created_at: i64,
}

/// Errors specific to ticket source operations.
#[derive(Debug, thiserror::Error)]
pub enum TicketSourceError {
    #[error("ticket not found: {reference}")]
    NotFound { reference: String },
    #[error("authentication failed: {reason}")]
    Auth { reason: String },
    #[error("kind mismatch: source supports {expected}, got {actual:?}")]
    KindMismatch {
        expected: &'static str,
        actual: crate::events::TicketKind,
    },
    #[error("transport error: {source}")]
    Transport {
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("parse error: {reason}")]
    Parse { reason: String },
}

#[async_trait]
pub trait TicketSource: Send + Sync {
    async fn fetch(&self, reference: &TicketRef) -> Result<Ticket, TicketSourceError>;
}
