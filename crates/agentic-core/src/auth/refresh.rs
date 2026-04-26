use std::time::Duration;

use crate::auth::AccessToken;

/// Strategy for refreshing a token via a provider's refresh endpoint.
///
/// Implementations call the provider's POST endpoint with the refresh token
/// and return a new [`AccessToken`]. Used by [`RefreshScheduler`] to perform
/// the actual refresh — the scheduler handles timing and state, the strategy
/// handles transport.
#[async_trait::async_trait]
pub trait RefreshStrategy: Send + Sync {
    async fn refresh(&self, refresh_token: &str) -> Result<AccessToken, RefreshError>;
}

/// Errors produced by a [`RefreshStrategy`].
#[derive(Debug, thiserror::Error)]
pub enum RefreshError {
    #[error("provider rejected refresh: {error}: {description}")]
    Rejected { error: String, description: String },
    #[error("transport: {0}")]
    Transport(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// Account status after a refresh attempt.
///
/// The `reason` field is a human-readable diagnostic string and MUST NOT be
/// parsed programmatically — treat it as display-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    /// Token is current and valid.
    Active,
    /// Refresh failed. User must re-authenticate.
    NeedsReauth { reason: String },
}

/// Synchronous building blocks for token refresh scheduling.
///
/// The actual background loop (tokio::spawn + sleep + wake) is deferred to
/// Phase 15 (UI auth integration) when concrete consumers exist.
pub struct RefreshScheduler<S: RefreshStrategy> {
    strategy: S,
    /// How long before `token_expires_at` to refresh. Default 5 minutes.
    lead_time: Duration,
}

impl<S: RefreshStrategy> RefreshScheduler<S> {
    pub fn new(strategy: S) -> Self {
        Self {
            strategy,
            lead_time: Duration::from_secs(300),
        }
    }

    pub fn with_lead_time(mut self, lead_time: Duration) -> Self {
        self.lead_time = lead_time;
        self
    }

    /// Returns `true` if `now_ms + lead_time >= expires_at`.
    ///
    /// Tokens without an expiry never need refreshing.
    pub fn should_refresh(&self, token: &AccessToken, now_ms: i64) -> bool {
        todo!()
    }

    /// Attempt one refresh via the strategy.
    ///
    /// Returns the new [`AccessToken`] on success, or
    /// [`AccountStatus::NeedsReauth`] on any failure. The caller is
    /// responsible for persisting the returned status.
    pub async fn refresh_once(
        &self,
        token: &AccessToken,
    ) -> Result<AccessToken, AccountStatus> {
        todo!()
    }
}

// ── GitHub refresh strategy ──────────────────────────────────────────────────

/// Refresh strategy for GitHub OAuth apps with token expiration enabled.
pub struct GithubRefreshStrategy {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    client: reqwest::Client,
}

impl GithubRefreshStrategy {
    pub fn new(
        base_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: Option<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret,
            client: crate::ticket_sources::http::shared_client(),
        }
    }

    /// Convenience constructor targeting `github.com`.
    pub fn github_com(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        Self::new("https://github.com", client_id, client_secret)
    }
}

#[async_trait::async_trait]
impl RefreshStrategy for GithubRefreshStrategy {
    async fn refresh(&self, refresh_token: &str) -> Result<AccessToken, RefreshError> {
        todo!()
    }
}
