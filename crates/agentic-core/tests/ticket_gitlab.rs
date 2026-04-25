use agentic_core::events::{TicketKind, TicketRef};
use agentic_core::ticket_sources::{
    GitlabAuth, GitlabTicketSource, TicketSource, TicketSourceError,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn gitlab_ref(reference: &str) -> TicketRef {
    TicketRef {
        kind: TicketKind::GitlabIssue,
        reference: reference.to_string(),
        title: None,
    }
}

#[tokio::test]
async fn fetch_returns_parsed_ticket_on_200_ok() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/group%2Frepo/issues/42"))
        .and(header("PRIVATE-TOKEN", "test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "Add feature",
            "description": "Body.\n\n## Acceptance Criteria\n- [ ] Works\n\n## Notes\nMore.",
            "web_url": "https://gitlab.com/group/repo/-/issues/42"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/group%2Frepo/issues/42/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GitlabTicketSource::new(server.uri(), "test-token", GitlabAuth::PrivateToken);
    let r = gitlab_ref("group/repo#42");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "Add feature");
    assert!(ticket.ac_field.as_deref().unwrap().contains("Works"));
    assert!(!ticket.ac_field.as_deref().unwrap().contains("Notes"));
    assert_eq!(
        ticket.url.as_deref(),
        Some("https://gitlab.com/group/repo/-/issues/42")
    );
}

#[tokio::test]
async fn fetch_uses_bearer_auth_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1"))
        .and(header("Authorization", "Bearer oauth-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "t", "description": "", "web_url": "u"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GitlabTicketSource::new(server.uri(), "oauth-token", GitlabAuth::Bearer);
    let r = gitlab_ref("g/r#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "t");
}

#[tokio::test]
async fn fetch_returns_not_found_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/999"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let src = GitlabTicketSource::new(server.uri(), "t", GitlabAuth::PrivateToken);
    let r = gitlab_ref("g/r#999");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(err, TicketSourceError::NotFound { .. }));
}

#[tokio::test]
async fn fetch_returns_auth_error_on_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let src = GitlabTicketSource::new(server.uri(), "bad", GitlabAuth::PrivateToken);
    let r = gitlab_ref("g/r#1");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(err, TicketSourceError::Auth { .. }));
}

#[tokio::test]
async fn fetch_handles_nested_namespace() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/group%2Fsub%2Fproject/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "nested", "description": "", "web_url": "u"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/group%2Fsub%2Fproject/issues/1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let src = GitlabTicketSource::new(server.uri(), "t", GitlabAuth::PrivateToken);
    let r = gitlab_ref("group/sub/project#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "nested");
}

#[tokio::test]
async fn fetch_skips_system_notes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "t", "description": "", "web_url": "u"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"author": {"username": "alice"}, "body": "real comment", "created_at": "2026-04-24T10:00:00Z", "system": false},
            {"author": {"username": "ghost"}, "body": "added label foo", "created_at": "2026-04-24T11:00:00Z", "system": true}
        ])))
        .mount(&server)
        .await;
    let src = GitlabTicketSource::new(server.uri(), "t", GitlabAuth::PrivateToken);
    let r = gitlab_ref("g/r#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.comments.len(), 1);
    assert_eq!(ticket.comments[0].author, "alice");
}

#[tokio::test]
async fn fetch_rejects_non_gitlab_kind() {
    let src = GitlabTicketSource::new("https://example", "", GitlabAuth::PrivateToken);
    let r = TicketRef {
        kind: TicketKind::GithubIssue,
        reference: "owner/repo#1".into(),
        title: None,
    };
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(
        err,
        TicketSourceError::KindMismatch {
            expected: "GitlabIssue",
            ..
        }
    ));
}

#[tokio::test]
async fn fetch_handles_null_description() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "no description issue",
            "description": null,
            "web_url": "https://gitlab.com/g/r/-/issues/1"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/projects/g%2Fr/issues/1/notes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GitlabTicketSource::new(server.uri(), "", GitlabAuth::PrivateToken);
    let r = gitlab_ref("g/r#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "no description issue");
    assert_eq!(ticket.body, "");
    assert!(ticket.ac_field.is_none());
}
