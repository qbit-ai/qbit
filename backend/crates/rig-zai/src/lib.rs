//! Z.AI provider for rig with reasoning_content support.
//!
//! This crate provides a rig-compatible client for Z.AI's GLM models,
//! specifically supporting the thinking/reasoning mode via `reasoning_content`.
//!
//! # Features
//!
//! - Full OpenAI-compatible API support
//! - Streaming with `reasoning_content` for thinking mode
//! - Tool/function calling support
//! - Automatic thinking mode for GLM-4.7
//!
//! # Example
//!
//! ```ignore
//! use rig_zai::Client;
//! use rig::completion::CompletionModel;
//!
//! let client = Client::new("your-api-key");
//! let model = client.completion_model("GLM-4.7");
//!
//! // Use with rig's completion API
//! let response = model.completion(request).await?;
//! ```
//!
//! # Thinking Mode
//!
//! When using GLM-4.7, thinking mode is automatically enabled. The model will
//! stream `reasoning_content` before the final `content`, which is emitted as
//! `StreamedAssistantContent::Reasoning` in the rig streaming API.

pub mod client;
pub mod completion;
pub mod streaming;
pub mod types;

pub use client::Client;
pub use completion::CompletionModel;

/// Z.AI provider errors
#[derive(Debug, thiserror::Error)]
pub enum ZaiError {
    #[error("Request error: {0}")]
    RequestError(String),

    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Available Z.AI models
pub mod models {
    /// GLM-4.7 - Latest flagship with best coding performance and thinking mode
    pub const GLM_4_7: &str = "GLM-4.7";

    /// GLM-4.5-air - Fast, cost-efficient variant
    pub const GLM_4_5_AIR: &str = "GLM-4.5-air";
}
