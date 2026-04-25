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
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[redacted]"),
            )
            .field("expires_at", &self.expires_at)
            .field("token_type", &self.token_type)
            .field("scopes", &self.scopes)
            .finish()
    }
}

/// Errors produced by the GitHub OAuth client.
#[derive(Debug, thiserror::Error)]
pub enum GithubOauthError {
    #[error("state mismatch — values do not match (both redacted)")]
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
///
/// Uses constant-time comparison to avoid timing-based state oracle attacks.
pub fn validate_state(expected: &str, actual: &str) -> Result<(), GithubOauthError> {
    // Constant-time compare: both slices must have the same length and contents.
    // We do this manually to avoid adding a new workspace dependency.
    let exp_bytes = expected.as_bytes();
    let act_bytes = actual.as_bytes();

    // Lengths must match first (XOR would give false positive if one is a prefix).
    let len_ok = exp_bytes.len() == act_bytes.len();

    // Accumulate XOR differences without short-circuiting.
    let mut diff: u8 = 0;
    let min_len = exp_bytes.len().min(act_bytes.len());
    for i in 0..min_len {
        diff |= exp_bytes[i] ^ act_bytes[i];
    }

    if len_ok && diff == 0 {
        Ok(())
    } else {
        Err(GithubOauthError::StateMismatch {
            expected: "[redacted]".to_string(),
            actual: "[redacted]".to_string(),
        })
    }
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
        Self {
            base_url: base_url.into(),
            client_id: client_id.into(),
            client_secret,
            client: shared_client(),
        }
    }

    /// github.com convenience constructor.
    pub fn github_com(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        Self::new("https://github.com", client_id, client_secret)
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
        let url = format!("{}/login/oauth/access_token", self.base_url);

        let mut form = vec![
            ("client_id", self.client_id.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
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
            .map_err(|e| GithubOauthError::Transport(e.to_string()))?;

        // GitHub returns 200 even for OAuth errors — must inspect body.
        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GithubOauthError::Parse(format!("response body json: {e}")))?;

        if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
            let description = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(GithubOauthError::OauthError {
                error: err.to_string(),
                description,
            });
        }

        if !status.is_success() {
            return Err(GithubOauthError::Transport(format!("HTTP {status}")));
        }

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GithubOauthError::Parse("missing access_token".into()))?
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

        let refresh_token = body
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
            refresh_token,
            expires_at,
            token_type,
            scopes,
        })
    }
}
