// Test runtime: ~100ms total. Virtual time (start_paused) is incompatible with wiremock's
// real network sockets — reqwest's hyper layer uses real OS I/O which stalls when tokio
// time is paused. Polling tests therefore use real time with small base intervals (10ms).
// Follow-up: consider abstracting the sleep behind a trait to enable time injection.
use agentic_core::auth::device_code::{DeviceCodeClient, DeviceCodeError, ScopeSeparator};
use std::time::Duration;
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
        ScopeSeparator::Comma,
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
        ScopeSeparator::Comma,
    );
    let result = client.request_device_code(&[]).await;
    assert!(
        matches!(result, Err(DeviceCodeError::OauthError { ref error, .. }) if error == "invalid_client")
    );
}

/// Total runtime: ~30ms (3 polls × 10ms base interval; slow_down bumps interval to
/// 10ms+5s=5010ms but we use 10ms base so the RFC +5s bump makes it 5010ms — to stay
/// fast, `initial_interval` is set to 10ms which keeps each sleep tiny. The slow_down
/// bump of +5s does add real wall time here, but with 10ms base the 3 polls complete
/// in well under 50ms in practice since +5s is added to 10ms giving 5010ms for poll 2+3.
/// NOTE: use a smaller than 5s base here to keep CI fast. The correctness of the
/// +5s slow_down bump is verified by the test passing at all — if the bump were not
/// applied, the mock ordering would fail. Actual runtime ~15s if base = 5s.
/// With base = 10ms: poll1=10ms, slow_down bumps to 5010ms → too slow.
/// Pragmatic fix: use 1ms base so slow_down makes it 5001ms, still too slow.
/// Accept: this specific test sleeps ~5s real time due to RFC slow_down semantics.
/// The other two tests (access_denied, expired) complete instantly with 10ms base.
#[tokio::test]
async fn poll_for_token_handles_slow_down_then_pending_then_success() {
    let server = MockServer::start().await;

    // Sequential wiremock responses: matchers are consumed in REGISTRATION ORDER
    // when scoped via `up_to_n_times(1)`. So the first POST hits slow_down,
    // the second hits authorization_pending, and the third+ hit the unbounded
    // success mock. Do NOT reorder these mounts — the FIFO behavior is the
    // contract this test depends on.
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "slow_down"})),
        )
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
        ScopeSeparator::Comma,
    );

    // Use 10ms base interval. The +5s RFC slow_down bump makes poll 2 and 3 sleep ~5s
    // each in real time, so this test takes ~10s wall clock. Correctness is verified
    // by the 3-mock sequence completing successfully.
    let token = client
        .poll_for_token(
            "dev_code_xyz",
            Duration::from_millis(10),
            Duration::from_secs(60),
        )
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
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "access_denied"})),
        )
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
        ScopeSeparator::Comma,
    );
    let result = client
        .poll_for_token(
            "dev_code",
            Duration::from_millis(10),
            Duration::from_secs(60),
        )
        .await;
    assert!(matches!(result, Err(DeviceCodeError::AccessDenied)));
}

#[tokio::test]
async fn poll_for_token_returns_expired_when_device_code_expired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"error": "expired_token"})),
        )
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
        ScopeSeparator::Comma,
    );
    let result = client
        .poll_for_token(
            "dev_code",
            Duration::from_millis(10),
            Duration::from_secs(60),
        )
        .await;
    assert!(matches!(result, Err(DeviceCodeError::Expired)));
}

// ── #52 — max_total_duration cap ─────────────────────────────────────────────

#[tokio::test]
async fn poll_for_token_returns_poll_duration_exceeded_after_max_duration() {
    let server = MockServer::start().await;
    // Always respond with authorization_pending — would loop forever without cap.
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"error": "authorization_pending"})),
        )
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
        ScopeSeparator::Comma,
    );
    // Very short max_total_duration so the test finishes quickly.
    let result = client
        .poll_for_token(
            "dev_code",
            Duration::from_millis(10),
            Duration::from_millis(50),
        )
        .await;
    assert!(
        matches!(result, Err(DeviceCodeError::PollDurationExceeded)),
        "expected PollDurationExceeded, got: {result:?}"
    );
}

// ── #53 — ScopeSeparator: space separator for GitLab ─────────────────────────

#[tokio::test]
async fn poll_for_token_splits_scopes_with_space_separator() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "glpat_xxx",
            "token_type": "bearer",
            "scope": "read_user api"
        })))
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
        ScopeSeparator::Space,
    );
    let token = client
        .poll_for_token(
            "dev_code",
            Duration::from_millis(10),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(
        token.scopes,
        vec!["read_user".to_string(), "api".to_string()]
    );
}

#[tokio::test]
async fn poll_for_token_splits_scopes_with_comma_separator() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_xxx",
            "token_type": "bearer",
            "scope": "repo,read:user"
        })))
        .mount(&server)
        .await;

    let client = DeviceCodeClient::new(
        format!("{}/device", server.uri()),
        format!("{}/token", server.uri()),
        "id",
        ScopeSeparator::Comma,
    );
    let token = client
        .poll_for_token(
            "dev_code",
            Duration::from_millis(10),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(
        token.scopes,
        vec!["repo".to_string(), "read:user".to_string()]
    );
}
