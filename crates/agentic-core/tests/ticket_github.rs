use agentic_core::events::{TicketKind, TicketRef};
use agentic_core::ticket_sources::{GithubTicketSource, TicketSource, TicketSourceError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn github_ref(reference: &str) -> TicketRef {
    TicketRef {
        kind: TicketKind::GithubIssue,
        reference: reference.to_string(),
        title: None,
    }
}

#[tokio::test]
async fn fetch_returns_parsed_ticket_on_200_ok() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/42"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "Add a feature",
            "body": "Description here.\n\n## Acceptance Criteria\n- [ ] Feature works\n- [ ] Tests pass\n\n## Notes\n\nFurther context.",
            "html_url": "https://github.com/owner/repo/issues/42"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/42/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "test-token");
    let r = github_ref("owner/repo#42");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "Add a feature");
    assert!(ticket.body.contains("Description here"));
    assert!(
        ticket
            .ac_field
            .as_deref()
            .unwrap()
            .contains("Feature works")
    );
    assert!(ticket.ac_field.as_deref().unwrap().contains("Tests pass"));
    assert!(!ticket.ac_field.as_deref().unwrap().contains("Notes"));
    assert_eq!(
        ticket.url.as_deref(),
        Some("https://github.com/owner/repo/issues/42")
    );
}

#[tokio::test]
async fn fetch_returns_not_found_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/999"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "test-token");
    let r = github_ref("owner/repo#999");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(err, TicketSourceError::NotFound { .. }));
}

#[tokio::test]
async fn fetch_returns_auth_error_on_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/1"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "bad-token");
    let r = github_ref("owner/repo#1");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(err, TicketSourceError::Auth { .. }));
}

#[tokio::test]
async fn fetch_uses_ghes_base_url_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/owner/repo/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "GHES",
            "body": "issue body",
            "html_url": format!("{}/owner/repo/issues/1", server.uri())
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v3/repos/owner/repo/issues/1/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let base_url = format!("{}/api/v3", server.uri());
    let src = GithubTicketSource::new(base_url, "test-token");
    let r = github_ref("owner/repo#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "GHES");
}

#[tokio::test]
async fn fetch_includes_comments_when_present() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "issue",
            "body": "body",
            "html_url": "https://example/5"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/owner/repo/issues/5/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"user": {"login": "alice"}, "body": "first", "created_at": "2026-04-24T10:00:00Z"},
            {"user": {"login": "bob"}, "body": "second", "created_at": "2026-04-24T11:00:00Z"}
        ])))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "t");
    let r = github_ref("owner/repo#5");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.comments.len(), 2);
    assert_eq!(ticket.comments[0].author, "alice");
    assert!(ticket.comments[0].created_at > 0);
}

#[tokio::test]
async fn fetch_returns_no_ac_field_when_section_absent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "t",
            "body": "no ac here",
            "html_url": "x"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/1/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "");
    let r = github_ref("o/r#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert!(ticket.ac_field.is_none());
}

#[tokio::test]
async fn fetch_returns_parse_error_on_malformed_reference() {
    let src = GithubTicketSource::new("https://example", "");
    let r = github_ref("not-a-valid-ref");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(matches!(err, TicketSourceError::Parse { .. }));
}

#[tokio::test]
async fn fetch_handles_null_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "no body issue",
            "body": null,
            "html_url": "https://github.com/o/r/issues/1"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/1/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "");
    let r = github_ref("o/r#1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "no body issue");
    assert_eq!(ticket.body, "");
    assert!(ticket.ac_field.is_none());
}

#[tokio::test]
async fn fetch_parses_h1_acceptance_criteria() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "title": "h1 ac test",
            "body": "Description.\n\n# Acceptance Criteria\n- [ ] Must work\n\n# Notes\nIgnored.",
            "html_url": "https://github.com/o/r/issues/2"
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/o/r/issues/2/comments"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;

    let src = GithubTicketSource::new(server.uri(), "");
    let r = github_ref("o/r#2");
    let ticket = src.fetch(&r).await.unwrap();
    assert!(
        ticket.ac_field.as_deref().unwrap().contains("Must work"),
        "H1 Acceptance Criteria section should be parsed"
    );
    assert!(
        !ticket.ac_field.as_deref().unwrap().contains("Ignored"),
        "content after next H1 should not be included"
    );
}
