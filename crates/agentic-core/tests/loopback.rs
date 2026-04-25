use std::time::Duration;

use agentic_core::auth::loopback::{CallbackQuery, LoopbackError, start};

#[tokio::test]
async fn port_is_in_ephemeral_range() {
    let listener = start(Duration::from_secs(5)).await.unwrap();
    assert!(listener.port > 1024, "port {} is not > 1024", listener.port);
    assert!(listener.port < 65535, "port {} is not < 65535", listener.port);
    listener.callback.abort();
}

#[tokio::test]
async fn valid_callback_resolves_with_code_and_state() {
    let listener = start(Duration::from_secs(5)).await.unwrap();
    let port = listener.port;
    let url = format!("http://127.0.0.1:{port}/callback?code=abc&state=xyz");

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 200);

    let result = listener.callback.await.unwrap();
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
    let listener = start(Duration::from_millis(500)).await.unwrap();
    let port = listener.port;
    let url = format!("http://127.0.0.1:{port}/other");

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status().as_u16(), 404);

    // The future should timeout (not resolve from the 404 hit).
    let result = listener.callback.await.unwrap();
    assert!(
        matches!(result, Err(LoopbackError::Timeout(_))),
        "expected Timeout, got: {result:?}"
    );
}

#[tokio::test]
async fn timeout_returns_timeout_error() {
    let listener = start(Duration::from_millis(200)).await.unwrap();
    let result = listener.callback.await.unwrap();
    assert!(
        matches!(result, Err(LoopbackError::Timeout(_))),
        "expected Timeout, got: {result:?}"
    );
}
