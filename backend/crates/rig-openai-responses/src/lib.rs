//! OpenAI Responses API adapter for rig-core.
//!
//! This crate provides a thin adapter layer that wraps `async-openai` to implement
//! rig-core's `CompletionModel` trait with explicit streaming event handling for
//! reasoning models (o1, o3, gpt-5.x).
//!
//! # Key Features
//!
//! - **Explicit reasoning event separation**: Reasoning deltas are mapped to
//!   `RawStreamingChoice::ReasoningDelta`, never mixed with text deltas.
//! - **Full Responses API support**: Uses OpenAI's newer Responses API instead
//!   of the Chat Completions API.
//! - **Reasoning effort configuration**: Configure `low`, `medium`, or `high`
//!   reasoning effort for supported models.
//!
//! # Example
//!
//! ```ignore
//! use rig_openai_responses::{Client, ReasoningEffort};
//!
//! let client = Client::new("your-api-key");
//! let model = client
//!     .completion_model("gpt-5")
//!     .with_reasoning_effort(ReasoningEffort::High);
//! ```

mod completion;
mod error;

pub use completion::{Client, CompletionModel, ReasoningEffort, StreamingResponseData};
pub use error::OpenAiResponsesError;

/// Check if a model is an OpenAI reasoning model that benefits from this provider.
///
/// Reasoning models include:
/// - o1, o1-preview, o1-mini
/// - o3, o3-mini
/// - o4-mini (future)
/// - gpt-5, gpt-5.1, gpt-5.2, gpt-5-mini, gpt-5-nano
pub fn is_reasoning_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.starts_with("o4")
        || model_lower.starts_with("gpt-5")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_reasoning_model() {
        // Reasoning models
        assert!(is_reasoning_model("o1"));
        assert!(is_reasoning_model("o1-preview"));
        assert!(is_reasoning_model("o1-mini"));
        assert!(is_reasoning_model("o3"));
        assert!(is_reasoning_model("o3-mini"));
        assert!(is_reasoning_model("o4-mini"));
        assert!(is_reasoning_model("gpt-5"));
        assert!(is_reasoning_model("gpt-5.1"));
        assert!(is_reasoning_model("gpt-5.2"));
        assert!(is_reasoning_model("gpt-5-mini"));
        assert!(is_reasoning_model("gpt-5-nano"));

        // Case insensitive
        assert!(is_reasoning_model("GPT-5"));
        assert!(is_reasoning_model("O3-MINI"));

        // Non-reasoning models
        assert!(!is_reasoning_model("gpt-4o"));
        assert!(!is_reasoning_model("gpt-4.1"));
        assert!(!is_reasoning_model("gpt-4o-mini"));
        assert!(!is_reasoning_model("chatgpt-4o-latest"));
    }
}
