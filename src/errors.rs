use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
