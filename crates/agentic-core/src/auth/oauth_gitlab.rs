use crate::auth::oauth_github::AccessToken;
use crate::ticket_sources::http::shared_client;

// Re-export the provider-agnostic state validator so callers don't need to
// import from the github module.
pub use super::oauth_github::validate_state;

/// Errors produced by the GitLab OAuth client.
#[derive(Debug, thiserror::Error)]
pub enum GitlabOauthError {
    #[error("oauth error from gitlab: {error}: {description}")]
    OauthError { error: String, description: String },
    #[error("transport: {0}")]
    Transport(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// HTTP client for the GitLab OAuth token endpoint.
///
/// Supports both gitlab.com and self-hosted instances via `base_url`.
pub struct GitlabOauthClient {
    pub base_url: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    client: reqwest::Client,
}

impl GitlabOauthClient {
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

    /// gitlab.com convenience constructor.
    pub fn gitlab_com(client_id: impl Into<String>, client_secret: Option<String>) -> Self {
        Self::new("https://gitlab.com", client_id, client_secret)
    }

    /// Exchange an authorization code + PKCE verifier for an access token.
    ///
    /// Caller MUST validate the `state` parameter via `validate_state` before
    /// calling this, to defend against CSRF.
    pub async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
        redirect_uri: &str,
    ) -> Result<AccessToken, GitlabOauthError> {
        let url = format!("{}/oauth/token", self.base_url);

        let mut form = vec![
            ("grant_type", "authorization_code"),
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
            .map_err(|e| GitlabOauthError::Transport(e.to_string()))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GitlabOauthError::Parse(format!("response body json: {e}")))?;

        // GitLab returns OAuth errors with HTTP 4xx (proper RFC behaviour,
        // unlike GitHub which returns 200 with an error field).
        if !status.is_success() {
            let err = body
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown_error")
                .to_string();
            let description = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(GitlabOauthError::OauthError {
                error: err,
                description,
            });
        }

        // Defensive: even on 200, surface an error field if present.
        if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
            let description = body
                .get("error_description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(GitlabOauthError::OauthError {
                error: err.to_string(),
                description,
            });
        }

        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| GitlabOauthError::Parse("missing access_token".into()))?
            .to_string();

        let token_type = body
            .get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("bearer")
            .to_string();

        // GitLab uses SPACE separator (RFC 6749), unlike GitHub's comma.
        let scopes_raw = body.get("scope").and_then(|v| v.as_str()).unwrap_or("");
        let scopes: Vec<String> = scopes_raw
            .split(' ')
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
