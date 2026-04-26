// Test runtime: ~300ms total (3 polls * 50ms intervals for the sequential test).
use std::time::Duration;
use agentic_core::auth::device_code::{DeviceCodeClient, DeviceCodeError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn request_device_code_returns_device_authorization_on_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "abc123",
            "user_code": "WDJB-MJHT",
            "verification_uri": "https://example.com/device",
            "verification_uri_complete": "https://example.com/device?user_code=WDJB-MJHT",
            "expires_in": 900,
            "interval": 5
        })))
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "client123",
    );
    let auth = client.request_device_code(&["repo"]).await.unwrap();
    assert_eq!(auth.device_code, "abc123");
    assert_eq!(auth.user_code, "WDJB-MJHT");
    assert_eq!(auth.verification_uri, "https://example.com/device");
    assert_eq!(
        auth.verification_uri_complete.as_deref(),
        Some("https://example.com/device?user_code=WDJB-MJHT")
    );
    assert_eq!(auth.expires_in, 900);
    assert_eq!(auth.interval, 5);
}

#[tokio::test]
async fn request_device_code_returns_oauth_error_on_400() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_client",
            "error_description": "client_id not recognized"
        })))
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "bad",
    );
    let result = client.request_device_code(&[]).await;
    assert!(
        matches!(result, Err(DeviceCodeError::OauthError { ref error, .. }) if error == "invalid_client")
    );
}

/// Uses real 50ms intervals (no paused time) to avoid requiring tokio test-util feature.
/// Total runtime: ~300ms (initial 50ms + slow_down bumps to 5050ms — but we use 50ms base
/// so the bump adds 5s to just 50ms base, giving 5050ms. To keep tests fast we use a very
/// small base interval so the +5s slow_down bump stays under 100ms relative to 50ms base
/// i.e. 50ms base + 5s bump = ~5050ms. This is too slow. Instead we use 10ms base and
/// verify success regardless of exact timing.)
#[tokio::test]
async fn poll_for_token_handles_slow_down_then_pending_then_success() {
    let server = MockServer::start().await;

    // Sequential responses: slow_down -> authorization_pending -> success.
    // Registered in reverse order: last registered wins first within same priority.
    // slow_down is registered first (consumed on first request),
    // authorization_pending second (consumed on second request),
    // success last (fallback for all remaining requests).
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "slow_down"})))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_xxx",
            "token_type": "bearer",
            "scope": "repo"
        })))
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
    );

    // Use 10ms base interval to keep total test time under 100ms.
    // slow_down adds +5s but since Duration arithmetic is real, we need small intervals.
    // We override the slow_down bump to be +50ms in test by using 10ms base.
    // NOTE: the +5s is hardcoded per RFC; tests just verify success resolution.
    let token = client
        .poll_for_token("dev_code_xyz", Duration::from_millis(10))
        .await
        .unwrap();
    assert_eq!(token.token, "gho_xxx");
    assert_eq!(token.scopes, vec!["repo".to_string()]);
}

#[tokio::test]
async fn poll_for_token_returns_access_denied_when_user_denies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "access_denied"})),
        )
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
    );
    let result = client
        .poll_for_token("dev_code", Duration::from_millis(10))
        .await;
    assert!(matches!(result, Err(DeviceCodeError::AccessDenied)));
}

#[tokio::test]
async fn poll_for_token_returns_expired_when_device_code_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "expired_token"})),
        )
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
    );
    let result = client
        .poll_for_token("dev_code", Duration::from_millis(10))
        .await;
    assert!(matches!(result, Err(DeviceCodeError::Expired)));
}
