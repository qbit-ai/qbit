//! Model capability definitions.
//!
//! Capabilities describe what features a model supports, such as:
//! - Temperature control
//! - Vision/image inputs
//! - Web search integration
//! - Reasoning/thinking traces
//! - Context window size

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Capabilities that vary across LLM models.
///
/// This struct provides explicit metadata about what a model supports,
/// replacing runtime string-matching heuristics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "generated/")]
pub struct ModelCapabilities {
    /// Whether the model supports the temperature parameter.
    ///
    /// Most models support temperature, but OpenAI reasoning models (o1, o3, gpt-5)
    /// and codex models do not.
    #[serde(default = "default_true")]
    pub supports_temperature: bool,

    /// Whether thinking/reasoning should be tracked in message history.
    ///
    /// Models that produce reasoning traces that should be preserved:
    /// - Anthropic: All models (extended thinking feature)
    /// - OpenAI: Reasoning models (o1, o3, gpt-5 series)
    /// - Gemini: gemini-2.0-flash-thinking-exp
    #[serde(default)]
    pub supports_thinking_history: bool,

    /// Whether the model supports image/vision inputs.
    #[serde(default)]
    pub supports_vision: bool,

    /// Whether the model supports native web search tools.
    #[serde(default)]
    pub supports_web_search: bool,

    /// Whether this is a reasoning model (uses OpenAI reasoning client).
    ///
    /// Reasoning models (o1, o3, o4, gpt-5) have explicit reasoning events
    /// that must be handled separately from text deltas.
    #[serde(default)]
    pub is_reasoning_model: bool,

    /// Whether this is a coding-optimized model (codex variants).
    #[serde(default)]
    pub is_codex_model: bool,

    /// Context window size in tokens.
    #[serde(default)]
    pub context_window: u32,

    /// Maximum output tokens.
    #[serde(default)]
    pub max_output_tokens: u32,
}

fn default_true() -> bool {
    true
}

impl ModelCapabilities {
    /// Create capabilities with conservative defaults.
    ///
    /// Returns capabilities that are safe for most models:
    /// - supports_temperature: true
    /// - All other capabilities: false/0
    pub fn conservative_defaults() -> Self {
        Self {
            supports_temperature: true,
            ..Default::default()
        }
    }

    /// Create capabilities for Anthropic Claude models.
    pub fn anthropic_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: true,
            context_window: 200_000,
            max_output_tokens: 8_192,
            ..Default::default()
        }
    }

    /// Create capabilities for OpenAI GPT-4 series models.
    pub fn openai_gpt4_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_vision: true,
            supports_web_search: true,
            context_window: 128_000,
            max_output_tokens: 16_384,
            ..Default::default()
        }
    }

    /// Create capabilities for OpenAI GPT-5 reasoning models.
    pub fn openai_gpt5_defaults() -> Self {
        Self {
            supports_temperature: false,
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: true,
            is_reasoning_model: true,
            context_window: 400_000, // GPT-5 series has 400k context
            max_output_tokens: 128_000,
            ..Default::default()
        }
    }

    /// Create capabilities for OpenAI o-series reasoning models (o1, o3, o4).
    pub fn openai_o_series_defaults() -> Self {
        Self {
            supports_temperature: false,
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: true,
            is_reasoning_model: true,
            context_window: 200_000, // o-series has 200k context
            max_output_tokens: 100_000,
            ..Default::default()
        }
    }

    /// Create capabilities for OpenAI codex models.
    pub fn openai_codex_defaults() -> Self {
        Self {
            supports_temperature: false,
            supports_thinking_history: true,
            supports_vision: true,
            supports_web_search: false,
            is_reasoning_model: true,
            is_codex_model: true,
            context_window: 192_000, // Codex has 192k context
            max_output_tokens: 100_000,
        }
    }

    /// Create capabilities for Gemini models.
    pub fn gemini_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_vision: true,
            context_window: 1_048_576, // 1M context window
            max_output_tokens: 65_536, // 65K max output tokens (Gemini 2.5+ and 3.x)
            ..Default::default()
        }
    }

    /// Create capabilities for Gemini 2.0 Flash-Lite (older model with lower output limit).
    pub fn gemini_2_0_flash_lite_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_vision: true,
            context_window: 1_048_576, // 1M context window
            max_output_tokens: 8_192,  // 8K max output tokens (2.0 Flash-Lite only)
            ..Default::default()
        }
    }

    /// Create capabilities for Groq models.
    pub fn groq_defaults() -> Self {
        Self {
            supports_temperature: true,
            context_window: 131_072,
            max_output_tokens: 8_192,
            ..Default::default()
        }
    }

    /// Create capabilities for xAI Grok models.
    pub fn xai_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_vision: false, // Grok doesn't support vision yet
            context_window: 131_072,
            max_output_tokens: 16_384,
            ..Default::default()
        }
    }

    /// Create capabilities for Z.AI GLM models.
    pub fn zai_defaults() -> Self {
        Self {
            supports_temperature: true,
            context_window: 128_000,
            max_output_tokens: 8_192,
            ..Default::default()
        }
    }

    /// Create capabilities for Z.AI GLM-4.7 (with thinking support).
    pub fn zai_thinking_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_thinking_history: true,
            context_window: 128_000,
            max_output_tokens: 8_192,
            ..Default::default()
        }
    }

    /// Create capabilities for Z.AI GLM vision models.
    pub fn zai_vision_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_vision: true,
            context_window: 128_000,
            max_output_tokens: 8_192,
            ..Default::default()
        }
    }

    /// Create capabilities for Ollama local models.
    ///
    /// Capabilities vary by model, so this returns conservative defaults.
    pub fn ollama_defaults() -> Self {
        Self {
            supports_temperature: true,
            context_window: 8_192, // Varies by model
            max_output_tokens: 4_096,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conservative_defaults() {
        let caps = ModelCapabilities::conservative_defaults();
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
        assert!(!caps.supports_vision);
        assert!(!caps.is_reasoning_model);
    }

    #[test]
    fn test_anthropic_defaults() {
        let caps = ModelCapabilities::anthropic_defaults();
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);
        assert!(caps.supports_vision);
        assert!(caps.supports_web_search);
        assert!(!caps.is_reasoning_model);
    }

    #[test]
    fn test_openai_gpt5_defaults() {
        let caps = ModelCapabilities::openai_gpt5_defaults();
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);
        assert!(caps.is_reasoning_model);
        assert!(!caps.is_codex_model);
        assert_eq!(caps.context_window, 400_000);
        assert_eq!(caps.max_output_tokens, 128_000);
    }

    #[test]
    fn test_openai_o_series_defaults() {
        let caps = ModelCapabilities::openai_o_series_defaults();
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);
        assert!(caps.is_reasoning_model);
        assert!(!caps.is_codex_model);
        assert_eq!(caps.context_window, 200_000);
        assert_eq!(caps.max_output_tokens, 100_000);
    }

    #[test]
    fn test_openai_codex_defaults() {
        let caps = ModelCapabilities::openai_codex_defaults();
        assert!(!caps.supports_temperature);
        assert!(caps.is_reasoning_model);
        assert!(caps.is_codex_model);
    }

    #[test]
    fn test_gemini_defaults() {
        let caps = ModelCapabilities::gemini_defaults();
        assert!(caps.supports_temperature);
        assert!(caps.supports_vision);
        assert!(!caps.is_reasoning_model);
        assert_eq!(caps.context_window, 1_048_576);
        assert_eq!(caps.max_output_tokens, 65_536);
    }

    #[test]
    fn test_gemini_2_0_flash_lite_defaults() {
        let caps = ModelCapabilities::gemini_2_0_flash_lite_defaults();
        assert!(caps.supports_temperature);
        assert!(caps.supports_vision);
        assert!(!caps.is_reasoning_model);
        assert_eq!(caps.context_window, 1_048_576);
        assert_eq!(caps.max_output_tokens, 8_192);
    }
}
