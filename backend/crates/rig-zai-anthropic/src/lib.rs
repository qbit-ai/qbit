//! Z.AI GLM models via Anthropic-compatible API provider for rig.
//!
//! This crate provides integration with Z.AI's GLM models using their Anthropic-compatible
//! API endpoint. It wraps rig-core's anthropic provider with Z.AI's base URL.
//!
//! # Example
//!
//! ```rust,no_run
//! use rig::client::CompletionClient;
//! use rig_zai_anthropic::Client;
//!
//! // Create client with your Z.AI API key
//! let client = rig_zai_anthropic::new("your-zai-api-key");
//!
//! // Use the GLM-4.7 model
//! let model = client.completion_model(rig_zai_anthropic::GLM_4_7);
//! ```
//!
//! # Environment Variables
//!
//! The client can be created from environment variables:
//! - `ZAI_API_KEY` - Your Z.AI API key (required)
//!
//! # Model Mappings
//!
//! Z.AI's GLM Coding Plan maps Claude models to GLM models:
//! - Claude Opus → GLM-4.7
//! - Claude Sonnet → GLM-4.7
//! - Claude Haiku → GLM-4.5-Air

use rig::providers::anthropic as rig_anthropic;

// Re-export commonly used types from rig's anthropic provider
pub use rig_anthropic::completion::CompletionModel;
pub use rig_anthropic::completion::ANTHROPIC_VERSION_LATEST;

pub mod json_fixer;
mod logging_client;
mod sse_transformer;

pub use logging_client::LoggingClient;
pub use sse_transformer::SseTransformerStream;

// ================================================================
// Z.AI API Constants
// ================================================================

/// Z.AI Anthropic-compatible API base URL
pub const ZAI_ANTHROPIC_BASE_URL: &str = "https://api.z.ai/api/anthropic";

/// GLM-4.7 - Latest and most capable model (maps to Claude Opus/Sonnet)
pub const GLM_4_7: &str = "GLM-4.7";

/// GLM-4.6 - Previous main model
pub const GLM_4_6: &str = "GLM-4.6";

/// GLM-4.5-Air - Fast and economical model (maps to Claude Haiku)
pub const GLM_4_5_AIR: &str = "GLM-4.5-Air";

// ================================================================
// Client
// ================================================================

/// Z.AI client using Anthropic-compatible API.
///
/// This is a type alias for rig's Anthropic client.
pub type Client<H = reqwest::Client> = rig_anthropic::Client<H>;

/// Z.AI client with debug logging enabled.
///
/// This client logs all raw HTTP responses for debugging purposes.
pub type LoggingZaiClient = rig_anthropic::Client<LoggingClient>;

/// Create a new Z.AI Anthropic-compatible client.
///
/// # Arguments
///
/// * `api_key` - Your Z.AI API key
///
/// # Example
///
/// ```rust,no_run
/// use rig::client::CompletionClient;
///
/// let client = rig_zai_anthropic::new("your-api-key");
/// let model = client.completion_model(rig_zai_anthropic::GLM_4_7);
/// ```
pub fn new(api_key: &str) -> Client {
    Client::builder()
        .api_key(api_key)
        .base_url(ZAI_ANTHROPIC_BASE_URL)
        .build()
        .expect("Failed to build Z.AI Anthropic client")
}

/// Create a new Z.AI Anthropic-compatible client with debug logging.
///
/// This client logs all raw HTTP responses for debugging purposes.
/// Use this when troubleshooting API response format issues.
///
/// # Arguments
///
/// * `api_key` - Your Z.AI API key
///
/// # Example
///
/// ```rust,no_run
/// use rig::client::CompletionClient;
///
/// let client = rig_zai_anthropic::new_with_logging("your-api-key");
/// let model = client.completion_model(rig_zai_anthropic::GLM_4_7);
/// ```
pub fn new_with_logging(api_key: &str) -> LoggingZaiClient {
    let logging_client = LoggingClient::new();
    // Use explicit type annotation to help type inference with the custom HTTP client
    let builder: rig_anthropic::ClientBuilder<LoggingClient> =
        rig_anthropic::Client::<LoggingClient>::builder()
            .http_client(logging_client)
            .api_key(api_key)
            .base_url(ZAI_ANTHROPIC_BASE_URL);
    builder
        .build()
        .expect("Failed to build Z.AI Anthropic client with logging")
}

/// Create a new Z.AI client from the `ZAI_API_KEY` environment variable.
///
/// # Panics
///
/// Panics if the `ZAI_API_KEY` environment variable is not set.
///
/// # Example
///
/// ```rust,no_run
/// use rig::client::CompletionClient;
///
/// // Requires ZAI_API_KEY environment variable
/// let client = rig_zai_anthropic::from_env();
/// let model = client.completion_model(rig_zai_anthropic::GLM_4_7);
/// ```
pub fn from_env() -> Client {
    let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY environment variable not set");
    new(&api_key)
}

// ================================================================
// Tests
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_constants() {
        assert_eq!(GLM_4_7, "GLM-4.7");
        assert_eq!(GLM_4_6, "GLM-4.6");
        assert_eq!(GLM_4_5_AIR, "GLM-4.5-Air");
    }

    #[test]
    fn test_base_url_constant() {
        assert_eq!(ZAI_ANTHROPIC_BASE_URL, "https://api.z.ai/api/anthropic");
    }

    #[test]
    fn test_new_function() {
        // Just verify it compiles and runs without panicking
        let _client = new("test-api-key");
    }

    #[test]
    fn test_new_with_logging_function() {
        // Just verify it compiles and runs without panicking
        let _client = new_with_logging("test-api-key");
    }
}
