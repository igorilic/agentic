//! Tests for `GitlabTicketSource` using the `glab` CLI shell-out path.
//! These tests use fake-binary path injection via `GitlabTicketSource::with_binary_path`.
//! All tests are unix-only because the fixtures are shell scripts.

#![cfg(unix)]

use agentic_core::events::{TicketKind, TicketRef};
use agentic_core::ticket_sources::{GitlabTicketSource, TicketSource, TicketSourceError};
use std::path::PathBuf;

fn fixture_bin(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bin")
        .join(name)
}

fn gitlab_ref(reference: &str) -> TicketRef {
    TicketRef {
        kind: TicketKind::GitlabIssue,
        reference: reference.to_string(),
        title: None,
    }
}

// ── happy path ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn glab_get_ticket_returns_parsed_ticket_on_success() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-success.sh"));
    let r = gitlab_ref("group/repo#42");
    let ticket = src.fetch(&r).await.unwrap();

    assert_eq!(ticket.title, "Add feature");
    assert!(ticket.body.contains("Body."), "body should contain description");
    assert_eq!(
        ticket.url.as_deref(),
        Some("https://gitlab.com/group/repo/-/issues/42")
    );
}

#[tokio::test]
async fn glab_get_ticket_includes_comments() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-success.sh"));
    let r = gitlab_ref("group/repo#42");
    let ticket = src.fetch(&r).await.unwrap();

    assert_eq!(ticket.comments.len(), 1);
    assert_eq!(ticket.comments[0].author, "bob");
    assert_eq!(ticket.comments[0].body, "first note");
    assert!(ticket.comments[0].created_at > 0, "created_at should be positive");
}

// ── acceptance criteria parsing ───────────────────────────────────────────────

#[tokio::test]
async fn glab_acceptance_criteria_parsed_from_body() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-success.sh"));
    let r = gitlab_ref("group/repo#42");
    let ticket = src.fetch(&r).await.unwrap();

    let ac = ticket.ac_field.as_deref().expect("AC field should be present");
    assert!(ac.contains("Works"), "AC should contain the item");
    assert!(
        !ac.contains("More"),
        "AC should not include content after the next section"
    );
}

// ── error paths ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn glab_get_ticket_returns_not_found_when_cli_exits_nonzero_with_not_found_stderr() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-not-found.sh"));
    let r = gitlab_ref("group/repo#999");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(err, TicketSourceError::NotFound { .. }),
        "expected NotFound, got: {err:?}"
    );
}

#[tokio::test]
async fn glab_get_ticket_parse_error_on_garbage_json() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-broken-json.sh"));
    let r = gitlab_ref("group/repo#42");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(err, TicketSourceError::Parse { .. }),
        "expected Parse error, got: {err:?}"
    );
}

#[tokio::test]
async fn glab_get_ticket_transport_error_when_binary_not_found() {
    let src = GitlabTicketSource::with_binary_path(PathBuf::from("/nonexistent/path/to/glab"));
    let r = gitlab_ref("group/repo#42");
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(err, TicketSourceError::Transport { .. }),
        "expected Transport error for missing binary, got: {err:?}"
    );
}

// ── argv shape ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn glab_get_ticket_passes_correct_argv() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-echo-argv.sh"));
    let r = gitlab_ref("group/repo#42");
    // The echo-argv fixture outputs args as stdout (not valid JSON), exits 0.
    // This will produce a Parse error — confirming the binary was called.
    let result = src.fetch(&r).await;
    assert!(
        matches!(result, Err(TicketSourceError::Parse { .. })),
        "expected Parse error (binary was called but output is not JSON), got: {result:?}"
    );
}

// ── kind mismatch ────────────────────────────────────────────────────────────

#[tokio::test]
async fn glab_rejects_non_gitlab_kind() {
    let src =
        GitlabTicketSource::with_binary_path(fixture_bin("fake-glab-issue-view-success.sh"));
    let r = TicketRef {
        kind: TicketKind::GithubIssue,
        reference: "owner/repo#1".into(),
        title: None,
    };
    let err = src.fetch(&r).await.unwrap_err();
    assert!(
        matches!(
            err,
            TicketSourceError::KindMismatch {
                expected: "GitlabIssue",
                ..
            }
        ),
        "expected KindMismatch for non-GitLab ref, got: {err:?}"
    );
}
