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
        match token.expires_at {
            None => false,
            Some(expires_at) => {
                let lead_ms = self.lead_time.as_millis() as i64;
                now_ms + lead_ms >= expires_at
            }
        }
    }

    /// Attempt one refresh via the strategy.
    ///
    /// Returns the new [`AccessToken`] on success, or
    /// [`AccountStatus::NeedsReauth`] on any failure. The caller is
    /// responsible for persisting the returned status.
    pub async fn refresh_once(&self, token: &AccessToken) -> Result<AccessToken, AccountStatus> {
        let refresh_token = match &token.refresh_token {
            Some(rt) => rt.as_str(),
            None => {
                return Err(AccountStatus::NeedsReauth {
                    reason: "no refresh_token available".to_string(),
                });
            }
        };

        match self.strategy.refresh(refresh_token).await {
            Ok(new_token) => Ok(new_token),
            Err(RefreshError::Rejected { error, description }) => Err(AccountStatus::NeedsReauth {
                reason: format!("provider rejected: {error} — {description}"),
            }),
            Err(RefreshError::Transport(msg)) | Err(RefreshError::Parse(msg)) => {
                // Transport/parse errors are usually transient — for MVP we
                // surface them as needs_reauth. A future retry policy would
                // distinguish transient from permanent failures.
                Err(AccountStatus::NeedsReauth { reason: msg })
            }
        }
    }
}

// ── GitHub refresh strategy ──────────────────────────────────────────────────

/// Refresh strategy for GitHub OAuth apps with token expiration enabled.
///
/// NOTE: `GithubRefreshStrategy::refresh` duplicates some body-parsing logic
/// from `oauth_github::GithubOauthClient::exchange_code`. This is acceptable
/// for now — factoring it into a shared helper is tracked as issue #51.
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
        let url = format!("{}/login/oauth/access_token", self.base_url);

        let mut form = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
        ];
        if let Some(secret) = &self.client_secret {
            form.push(("client_secret", secret.as_str()));
        }

        let resp = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|e| RefreshError::Transport(e.to_string()))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| RefreshError::Parse(format!("response body json: {e}")))?;

        // GitHub returns OAuth errors with HTTP 200 + `error` field.
        if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
            let description = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(RefreshError::Rejected {
                error: err.to_string(),
                description,
            });
        }

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RefreshError::Parse("missing access_token".into()))?
            .to_string();

        let token_type = body
            .get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bearer")
            .to_string();

        let scopes_raw = body.get("scope").and_then(|v| v.as_str()).unwrap_or("");
        let scopes: Vec<String> = scopes_raw
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        let new_refresh_token = body
            .get("refresh_token")
            .and_then(|v| v.as_str())
            .map(String::from);

        let expires_in_secs = body.get("expires_in").and_then(|v| v.as_i64());
        let expires_at = expires_in_secs.map(|secs| {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            now_ms + (secs * 1000)
        });

        Ok(AccessToken {
            token,
            refresh_token: new_refresh_token,
            expires_at,
            token_type,
            scopes,
        })
    }
}
