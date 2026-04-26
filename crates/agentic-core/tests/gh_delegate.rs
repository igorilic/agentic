#![cfg(unix)]

use agentic_core::auth::{GhDelegate, GhDelegateError, MemSecretStore, SecretStore};
use std::path::PathBuf;

fn fixture_bin(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bin")
        .join(name)
}

#[tokio::test]
async fn import_token_stores_token_when_session_valid() {
    let delegate = GhDelegate::with_binary(fixture_bin("fake-gh-with-session.sh"));
    let secrets = MemSecretStore::new();
    delegate
        .import_token(&secrets, "github.access_token")
        .await
        .unwrap();
    assert_eq!(
        secrets.get("github.access_token").unwrap(),
        "ghp_faketoken_for_test_xyz"
    );
}

#[tokio::test]
async fn import_token_returns_no_existing_session_when_gh_not_logged_in() {
    let delegate = GhDelegate::with_binary(fixture_bin("fake-gh-no-session.sh"));
    let secrets = MemSecretStore::new();
    let result = delegate.import_token(&secrets, "github.access_token").await;
    assert!(matches!(result, Err(GhDelegateError::NoExistingSession)));
    // Secret store was not touched.
    assert!(secrets.get("github.access_token").is_err());
}

#[tokio::test]
async fn import_token_returns_gh_not_available_when_binary_missing() {
    let delegate = GhDelegate::with_binary(PathBuf::from("/nonexistent/path/to/gh"));
    let secrets = MemSecretStore::new();
    let result = delegate.import_token(&secrets, "github.access_token").await;
    assert!(matches!(result, Err(GhDelegateError::GhNotAvailable(_))));
}

#[tokio::test]
async fn import_token_returns_empty_token_error_on_blank_output() {
    let delegate = GhDelegate::with_binary(fixture_bin("fake-gh-empty-token.sh"));
    let secrets = MemSecretStore::new();
    let result = delegate.import_token(&secrets, "github.access_token").await;
    assert!(matches!(result, Err(GhDelegateError::EmptyToken)));
}

#[tokio::test]
async fn check_session_returns_ok_for_valid_session() {
    let delegate = GhDelegate::with_binary(fixture_bin("fake-gh-with-session.sh"));
    delegate.check_session().await.unwrap();
}

#[tokio::test]
async fn check_session_returns_err_for_no_session() {
    let delegate = GhDelegate::with_binary(fixture_bin("fake-gh-no-session.sh"));
    let result = delegate.check_session().await;
    assert!(matches!(result, Err(GhDelegateError::NoExistingSession)));
}
