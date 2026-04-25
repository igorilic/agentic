use std::time::Duration;

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

/// Start an axum server on 127.0.0.1:0 (OS picks the port). The first
/// `GET /callback?code=...&state=...` resolves the returned future and
/// gracefully shuts the server down. Other paths return 404 and do not
/// resolve. The future returns `Err(Timeout)` after `timeout` elapses.
pub async fn start(_timeout: Duration) -> Result<LoopbackListener, LoopbackError> {
    todo!("loopback::start not yet implemented")
}
