//! Z.AI API provider for rig-core.
//!
//! This crate provides a native Rust SDK for the Z.AI API, implementing rig-core's
//! `CompletionModel` trait for seamless integration with the rig ecosystem.
//!
//! # Features
//!
//! - **Native SDK implementation**: Direct HTTP calls following the Z.AI API specification
//! - **Streaming support**: Full SSE streaming with tool call accumulation
//! - **Thinking/reasoning**: Always enabled for enhanced model capabilities
//! - **Tool calling**: Support for function tools with streaming tool calls
//!
//! # Example
//!
//! ```rust,no_run
//! use rig_zai_sdk::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client with API key
//!     let client = Client::new("your-api-key");
//!
//!     // Get a completion model
//!     let model = client.completion_model("glm-4-flash");
//!
//!     // Use with rig's agent or completion request builders
//!     Ok(())
//! }
//! ```
//!
//! # Available Models
//!
//! Common Z.AI models include:
//! - `glm-4-flash` - Fast inference model
//! - `glm-4` - Standard GLM-4 model
//! - `glm-4-plus` - Enhanced GLM-4 model
//! - `glm-4v` - Vision-capable model
//! - `glm-4-alltools` - Model with all tool capabilities

mod client;
mod completion;
mod error;
mod streaming;
mod types;

pub use client::Client;
pub use completion::{CompletionModel, StreamingResponseData, StreamingUsage};
pub use error::ZaiError;
pub use types::{
    ChatCompletionChunk, ChoiceDelta, ChoiceDeltaFunction, ChoiceDeltaToolCall, Completion,
    CompletionChoice, CompletionMessage, CompletionRequest, ContentPart, FunctionCall,
    FunctionDefinition, ImageUrl, Message, MessageContent, Role, StreamingChoice, ToolCall,
    ToolDefinition, Usage,
};

/// Available Z.AI models
pub mod models {
    /// GLM-4.7 - Latest flagship model for agentic coding (December 2025)
    pub const GLM_4_7: &str = "glm-4.7";
    /// GLM-4.6 - Previous generation model
    pub const GLM_4_6: &str = "glm-4.6";
    /// GLM-4 Flash - Fast inference model
    pub const GLM_4_FLASH: &str = "glm-4-flash";
    /// GLM-4 - Standard model (legacy, use GLM_4_7 instead)
    pub const GLM_4: &str = "glm-4.7";
    /// GLM-4 Plus - Enhanced model
    pub const GLM_4_PLUS: &str = "glm-4-plus";
    /// GLM-4V - Vision-capable model
    pub const GLM_4V: &str = "glm-4v";
    /// GLM-4V Plus - Enhanced vision model
    pub const GLM_4V_PLUS: &str = "glm-4v-plus";
    /// GLM-4 All Tools - Model with all tool capabilities
    pub const GLM_4_ALLTOOLS: &str = "glm-4-alltools";
    /// GLM-4 Long - Extended context model
    pub const GLM_4_LONG: &str = "glm-4-long";
    /// GLM-4 Air - Lightweight model
    pub const GLM_4_AIR: &str = "glm-4-air";
    /// GLM-4 Air X - Enhanced lightweight model
    pub const GLM_4_AIR_X: &str = "glm-4-airx";
    /// GLM-4 Flash X - Enhanced fast model
    pub const GLM_4_FLASH_X: &str = "glm-4-flashx";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new("test-api-key");
        assert_eq!(client.api_key(), "test-api-key");
    }

    #[test]
    fn test_completion_model_creation() {
        let client = Client::new("test-api-key");
        let model = client.completion_model(models::GLM_4_FLASH);
        assert_eq!(model.model(), "glm-4-flash");
    }
}
