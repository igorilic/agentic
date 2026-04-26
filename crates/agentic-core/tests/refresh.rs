use agentic_core::auth::{
    AccessToken, GithubRefreshStrategy, RefreshError, RefreshOutcome, RefreshScheduler,
    RefreshStrategy,
};
use std::time::Duration;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn token_with_expiry(expires_at_ms: i64) -> AccessToken {
    AccessToken {
        token: "old_token".to_string(),
        refresh_token: Some("old_refresh".to_string()),
        expires_at: Some(expires_at_ms),
        token_type: "bearer".to_string(),
        scopes: vec!["repo".to_string()],
    }
}

#[derive(Default)]
struct MockStrategy {
    response: std::sync::Mutex<Option<Result<AccessToken, RefreshError>>>,
}

impl MockStrategy {
    fn set(&self, response: Result<AccessToken, RefreshError>) {
        *self.response.lock().unwrap() = Some(response);
    }
}

#[async_trait::async_trait]
impl RefreshStrategy for MockStrategy {
    async fn refresh(
        &self,
        _refresh_token: &str,
        _now_ms: i64,
    ) -> Result<AccessToken, RefreshError> {
        self.response
            .lock()
            .unwrap()
            .take()
            .expect("no response set")
    }
}

// ── should_refresh ──────────────────────────────────────────────────────────

#[test]
fn should_refresh_returns_true_within_lead_time() {
    let scheduler = RefreshScheduler::new(MockStrategy::default());
    let now_ms: i64 = 1_000_000_000;
    // expires in 4 minutes — lead time is 5 minutes — needs refresh
    let token = token_with_expiry(now_ms + 4 * 60 * 1000);
    assert!(scheduler.should_refresh(&token, now_ms));
}

#[test]
fn should_refresh_returns_false_far_from_expiry() {
    let scheduler = RefreshScheduler::new(MockStrategy::default());
    let now_ms: i64 = 1_000_000_000;
    let token = token_with_expiry(now_ms + 30 * 60 * 1000); // 30 min away
    assert!(!scheduler.should_refresh(&token, now_ms));
}

#[test]
fn should_refresh_returns_false_when_no_expires_at() {
    let scheduler = RefreshScheduler::new(MockStrategy::default());
    let token = AccessToken {
        token: "t".to_string(),
        refresh_token: Some("r".to_string()),
        expires_at: None,
        token_type: "bearer".to_string(),
        scopes: vec![],
    };
    assert!(!scheduler.should_refresh(&token, 1_000_000_000));
}

// ── refresh_once ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn refresh_once_returns_new_token_on_success() {
    let mock = MockStrategy::default();
    mock.set(Ok(AccessToken {
        token: "new_token".to_string(),
        refresh_token: Some("new_refresh".to_string()),
        expires_at: Some(2_000_000_000),
        token_type: "bearer".to_string(),
        scopes: vec!["repo".to_string()],
    }));
    let scheduler = RefreshScheduler::new(mock);
    let old = token_with_expiry(1_000_000_000);
    let outcome = scheduler.refresh_once(&old, 1_000_000_000).await;
    match outcome {
        RefreshOutcome::Refreshed(new_token) => {
            assert_eq!(new_token.token, "new_token");
            assert_eq!(new_token.refresh_token.as_deref(), Some("new_refresh"));
        }
        other => panic!("expected Refreshed, got: {other:?}"),
    }
}

#[tokio::test]
async fn refresh_once_returns_needs_reauth_when_no_refresh_token() {
    let scheduler = RefreshScheduler::new(MockStrategy::default());
    let token = AccessToken {
        token: "t".to_string(),
        refresh_token: None,
        expires_at: Some(2_000_000_000),
        token_type: "bearer".to_string(),
        scopes: vec![],
    };
    let outcome = scheduler.refresh_once(&token, 1_000_000_000).await;
    assert!(
        matches!(outcome, RefreshOutcome::NeedsReauth { .. }),
        "expected NeedsReauth, got: {outcome:?}"
    );
}

#[tokio::test]
async fn refresh_once_returns_needs_reauth_when_provider_rejects() {
    let mock = MockStrategy::default();
    mock.set(Err(RefreshError::Rejected {
        error: "bad_refresh_token".to_string(),
        description: "Token revoked".to_string(),
    }));
    let scheduler = RefreshScheduler::new(mock);
    let token = token_with_expiry(1_000_000_000);
    let outcome = scheduler.refresh_once(&token, 1_000_000_000).await;
    match outcome {
        RefreshOutcome::NeedsReauth { reason } => {
            assert!(reason.contains("bad_refresh_token"), "reason: {reason}");
        }
        other => panic!("expected NeedsReauth, got: {other:?}"),
    }
}

#[tokio::test]
async fn refresh_once_returns_needs_reauth_on_transport_error() {
    let mock = MockStrategy::default();
    mock.set(Err(RefreshError::Transport(
        "connection refused".to_string(),
    )));
    let scheduler = RefreshScheduler::new(mock);
    let token = token_with_expiry(1_000_000_000);
    let outcome = scheduler.refresh_once(&token, 1_000_000_000).await;
    // Old behavior mapped Transport to NeedsReauth — but now we have Transient.
    // This test is updated to assert Transient (the new explicit variant).
    assert!(
        matches!(outcome, RefreshOutcome::Transient { .. }),
        "expected Transient, got: {outcome:?}"
    );
}

// ── with_lead_time ────────────────────────────────────────────────────────────

#[test]
fn with_lead_time_overrides_default() {
    let scheduler =
        RefreshScheduler::new(MockStrategy::default()).with_lead_time(Duration::from_secs(60)); // 1 minute lead
    let now_ms: i64 = 1_000_000_000;
    // expires in 30 seconds — within 1 min lead time, so should refresh
    let token = token_with_expiry(now_ms + 30 * 1000);
    assert!(scheduler.should_refresh(&token, now_ms));
    // expires in 5 minutes — outside 1 min lead time, so should NOT refresh
    let far_token = token_with_expiry(now_ms + 5 * 60 * 1000);
    assert!(!scheduler.should_refresh(&far_token, now_ms));
}

// ── GithubRefreshStrategy (wiremock) ─────────────────────────────────────────

#[tokio::test]
async fn github_refresh_strategy_returns_new_access_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("refresh_token=ghr_old"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghp_new",
            "token_type": "bearer",
            "scope": "repo,read:user",
            "expires_in": 28800,
            "refresh_token": "ghr_new"
        })))
        .mount(&server)
        .await;

    let now_ms = 1_700_000_000_000_i64;
    let strat = GithubRefreshStrategy::new(server.uri(), "client_id", Some("secret".into()));
    let new_token = strat.refresh("ghr_old", now_ms).await.unwrap();
    assert_eq!(new_token.token, "ghp_new");
    assert_eq!(new_token.refresh_token.as_deref(), Some("ghr_new"));
    assert_eq!(
        new_token.scopes,
        vec!["repo".to_string(), "read:user".to_string()]
    );
    assert_eq!(
        new_token.expires_at,
        Some(now_ms + 28800 * 1000),
        "expires_at should be deterministic with injected clock"
    );
}

#[tokio::test]
async fn github_refresh_strategy_returns_rejected_on_oauth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": "bad_refresh_token",
            "error_description": "Token has been revoked"
        })))
        .mount(&server)
        .await;

    let strat = GithubRefreshStrategy::new(server.uri(), "id", None);
    let result = strat.refresh("ghr_revoked", 1_700_000_000_000).await;
    assert!(matches!(
        result,
        Err(RefreshError::Rejected { ref error, .. }) if error == "bad_refresh_token"
    ));
}

// ── F7: new edge-case tests ───────────────────────────────────────────────────

#[test]
fn should_refresh_returns_true_at_exact_boundary() {
    let scheduler =
        RefreshScheduler::new(MockStrategy::default()).with_lead_time(Duration::from_secs(60));
    let now_ms: i64 = 1_000_000_000;
    let lead_ms: i64 = 60_000;
    let expires_at = now_ms + lead_ms; // exactly at boundary
    let token = token_with_expiry(expires_at);
    assert!(
        scheduler.should_refresh(&token, now_ms),
        "should refresh at exact boundary: now_ms + lead_ms == expires_at"
    );
}

#[tokio::test]
async fn github_refresh_strategy_returns_none_refresh_token_when_response_omits_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghp_new",
            "token_type": "bearer",
            "scope": "repo",
            "expires_in": 28800
            // NOTE: no refresh_token field
        })))
        .mount(&server)
        .await;

    let strat = GithubRefreshStrategy::new(server.uri(), "id", None);
    let new_token = strat.refresh("ghr_old", 1_700_000_000_000).await.unwrap();
    assert_eq!(new_token.token, "ghp_new");
    assert!(
        new_token.refresh_token.is_none(),
        "refresh_token should be None when response omits it"
    );
}

#[tokio::test]
async fn github_refresh_strategy_handles_expires_in_zero() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/login/oauth/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ghp_x",
            "token_type": "bearer",
            "scope": "",
            "expires_in": 0,
            "refresh_token": "ghr_x"
        })))
        .mount(&server)
        .await;

    let now_ms: i64 = 1_700_000_000_000;
    let strat = GithubRefreshStrategy::new(server.uri(), "id", None);
    let new_token = strat.refresh("ghr_old", now_ms).await.unwrap();
    assert_eq!(
        new_token.expires_at,
        Some(now_ms),
        "expires_at should equal now_ms when expires_in is 0"
    );
}

// ── #56 — new Transient variant tests ─────────────────────────────────────────

#[tokio::test]
async fn refresh_once_returns_transient_on_transport_error() {
    let mock = MockStrategy::default();
    mock.set(Err(RefreshError::Transport("network down".to_string())));
    let scheduler = RefreshScheduler::new(mock);
    let token = token_with_expiry(1_000_000_000);
    let outcome = scheduler.refresh_once(&token, 1_000_000_000).await;
    match outcome {
        RefreshOutcome::Transient { reason } => {
            assert!(reason.contains("network down"), "reason: {reason}");
        }
        other => panic!("expected Transient, got: {other:?}"),
    }
}

#[tokio::test]
async fn refresh_once_returns_transient_on_parse_error() {
    let mock = MockStrategy::default();
    mock.set(Err(RefreshError::Parse("unexpected field".to_string())));
    let scheduler = RefreshScheduler::new(mock);
    let token = token_with_expiry(1_000_000_000);
    let outcome = scheduler.refresh_once(&token, 1_000_000_000).await;
    match outcome {
        RefreshOutcome::Transient { reason } => {
            assert!(reason.contains("unexpected field"), "reason: {reason}");
        }
        other => panic!("expected Transient, got: {other:?}"),
    }
}
