use agentic_core::auth::oauth_github::{
    AccessToken, GithubOauthClient, GithubOauthError, validate_state,
};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn validate_state_accepts_match() {
    assert!(validate_state("xyz", "xyz").is_ok());
}

#[tokio::test]
async fn validate_state_rejects_mismatch() {
    let result = validate_state("expected", "attacker");
    assert!(matches!(result, Err(GithubOauthError::StateMismatch { .. })));
}

#[tokio::test]
async fn validate_state_redacts_values_in_error() {
    let result = validate_state("expected", "attacker");
    let err = result.unwrap_err();
    let msg = format!("{err}");
    assert!(!msg.contains("expected"), "error message should not leak expected: {msg}");
    assert!(!msg.contains("attacker"), "error message should not leak actual: {msg}");
}

#[tokio::test]
async fn exchange_code_returns_access_token_on_valid_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .and(header("Accept", "application/json"))
        .and(body_string_contains("code=auth_code_xyz"))
        .and(body_string_contains("code_verifier=verifier_abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_xxx",
            "token_type": "bearer",
            "scope": "repo,read:user",
            "expires_in": 28800,
            "refresh_token": "ghr_yyy"
        })))
        .mount(&server)
        .await;

    let client = GithubOauthClient::new(server.uri(), "test-client-id", Some("test-secret".into()));
    let token = client.exchange_code("auth_code_xyz", "verifier_abc", "http://127.0.0.1:8080/callback").await.unwrap();

    assert_eq!(token.token, "gho_xxx");
    assert_eq!(token.token_type, "bearer");
    assert_eq!(token.scopes, vec!["repo".to_string(), "read:user".to_string()]);
    assert_eq!(token.refresh_token.as_deref(), Some("ghr_yyy"));
    assert!(token.expires_at.is_some());
}

#[tokio::test]
async fn exchange_code_returns_oauth_error_on_invalid_grant() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "The authorization code is incorrect."
        })))
        .mount(&server)
        .await;

    let client = GithubOauthClient::new(server.uri(), "test-client-id", None);
    let result = client.exchange_code("bad_code", "v", "http://x/cb").await;
    assert!(matches!(
        result,
        Err(GithubOauthError::OauthError { ref error, .. }) if error == "invalid_grant"
    ));
}

#[tokio::test]
async fn exchange_code_handles_missing_refresh_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "gho_xxx",
            "token_type": "bearer",
            "scope": "repo"
        })))
        .mount(&server)
        .await;

    let client = GithubOauthClient::new(server.uri(), "id", None);
    let token = client.exchange_code("c", "v", "http://x/cb").await.unwrap();
    assert!(token.refresh_token.is_none());
    assert!(token.expires_at.is_none());
}

#[tokio::test]
async fn exchange_code_returns_parse_error_on_missing_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let client = GithubOauthClient::new(server.uri(), "id", None);
    let result = client.exchange_code("c", "v", "http://x/cb").await;
    assert!(matches!(result, Err(GithubOauthError::Parse(_))));
}

#[tokio::test]
async fn debug_impl_redacts_token_and_refresh_token() {
    let token = AccessToken {
        token: "gho_supersecret".to_string(),
        refresh_token: Some("ghr_alsosecret".to_string()),
        expires_at: Some(1234567890),
        token_type: "bearer".to_string(),
        scopes: vec!["repo".to_string()],
    };
    let dbg = format!("{token:?}");
    assert!(!dbg.contains("gho_supersecret"));
    assert!(!dbg.contains("ghr_alsosecret"));
    assert!(dbg.contains("[redacted]"));
    assert!(dbg.contains("repo"));
}
