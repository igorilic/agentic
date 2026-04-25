use std::time::Duration;

use axum::{Router, extract::Query, routing::get};
use serde::Deserialize;
use tokio::sync::oneshot;

/// OAuth authorization code and CSRF state returned by the loopback callback.
#[derive(Clone, PartialEq, Eq)]
pub struct CallbackQuery {
    /// OAuth authorization code. SHORT-LIVED SECRET — never log or expose.
    /// Send only to the OAuth token-exchange endpoint and discard immediately.
    pub code: String,
    /// CSRF state parameter. Caller MUST verify this matches the value
    /// returned by `pkce::generate_state` before proceeding.
    pub state: String,
}

impl std::fmt::Debug for CallbackQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackQuery")
            .field("code", &"[redacted]")
            .field("state", &self.state)
            .finish()
    }
}

/// A running loopback listener waiting for the OAuth callback.
///
/// The listener owns the graceful-shutdown trigger for the axum server.
/// When this value is dropped — whether by awaiting the callback future
/// or by explicit `drop()` — the server shuts down and the wait task is
/// aborted. This prevents zombie tasks when the caller abandons the flow.
///
/// # Usage
///
/// ```ignore
/// let mut listener = start(Duration::from_secs(60)).await?;
/// // ... redirect the user's browser to the OAuth URL ...
/// let query = listener.take_callback().await??;
/// // `listener` drops here, triggering graceful shutdown.
/// ```
pub struct LoopbackListener {
    /// The OS-assigned ephemeral port the server is bound to.
    pub port: u16,
    callback: Option<tokio::task::JoinHandle<Result<CallbackQuery, LoopbackError>>>,
    /// Sender for graceful shutdown. `None` after the signal has been sent.
    /// The `Drop` impl fires it if still present.
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl LoopbackListener {
    /// Take ownership of the callback join handle.
    ///
    /// Await the returned handle to obtain the `CallbackQuery` once the
    /// browser hits `/callback`. Drops of `self` afterwards will still
    /// trigger graceful shutdown of the underlying axum server.
    ///
    /// # Panics
    ///
    /// Panics if called more than once on the same listener.
    pub fn take_callback(
        &mut self,
    ) -> tokio::task::JoinHandle<Result<CallbackQuery, LoopbackError>> {
        self.callback.take().expect("take_callback called twice")
    }
}

impl Drop for LoopbackListener {
    fn drop(&mut self) {
        // Trigger graceful shutdown if not already triggered.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        // Abort the wait task. If it has already finished, this is a no-op.
        if let Some(handle) = self.callback.take() {
            handle.abort();
        }
    }
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

/// Start an axum server on `127.0.0.1:0` (OS picks the port). The first
/// `GET /callback?code=...&state=...` resolves the returned future and
/// gracefully shuts the server down. Other paths return 404. The future
/// returns `Err(Timeout)` after `timeout` elapses.
///
/// # Security
///
/// **Callers MUST verify** that `CallbackQuery.state` matches the value
/// they generated via `pkce::generate_state` and sent to the OAuth
/// authorization endpoint. The listener does not validate `state`; it
/// only captures the query parameters. State validation prevents CSRF
/// attacks against the OAuth flow.
///
/// `CallbackQuery.code` is a short-lived secret. Send it once to the
/// token-exchange endpoint and discard. The `Debug` impl redacts it.
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

    // Spawn the axum server. Shuts down when shutdown_rx fires.
    // The listener owns shutdown_tx so Drop always triggers this.
    tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    // Wait task: race callback vs timeout. Does NOT hold shutdown_tx —
    // that stays on the LoopbackListener so Drop can trigger it.
    let callback = tokio::spawn(async move {
        tokio::select! {
            res = cb_rx => res.map_err(|_| LoopbackError::Cancelled),
            _ = tokio::time::sleep(timeout) => Err(LoopbackError::Timeout(timeout)),
        }
    });

    Ok(LoopbackListener {
        port,
        callback: Some(callback),
        shutdown_tx: Some(shutdown_tx),
    })
}
