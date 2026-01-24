//! Error types for the Z.AI SDK provider.

use thiserror::Error;

/// Errors that can occur when using the Z.AI SDK provider.
#[derive(Debug, Error)]
pub enum ZaiError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    Api {
        status: u16,
        message: String,
        code: Option<String>,
    },

    /// Streaming error
    #[error("Stream error: {0}")]
    Stream(String),

    /// Invalid configuration
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<ZaiError> for rig::completion::CompletionError {
    fn from(err: ZaiError) -> Self {
        match err {
            ZaiError::Http(e) => rig::completion::CompletionError::RequestError(Box::new(e)),
            ZaiError::Json(e) => rig::completion::CompletionError::ResponseError(e.to_string()),
            ZaiError::Api {
                status, message, ..
            } => rig::completion::CompletionError::ProviderError(format!(
                "API error ({}): {}",
                status, message
            )),
            ZaiError::Stream(msg) => rig::completion::CompletionError::ProviderError(msg),
            ZaiError::Config(msg) => rig::completion::CompletionError::ProviderError(msg),
        }
    }
}
