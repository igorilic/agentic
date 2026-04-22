#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid state transition from {from:?} to {to:?}")]
    InvalidStateTransition { from: String, to: String },

    #[error("database error: {0}")]
    Db(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("auth error: {0}")]
    Auth(String),

    #[error("backend error: {0}")]
    Backend(String),

    #[error("ticket source error: {0}")]
    TicketSource(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("agent '{name}' not found")]
    AgentNotFound {
        name: String,
        /// Paths that were checked, in search order.
        searched: Vec<std::path::PathBuf>,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        CoreError::Db(e.to_string())
    }
}

impl From<r2d2::Error> for CoreError {
    fn from(e: r2d2::Error) -> Self {
        CoreError::Db(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
