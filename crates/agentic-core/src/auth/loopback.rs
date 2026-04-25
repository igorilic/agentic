use std::time::Duration;

use axum::{Router, extract::Query, routing::get};
use serde::Deserialize;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

pub struct LoopbackListener {
    pub port: u16,
    pub callback: tokio::task::JoinHandle<Result<CallbackQuery, LoopbackError>>,
}

#[derive(Debug, thiserror::Error)]
pub enum LoopbackError {
    #[error("loopback timed out after {0:?}")]
    Timeout(Duration),
    #[error("loopback cancelled")]
    Cancelled,
    #[error("axum/hyper error: {0}")]
    Server(String),
}

#[derive(Deserialize)]
struct CallbackParams {
    code: String,
    state: String,
}

/// Start an axum server on 127.0.0.1:0 (OS picks the port). The first
/// `GET /callback?code=...&state=...` resolves the returned future and
/// gracefully shuts the server down. Other paths return 404 and do not
/// resolve. The future returns `Err(Timeout)` after `timeout` elapses.
pub async fn start(timeout: Duration) -> Result<LoopbackListener, LoopbackError> {
    let (cb_tx, cb_rx) = oneshot::channel::<CallbackQuery>();
    let cb_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(cb_tx)));
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app = Router::new().route(
        "/callback",
        get({
            let cb_tx = cb_tx.clone();
            move |Query(params): Query<CallbackParams>| {
                let cb_tx = cb_tx.clone();
                async move {
                    let query = CallbackQuery {
                        code: params.code,
                        state: params.state,
                    };
                    if let Some(tx) = cb_tx.lock().unwrap().take() {
                        let _ = tx.send(query);
                    }
                    "OK — you may close this tab."
                }
            }
        }),
    );

    let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| LoopbackError::Server(e.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|e| LoopbackError::Server(e.to_string()))?
        .port();

    // Spawn the axum server with a graceful shutdown signal.
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    // Spawn the wait task: race callback vs timeout, then trigger shutdown.
    let callback = tokio::spawn(async move {
        let result = tokio::select! {
            res = cb_rx => res.map_err(|_| LoopbackError::Cancelled),
            _ = tokio::time::sleep(timeout) => Err(LoopbackError::Timeout(timeout)),
        };
        let _ = shutdown_tx.send(());
        result
    });

    Ok(LoopbackListener { port, callback })
}
