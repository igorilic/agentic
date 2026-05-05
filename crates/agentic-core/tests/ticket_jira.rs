use agentic_core::events::{TicketKind, TicketRef};
use agentic_core::ticket_sources::{JiraAuth, JiraTicketSource, TicketSource, TicketSourceError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn jira_ref(reference: &str) -> TicketRef {
    TicketRef {
        kind: TicketKind::Jira,
        reference: reference.to_string(),
        title: None,
    }
}

fn basic_auth() -> JiraAuth {
    JiraAuth::Basic {
        email: "user@example.com".into(),
        token: "token".into(),
    }
}

// Test 1: happy path — AC parsed from description (no custom field configured)
#[tokio::test]
async fn fetch_returns_ticket_with_ac_from_description() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-1"))
        // base64("user@example.com:token") = "dXNlckBleGFtcGxlLmNvbTp0b2tlbg=="
        .and(header(
            "Authorization",
            "Basic dXNlckBleGFtcGxlLmNvbTp0b2tlbg==",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": "PROJ-1",
            "self": format!("{}/issue/PROJ-1", "PLACEHOLDER"),
            "fields": {
                "summary": "Add a feature",
                "description": {
                    "type": "doc", "version": 1,
                    "content": [
                        {"type": "paragraph", "content": [{"type": "text", "text": "Some description."}]},
                        {"type": "heading", "attrs": {"level": 2}, "content": [{"type": "text", "text": "Acceptance Criteria"}]},
                        {"type": "paragraph", "content": [{"type": "text", "text": "Feature works."}]}
                    ]
                },
                "comment": {"comments": []}
            }
        })))
        .mount(&server)
        .await;

    let src = JiraTicketSource::new(server.uri(), basic_auth(), None);
    let r = jira_ref("PROJ-1");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "Add a feature");
    assert!(
        ticket.body.contains("Some description"),
        "body should contain description text"
    );
    assert!(
        ticket
            .ac_field
            .as_deref()
            .unwrap_or("")
            .contains("Feature works"),
        "ac_field should contain AC text from description"
    );
}

// Test 2: custom AC field overrides description parsing
#[tokio::test]
async fn fetch_uses_ac_custom_field_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": "PROJ-2",
            "self": "x",
            "fields": {
                "summary": "t",
                "description": {
                    "type": "doc", "version": 1,
                    "content": [
                        {"type": "paragraph", "content": [{"type": "text", "text": "Description body, no AC here."}]}
                    ]
                },
                "customfield_10100": {
                    "type": "doc", "version": 1,
                    "content": [
                        {"type": "paragraph", "content": [{"type": "text", "text": "AC from custom field."}]}
                    ]
                },
                "comment": {"comments": []}
            }
        })))
        .mount(&server)
        .await;

    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        Some("customfield_10100".into()),
    );
    let r = jira_ref("PROJ-2");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.ac_field.as_deref(), Some("AC from custom field."));
    assert!(
        !ticket
            .ac_field
            .as_deref()
            .unwrap()
            .contains("Description body"),
        "ac_field should NOT contain description text when custom field is set"
    );
}

// Test 3: custom field configured but absent from response → None
#[tokio::test]
async fn fetch_returns_no_ac_when_custom_field_absent_from_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-3"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": "PROJ-3",
            "self": "x",
            "fields": {
                "summary": "t",
                "description": {
                    "type": "doc", "version": 1,
                    "content": [{"type": "paragraph", "content": [{"type": "text", "text": "no ac here"}]}]
                },
                "comment": {"comments": []}
            }
        })))
        .mount(&server)
        .await;

    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        Some("customfield_10100".into()),
    );
    let r = jira_ref("PROJ-3");
    let ticket = src.fetch(&r).await.unwrap();
    assert!(
        ticket.ac_field.is_none(),
        "ac_field should be None when custom field not present in response"
    );
}

// Test 4a: 404 → NotFound
#[tokio::test]
async fn fetch_returns_not_found_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-4"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        None,
    );
    let r = jira_ref("PROJ-4");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(err, TicketSourceError::NotFound { .. }),
        "expected NotFound, got: {err:?}"
    );
}

// Test 4b: 401 → Auth
#[tokio::test]
async fn fetch_returns_auth_error_on_401() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-401"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Basic {
            email: "u".into(),
            token: "bad-token".into(),
        },
        None,
    );
    let r = jira_ref("PROJ-401");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(err, TicketSourceError::Auth { .. }),
        "expected Auth error, got: {err:?}"
    );
}

// Test 5: comments parsed with ADF bodies
#[tokio::test]
async fn fetch_parses_comments_with_adf_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": "PROJ-5",
            "self": "x",
            "fields": {
                "summary": "t",
                "description": {"type": "doc", "version": 1, "content": []},
                "comment": {"comments": [
                    {
                        "author": {"displayName": "Alice"},
                        "body": {
                            "type": "doc", "version": 1,
                            "content": [{"type": "paragraph", "content": [{"type": "text", "text": "first comment"}]}]
                        },
                        "created": "2026-04-24T10:00:00.000+0000"
                    }
                ]}
            }
        })))
        .mount(&server)
        .await;

    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        None,
    );
    let r = jira_ref("PROJ-5");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.comments.len(), 1);
    assert_eq!(ticket.comments[0].author, "Alice");
    assert!(
        ticket.comments[0].body.contains("first comment"),
        "comment body should contain ADF text"
    );
    assert!(
        ticket.comments[0].created_at > 0,
        "created_at should be a positive timestamp"
    );
}

// Test 6: invalid reference key
#[tokio::test]
async fn fetch_rejects_invalid_reference() {
    let src = JiraTicketSource::new(
        "https://example",
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        None,
    );
    for bad in ["", "noproject", "lower-123", "PROJ-", "-123", "PROJ-abc"] {
        let r = TicketRef {
            kind: TicketKind::Jira,
            reference: bad.into(),
            title: None,
        };
        assert!(
            matches!(src.fetch(&r).await, Err(TicketSourceError::Parse { .. })),
            "expected Parse error for: {bad}"
        );
    }
}

// Test 7: kind mismatch
#[tokio::test]
async fn fetch_rejects_non_jira_kind() {
    let src = JiraTicketSource::new(
        "https://example",
        JiraAuth::Basic {
            email: "u".into(),
            token: "t".into(),
        },
        None,
    );
    let r = TicketRef {
        kind: TicketKind::GithubIssue,
        reference: "PROJ-1".into(),
        title: None,
    };
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(
            err,
            TicketSourceError::KindMismatch {
                expected: "JiraIssue",
                ..
            }
        ),
        "expected KindMismatch, got: {err:?}"
    );
}

// Test 8 (NEW): Bearer auth sends correct Authorization header
#[tokio::test]
async fn fetch_with_bearer_auth_sends_bearer_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/issue/PROJ-8"))
        .and(header("Authorization", "Bearer pat-xyz"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "key": "PROJ-8",
            "self": "x",
            "fields": {
                "summary": "Bearer test",
                "description": {"type": "doc", "version": 1, "content": []},
                "comment": {"comments": []}
            }
        })))
        .mount(&server)
        .await;

    let src = JiraTicketSource::new(
        server.uri(),
        JiraAuth::Bearer {
            token: "pat-xyz".into(),
        },
        None,
    );
    let r = jira_ref("PROJ-8");
    let ticket = src.fetch(&r).await.unwrap();
    assert_eq!(ticket.title, "Bearer test");
}
