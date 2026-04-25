use agentic_core::auth::oauth_gitlab::{GitlabOauthClient, GitlabOauthError};
use agentic_core::auth::AccessToken;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn exchange_code_returns_access_token_on_valid_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .and(header("Accept", "application/json"))
        .and(body_string_contains("code=auth_code_xyz"))
        .and(body_string_contains("code_verifier=verifier_abc"))
        .and(body_string_contains("grant_type=authorization_code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "glpat_xxx",
            "token_type": "bearer",
            "scope": "read_api write_repository",
            "expires_in": 7200,
            "refresh_token": "glrt_yyy",
            "created_at": 1234567890
        })))
        .mount(&server)
        .await;

    let client = GitlabOauthClient::new(server.uri(), "test-client-id", Some("secret".into()));
    let token = client
        .exchange_code("auth_code_xyz", "verifier_abc", "http://127.0.0.1:8080/callback")
        .await
        .unwrap();

    assert_eq!(token.token, "glpat_xxx");
    assert_eq!(
        token.scopes,
        vec!["read_api".to_string(), "write_repository".to_string()]
    );
    assert_eq!(token.refresh_token.as_deref(), Some("glrt_yyy"));
    assert!(token.expires_at.is_some());
}

#[tokio::test]
async fn exchange_code_returns_oauth_error_on_400_invalid_grant() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "The provided authorization grant is invalid."
        })))
        .mount(&server)
        .await;

    let client = GitlabOauthClient::new(server.uri(), "id", None);
    let result = client.exchange_code("bad_code", "v", "http://x/cb").await;
    assert!(matches!(
        result,
        Err(GitlabOauthError::OauthError { ref error, .. }) if error == "invalid_grant"
    ));
}

#[tokio::test]
async fn exchange_code_handles_self_hosted_base_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "x",
            "token_type": "bearer",
            "scope": "api"
        })))
        .mount(&server)
        .await;

    // server.uri() acts as our self-hosted GitLab base URL.
    let client = GitlabOauthClient::new(server.uri(), "id", None);
    let token = client.exchange_code("c", "v", "http://x/cb").await.unwrap();
    assert_eq!(token.token, "x");
}

#[tokio::test]
async fn exchange_code_returns_parse_error_on_missing_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "token_type": "bearer"
        })))
        .mount(&server)
        .await;

    let client = GitlabOauthClient::new(server.uri(), "id", None);
    let result = client.exchange_code("c", "v", "http://x/cb").await;
    assert!(matches!(result, Err(GitlabOauthError::Parse(_))));
}

#[tokio::test]
async fn exchange_code_handles_missing_refresh_token_and_expires_in() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "x",
            "token_type": "bearer",
            "scope": "api"
        })))
        .mount(&server)
        .await;

    let client = GitlabOauthClient::new(server.uri(), "id", None);
    let token = client.exchange_code("c", "v", "http://x/cb").await.unwrap();
    assert!(token.refresh_token.is_none());
    assert!(token.expires_at.is_none());
}

#[tokio::test]
async fn scope_field_split_on_space() {
    // Explicit test: GitLab uses space-separated scopes (RFC 6749 standard),
    // unlike GitHub which uses comma.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "x",
            "token_type": "bearer",
            "scope": "api read_user write_repository"
        })))
        .mount(&server)
        .await;

    let client = GitlabOauthClient::new(server.uri(), "id", None);
    let token = client.exchange_code("c", "v", "http://x/cb").await.unwrap();
    assert_eq!(
        token.scopes,
        vec![
            "api".to_string(),
            "read_user".to_string(),
            "write_repository".to_string()
        ]
    );
}

// Compile-time check: AccessToken is re-exported from auth root.
#[allow(dead_code)]
fn _type_check(_: AccessToken) {}
