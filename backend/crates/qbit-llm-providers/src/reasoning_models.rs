//! Reasoning model detection and categorization.
//!
//! This module provides centralized logic for detecting reasoning models
//! (o-series, gpt-5 series) that have special capabilities and constraints.
//!
//! # Why Reasoning Models Are Special
//!
//! Reasoning models (o1, o3, o4, gpt-5) have unique characteristics:
//! - They don't support the temperature parameter
//! - They produce explicit reasoning traces that should be preserved
//! - They use the OpenAI Responses API with dedicated reasoning events
//! - They support configurable reasoning effort levels
//!
//! # Usage
//!
//! ```
//! use qbit_llm_providers::{is_reasoning_model, ReasoningModelCategory};
//!
//! // Simple check
//! assert!(is_reasoning_model("o3-mini"));
//! assert!(is_reasoning_model("gpt-5.2"));
//! assert!(!is_reasoning_model("gpt-4o"));
//!
//! // Detailed categorization
//! assert_eq!(
//!     ReasoningModelCategory::detect("o3-mini"),
//!     Some(ReasoningModelCategory::O3Series)
//! );
//! ```

// Re-export the core detection function from rig-openai-responses
pub use rig_openai_responses::is_reasoning_model;

/// Category of reasoning model for detailed handling.
///
/// This enum allows code to handle different reasoning model families
/// with specific logic if needed (e.g., different context limits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReasoningModelCategory {
    /// o1 series (o1, o1-preview, o1-mini)
    O1Series,
    /// o3 series (o3, o3-mini)
    O3Series,
    /// o4 series (o4-mini, future models)
    O4Series,
    /// GPT-5 series (gpt-5, gpt-5.1, gpt-5.2, gpt-5-mini, gpt-5-nano)
    Gpt5Series,
}

impl ReasoningModelCategory {
    /// Detect the reasoning model category from a model name.
    ///
    /// Returns `None` if the model is not a reasoning model.
    ///
    /// # Examples
    ///
    /// ```
    /// use qbit_llm_providers::ReasoningModelCategory;
    ///
    /// assert_eq!(
    ///     ReasoningModelCategory::detect("o1-preview"),
    ///     Some(ReasoningModelCategory::O1Series)
    /// );
    /// assert_eq!(
    ///     ReasoningModelCategory::detect("gpt-5.2"),
    ///     Some(ReasoningModelCategory::Gpt5Series)
    /// );
    /// assert_eq!(
    ///     ReasoningModelCategory::detect("gpt-4o"),
    ///     None
    /// );
    /// ```
    pub fn detect(model: &str) -> Option<Self> {
        let model_lower = model.to_lowercase();

        if model_lower.starts_with("o1") {
            Some(Self::O1Series)
        } else if model_lower.starts_with("o3") {
            Some(Self::O3Series)
        } else if model_lower.starts_with("o4") {
            Some(Self::O4Series)
        } else if model_lower.starts_with("gpt-5") {
            Some(Self::Gpt5Series)
        } else {
            None
        }
    }

    /// Get all model prefixes for this category.
    pub fn prefixes(&self) -> &'static [&'static str] {
        match self {
            Self::O1Series => &["o1"],
            Self::O3Series => &["o3"],
            Self::O4Series => &["o4"],
            Self::Gpt5Series => &["gpt-5"],
        }
    }
}

/// Check if a model is OpenAI reasoning model based on provider context.
///
/// This is a provider-aware version of `is_reasoning_model` that considers
/// both the provider and model name. This is useful when you need to know
/// if reasoning model behavior applies for a specific provider.
///
/// # Arguments
/// * `provider` - The provider identifier (e.g., "openai", "openai_responses")
/// * `model` - The model identifier
///
/// # Returns
/// `true` if the model is a reasoning model on the given provider.
pub fn is_openai_reasoning_model(provider: &str, model: &str) -> bool {
    match provider {
        "openai" | "openai_responses" | "openai_reasoning" => is_reasoning_model(model),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoning_model_category_detect() {
        // O1 series
        assert_eq!(
            ReasoningModelCategory::detect("o1"),
            Some(ReasoningModelCategory::O1Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("o1-preview"),
            Some(ReasoningModelCategory::O1Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("o1-mini"),
            Some(ReasoningModelCategory::O1Series)
        );

        // O3 series
        assert_eq!(
            ReasoningModelCategory::detect("o3"),
            Some(ReasoningModelCategory::O3Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("o3-mini"),
            Some(ReasoningModelCategory::O3Series)
        );

        // O4 series
        assert_eq!(
            ReasoningModelCategory::detect("o4-mini"),
            Some(ReasoningModelCategory::O4Series)
        );

        // GPT-5 series
        assert_eq!(
            ReasoningModelCategory::detect("gpt-5"),
            Some(ReasoningModelCategory::Gpt5Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("gpt-5.1"),
            Some(ReasoningModelCategory::Gpt5Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("gpt-5.2"),
            Some(ReasoningModelCategory::Gpt5Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("gpt-5-mini"),
            Some(ReasoningModelCategory::Gpt5Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("gpt-5-nano"),
            Some(ReasoningModelCategory::Gpt5Series)
        );

        // Case insensitive
        assert_eq!(
            ReasoningModelCategory::detect("GPT-5"),
            Some(ReasoningModelCategory::Gpt5Series)
        );
        assert_eq!(
            ReasoningModelCategory::detect("O3-MINI"),
            Some(ReasoningModelCategory::O3Series)
        );

        // Non-reasoning models
        assert_eq!(ReasoningModelCategory::detect("gpt-4o"), None);
        assert_eq!(ReasoningModelCategory::detect("gpt-4.1"), None);
        assert_eq!(ReasoningModelCategory::detect("claude-3-opus"), None);
    }

    #[test]
    fn test_is_openai_reasoning_model() {
        // OpenAI providers
        assert!(is_openai_reasoning_model("openai", "o3-mini"));
        assert!(is_openai_reasoning_model("openai_responses", "gpt-5.2"));
        assert!(is_openai_reasoning_model("openai_reasoning", "o1"));

        // Non-reasoning models on OpenAI
        assert!(!is_openai_reasoning_model("openai", "gpt-4o"));
        assert!(!is_openai_reasoning_model("openai_responses", "gpt-4.1"));

        // Other providers - always false even if model name matches
        assert!(!is_openai_reasoning_model("anthropic", "o3-mini"));
        assert!(!is_openai_reasoning_model("vertex_ai", "gpt-5"));
    }
}
