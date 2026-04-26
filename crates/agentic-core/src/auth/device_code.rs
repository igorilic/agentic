use std::time::Duration;

use crate::auth::AccessToken;
use crate::ticket_sources::http::shared_client;

/// Information returned from the device authorization endpoint (RFC 8628 §3.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorization {
    /// Secret device code sent to the token endpoint during polling.
    pub device_code: String,
    /// Short user-facing code (e.g. "WDJB-MJHT"). Display alongside `verification_uri`.
    pub user_code: String,
    /// URL where the user opens a browser to complete authorization.
    pub verification_uri: String,
    /// Optional URI with the `user_code` pre-embedded (e.g. for QR codes).
    pub verification_uri_complete: Option<String>,
    /// Seconds until `device_code` expires.
    pub expires_in: u64,
    /// Minimum seconds the client must wait between token-endpoint polls.
    pub interval: u64,
}

/// Errors from the device code flow.
#[derive(Debug, thiserror::Error)]
pub enum DeviceCodeError {
    #[error("oauth error from server: {error}: {description}")]
    OauthError { error: String, description: String },
    #[error("user denied authorization")]
    AccessDenied,
    #[error("device_code expired before user completed authorization")]
    Expired,
    #[error("transport: {0}")]
    Transport(String),
    #[error("parse: {0}")]
    Parse(String),
}

/// Provider-agnostic OAuth device code flow client (RFC 8628).
pub struct DeviceCodeClient {
    /// Device authorization endpoint URL.
    pub device_authorization_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// OAuth client_id.
    pub client_id: String,
    client: reqwest::Client,
}

impl DeviceCodeClient {
    /// Create a new client with explicit endpoint URLs.
    pub fn new(
        device_authorization_url: impl Into<String>,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            device_authorization_url: device_authorization_url.into(),
            token_url: token_url.into(),
            client_id: client_id.into(),
            client: shared_client(),
        }
    }

    /// Convenience: GitHub.com device flow endpoints.
    pub fn github_com(client_id: impl Into<String>) -> Self {
        Self::new(
            "https://github.com/login/device/code",
            "https://github.com/login/oauth/access_token",
            client_id,
        )
    }

    /// Convenience: GitLab.com device flow endpoints.
    pub fn gitlab_com(client_id: impl Into<String>) -> Self {
        Self::new(
            "https://gitlab.com/oauth/authorize_device",
            "https://gitlab.com/oauth/token",
            client_id,
        )
    }

    /// Step 1: Request device + user codes from the device authorization endpoint.
    ///
    /// `scopes` is the set of OAuth scopes to request (space-joined for the request body).
    pub async fn request_device_code(
        &self,
        scopes: &[&str],
    ) -> Result<DeviceAuthorization, DeviceCodeError> {
        let scope_str = scopes.join(" ");
        let mut form = vec![("client_id", self.client_id.as_str())];
        if !scope_str.is_empty() {
            form.push(("scope", scope_str.as_str()));
        }

        let resp = self
            .client
            .post(&self.device_authorization_url)
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|e| DeviceCodeError::Transport(e.to_string()))?;

        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| DeviceCodeError::Parse(format!("device code response: {e}")))?;

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
            return Err(DeviceCodeError::OauthError {
                error: err,
                description,
            });
        }

        Ok(DeviceAuthorization {
            device_code: body
                .get("device_code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DeviceCodeError::Parse("missing device_code".into()))?
                .to_string(),
            user_code: body
                .get("user_code")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DeviceCodeError::Parse("missing user_code".into()))?
                .to_string(),
            verification_uri: body
                .get("verification_uri")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DeviceCodeError::Parse("missing verification_uri".into()))?
                .to_string(),
            verification_uri_complete: body
                .get("verification_uri_complete")
                .and_then(|v| v.as_str())
                .map(String::from),
            expires_in: body
                .get("expires_in")
                .and_then(|v| v.as_u64())
                .unwrap_or(900),
            interval: body.get("interval").and_then(|v| v.as_u64()).unwrap_or(5),
        })
    }

    /// Step 2: Poll the token endpoint at `initial_interval` (with `slow_down` backoff)
    /// until success, denial, or expiration.
    ///
    /// Per RFC 8628 §3.5, `slow_down` bumps the interval by +5 seconds. The `initial_interval`
    /// parameter takes a `Duration` (rather than integer seconds) to allow tests to use
    /// sub-second intervals without paused time.
    pub async fn poll_for_token(
        &self,
        device_code: &str,
        initial_interval: Duration,
    ) -> Result<AccessToken, DeviceCodeError> {
        let mut interval = initial_interval;

        loop {
            // Wait BEFORE polling (RFC 8628 §3.4 — first poll is also after `interval`).
            tokio::time::sleep(interval).await;

            let form = vec![
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", device_code),
                ("client_id", self.client_id.as_str()),
            ];

            let resp = self
                .client
                .post(&self.token_url)
                .header("Accept", "application/json")
                .form(&form)
                .send()
                .await
                .map_err(|e| DeviceCodeError::Transport(e.to_string()))?;

            let status = resp.status();
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| DeviceCodeError::Parse(format!("poll response: {e}")))?;

            // Both providers return error codes either via HTTP 4xx or via `error` field
            // on HTTP 200. Inspect body's `error` field.
            if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
                match err {
                    "authorization_pending" => continue,
                    "slow_down" => {
                        interval += Duration::from_secs(5); // RFC 8628 §3.5: bump by 5s.
                        continue;
                    }
                    "access_denied" => return Err(DeviceCodeError::AccessDenied),
                    "expired_token" => return Err(DeviceCodeError::Expired),
                    other => {
                        let description = body
                            .get("error_description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        return Err(DeviceCodeError::OauthError {
                            error: other.to_string(),
                            description,
                        });
                    }
                }
            }

            if !status.is_success() {
                return Err(DeviceCodeError::Transport(format!("HTTP {status}")));
            }

            let token = body
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DeviceCodeError::Parse("missing access_token".into()))?
                .to_string();

            let token_type = body
                .get("token_type")
                .and_then(|v| v.as_str())
                .unwrap_or("bearer")
                .to_string();

            let scopes_raw = body.get("scope").and_then(|v| v.as_str()).unwrap_or("");
            let scopes: Vec<String> = if scopes_raw.contains(',') {
                scopes_raw
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            } else {
                scopes_raw
                    .split(' ')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect()
            };

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

            return Ok(AccessToken {
                token,
                refresh_token,
                expires_at,
                token_type,
                scopes,
            });
        }
    }
}
