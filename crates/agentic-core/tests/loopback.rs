use std::time::Duration;

use agentic_core::auth::loopback::{CallbackQuery, LoopbackError, start};

#[tokio::test]
async fn port_is_in_ephemeral_range() {
    let listener = start(Duration::from_secs(5)).await.unwrap();
    assert!(listener.port > 1024, "port {} is not > 1024", listener.port);
    assert!(
        listener.port < 65535,
        "port {} is not < 65535",
        listener.port
    );
    // Drop triggers graceful shutdown — no explicit abort needed.
    drop(listener);
}

#[tokio::test]
async fn valid_callback_resolves_with_code_and_state() {
    let mut listener = start(Duration::from_secs(5)).await.unwrap();
    let port = listener.port;
    let url = format!("http://127.0.0.1:{port}/callback?code=abc&state=xyz");

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    let result = listener.take_callback().await.unwrap();
    let query = result.expect("expected Ok(CallbackQuery)");
    assert_eq!(
        query,
        CallbackQuery {
            code: "abc".into(),
            state: "xyz".into()
        }
    );
}

#[tokio::test]
async fn other_path_returns_404_and_does_not_resolve() {
    let mut listener = start(Duration::from_millis(500)).await.unwrap();
    let port = listener.port;
    let url = format!("http://127.0.0.1:{port}/other");

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 404);

    // The future should timeout (not resolve from the 404 hit).
    let result = listener.take_callback().await.unwrap();
    assert!(
        matches!(result, Err(LoopbackError::Timeout(_))),
        "expected Timeout, got: {result:?}"
    );
}

#[tokio::test]
async fn timeout_returns_timeout_error() {
    let mut listener = start(Duration::from_millis(200)).await.unwrap();
    let result = listener.take_callback().await.unwrap();
    assert!(
        matches!(result, Err(LoopbackError::Timeout(_))),
        "expected Timeout, got: {result:?}"
    );
}

#[test]
fn debug_impl_redacts_code() {
    let q = CallbackQuery {
        code: "supersecret_authcode".to_string(),
        state: "csrf-token-xyz".to_string(),
    };
    let dbg = format!("{q:?}");
    assert!(dbg.contains("[redacted]"), "code should be redacted: {dbg}");
    assert!(
        !dbg.contains("supersecret_authcode"),
        "raw code leaked: {dbg}"
    );
    assert!(
        dbg.contains("csrf-token-xyz"),
        "state should still appear: {dbg}"
    );
}

#[tokio::test]
async fn dropping_listener_without_awaiting_aborts_server_quickly() {
    let listener = start(Duration::from_secs(60)).await.unwrap();
    let port = listener.port;
    drop(listener); // should fire shutdown immediately

    // Allow the spawned tasks a moment to clean up.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // The server should be down — connection should be refused.
    let url = format!("http://127.0.0.1:{port}/callback?code=x&state=y");
    let result = reqwest::Client::new()
        .get(&url)
        .timeout(Duration::from_millis(500))
        .send()
        .await;
    assert!(
        result.is_err(),
        "expected connection refused after listener drop, got: {result:?}"
    );
}
