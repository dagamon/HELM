use thiserror::Error;

#[derive(Debug, Error)]
pub enum HelmError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal: {0}")]
    Internal(String),
}
