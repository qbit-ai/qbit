//! Error types for the Gemini Vertex AI provider.

use thiserror::Error;

/// Errors that can occur when using the Gemini Vertex AI provider.
#[derive(Debug, Error)]
pub enum GeminiVertexError {
    /// Failed to authenticate with Google Cloud
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    /// Failed to load service account credentials
    #[error("Failed to load credentials from {path}: {message}")]
    CredentialsError { path: String, message: String },

    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// API returned an error response
    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    /// Failed to parse response
    #[error("Failed to parse response: {0}")]
    ParseError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    StreamError(String),

    /// Token refresh error
    #[error("Failed to refresh token: {0}")]
    TokenRefreshError(String),
}

impl From<gcp_auth::Error> for GeminiVertexError {
    fn from(err: gcp_auth::Error) -> Self {
        GeminiVertexError::AuthenticationError(err.to_string())
    }
}

impl From<serde_json::Error> for GeminiVertexError {
    fn from(err: serde_json::Error) -> Self {
        GeminiVertexError::ParseError(err.to_string())
    }
}
