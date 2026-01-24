//! Error types for rig-openai-responses.

use thiserror::Error;

/// Errors that can occur when using the OpenAI Responses API provider.
#[derive(Debug, Error)]
pub enum OpenAiResponsesError {
    /// Error from the async-openai client
    #[error("OpenAI API error: {0}")]
    ApiError(#[from] async_openai::error::OpenAIError),

    /// Error converting between rig and OpenAI types
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl From<OpenAiResponsesError> for rig::completion::CompletionError {
    fn from(err: OpenAiResponsesError) -> Self {
        rig::completion::CompletionError::ProviderError(err.to_string())
    }
}
