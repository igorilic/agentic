#[derive(thiserror::Error, Debug)]
pub enum CoreError {
    #[error("stub: not yet implemented")]
    Stub,
}
