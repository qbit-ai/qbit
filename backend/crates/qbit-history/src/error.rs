use thiserror::Error;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("home directory not found")]
    HomeDirNotFound,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid entry type: {0}")]
    InvalidEntryType(String),
}

pub type Result<T> = std::result::Result<T, HistoryError>;
