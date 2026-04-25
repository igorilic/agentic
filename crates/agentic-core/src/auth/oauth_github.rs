use crate::ticket_sources::http::shared_client;

/// An OAuth access token returned after a successful code exchange.
#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken {
    /// The access token (Bearer). SHORT-LIVED SECRET.
    pub token: String,
    /// Optional refresh token. GitHub returns one when the app is configured
    /// for token expiration; otherwise it is None.
    pub refresh_token: Option<String>,
    /// Unix epoch ms when `token` expires. None for non-expiring tokens.
    pub expires_at: Option<i64>,
    /// Token type — typically "bearer".
    pub token_type: String,
    /// Granted scopes (comma-separated in GitHub's response — split into Vec).
    pub scopes: Vec<String>,
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccessToken")
            .field("token", &"[redacted]")
            .field("refresh_token", &self.refresh_token.as_ref().map(|_| "[redacted]"))
            .field("expires_at", &self.expires_at)
            .field("token_type", &self.token_type)
            .field("scopes", &self.scopes)
            .finish()
    }
}

/// Errors produced by the GitHub OAuth client.
#[derive(Debug, thiserror::Error)]
pub enum GithubOauthError {
    #[error("state mismatch: expected {expected}, got {actual}")]
    StateMismatch { expected: String, actual: String },
    #[error("oauth error from github: {error}: {description}")]
    OauthError { error: String, description: String },
    #[error("transport: {0}")]
    Transport(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// Validate that the callback's `state` matches the value we generated.
/// Returns `Err(StateMismatch)` if not equal — caller MUST call this before
/// `exchange_code` to defend against CSRF.
pub fn validate_state(expected: &str, actual: &str) -> Result<(), GithubOauthError> {
    todo!()
}

/// HTTP client for the GitHub OAuth token endpoint.
pub struct GithubOauthClient {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    client: reqwest::Client,
}

impl GithubOauthClient {
    pub fn new(
        base_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: Option<String>,
    ) -> Self {
        todo!()
    }

    /// github.com convenience constructor.
    pub fn github_com(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        todo!()
    }

    /// Exchange an authorization code + PKCE verifier for an access token.
    /// Caller MUST have validated `callback.state` against the expected value
    /// via `validate_state` before calling this.
    pub async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> Result<AccessToken, GithubOauthError> {
        todo!()
    }
}
