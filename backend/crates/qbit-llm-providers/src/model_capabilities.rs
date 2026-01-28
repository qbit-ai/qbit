//! Model capability detection for LLM providers.
//!
//! This module provides functions to determine what features are supported
//! by different models. This is particularly important for OpenAI models
//! where some (reasoning models, codex) don't support the temperature parameter.

use serde::{Deserialize, Serialize};

use crate::reasoning_models::is_reasoning_model;

// ============================================================================
// Vision Capabilities
// ============================================================================

/// Vision/image capabilities for LLM providers.
///
/// Used to determine if a model supports image inputs and what constraints apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VisionCapabilities {
    /// Whether the model supports image inputs.
    pub supports_vision: bool,
    /// Maximum image size in bytes (provider-specific limit).
    pub max_image_size_bytes: usize,
    /// Supported MIME types (e.g., "image/png", "image/jpeg").
    pub supported_formats: Vec<String>,
}

impl VisionCapabilities {
    /// Detect vision capabilities based on provider and model name.
    ///
    /// # Arguments
    /// * `provider_name` - The provider identifier (e.g., "openai", "anthropic", "vertex_ai")
    /// * `model_name` - The model identifier (e.g., "gpt-4o", "claude-sonnet-4-5")
    ///
    /// # Examples
    /// ```
    /// use qbit_llm_providers::VisionCapabilities;
    ///
    /// // Claude models support vision
    /// let caps = VisionCapabilities::detect("vertex_ai", "claude-sonnet-4-5");
    /// assert!(caps.supports_vision);
    ///
    /// // Ollama doesn't support vision in our implementation
    /// let caps = VisionCapabilities::detect("ollama", "llama3.2");
    /// assert!(!caps.supports_vision);
    /// ```
    pub fn detect(provider_name: &str, model_name: &str) -> Self {
        let model_lower = model_name.to_lowercase();
        let standard_formats = vec![
            "image/png".to_string(),
            "image/jpeg".to_string(),
            "image/gif".to_string(),
            "image/webp".to_string(),
        ];

        match provider_name {
            // Anthropic (Vertex AI or direct) - Claude 3+ models support vision
            "vertex_ai" | "vertex_ai_anthropic" | "anthropic_vertex" | "anthropic" => {
                let supports_vision = model_lower.contains("claude-3")
                    || model_lower.contains("claude-4")
                    || model_lower.contains("claude-sonnet")
                    || model_lower.contains("claude-opus")
                    || model_lower.contains("claude-haiku");

                Self {
                    supports_vision,
                    max_image_size_bytes: 5 * 1024 * 1024, // 5MB for Anthropic
                    supported_formats: if supports_vision {
                        standard_formats
                    } else {
                        vec![]
                    },
                }
            }

            // OpenAI - GPT-4+ and o-series support vision
            "openai" | "openai_responses" | "openai_reasoning" => {
                let supports_vision = model_lower.contains("gpt-4")
                    || model_lower.contains("gpt-5")
                    || model_lower.starts_with("o1")
                    || model_lower.starts_with("o3")
                    || model_lower.starts_with("o4");

                Self {
                    supports_vision,
                    max_image_size_bytes: 20 * 1024 * 1024, // 20MB for OpenAI
                    supported_formats: if supports_vision {
                        standard_formats
                    } else {
                        vec![]
                    },
                }
            }

            // Gemini - All models support vision
            "gemini" => Self {
                supports_vision: true,
                max_image_size_bytes: 20 * 1024 * 1024, // 20MB
                supported_formats: standard_formats,
            },

            // Z.AI - Vision models have "v" suffix (e.g., glm-4.6v, glm-4v)
            "zai" | "zai_sdk" => {
                // Vision models: glm-4v, glm-4.6v, etc.
                let supports_vision = model_lower.ends_with("v")
                    || model_lower.contains("-v-")
                    || model_lower.ends_with("-v");

                Self {
                    supports_vision,
                    max_image_size_bytes: 10 * 1024 * 1024, // 10MB for Z.AI
                    supported_formats: if supports_vision {
                        standard_formats
                    } else {
                        vec![]
                    },
                }
            }

            // Providers without vision support in our implementation
            "ollama" | "groq" | "xai" | "openrouter" | "mock" => Self::default(),

            // Unknown providers - no vision support
            _ => Self::default(),
        }
    }
}

// ============================================================================
// Model Capabilities (existing)
// ============================================================================

/// Capabilities that vary across LLM providers/models.
///
/// This struct provides a unified way to query model capabilities
/// that affect how the agent loop behaves.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelCapabilities {
    /// Whether the model supports the temperature parameter.
    ///
    /// Most models support temperature, but OpenAI reasoning models (o1, o3)
    /// and some codex models do not.
    pub supports_temperature: bool,

    /// Whether thinking/reasoning should be tracked in message history.
    ///
    /// Some models produce reasoning traces that should be preserved in
    /// the conversation history:
    /// - Anthropic: All models (extended thinking feature)
    /// - OpenAI: Reasoning models (o1, o3 series)
    /// - Gemini: gemini-2.0-flash-thinking-exp
    pub supports_thinking_history: bool,
}

impl ModelCapabilities {
    /// Detect capabilities based on provider and model name.
    ///
    /// # Arguments
    /// * `provider_name` - The provider identifier (e.g., "openai", "anthropic", "vertex_ai_anthropic")
    /// * `model_name` - The model identifier (e.g., "gpt-4o", "claude-3-opus", "o3-mini")
    ///
    /// # Examples
    /// ```
    /// use qbit_llm_providers::ModelCapabilities;
    ///
    /// // Anthropic models support thinking history
    /// let caps = ModelCapabilities::detect("anthropic", "claude-3-opus");
    /// assert!(caps.supports_temperature);
    /// assert!(caps.supports_thinking_history);
    ///
    /// // OpenAI reasoning models don't support temperature but do support thinking history
    /// let caps = ModelCapabilities::detect("openai", "o3-mini");
    /// assert!(!caps.supports_temperature);
    /// assert!(caps.supports_thinking_history);
    ///
    /// // Regular OpenAI models support temperature but not thinking history
    /// let caps = ModelCapabilities::detect("openai", "gpt-4o");
    /// assert!(caps.supports_temperature);
    /// assert!(!caps.supports_thinking_history);
    /// ```
    pub fn detect(provider_name: &str, model_name: &str) -> Self {
        let supports_temperature = model_supports_temperature(provider_name, model_name);
        let supports_thinking_history = detect_thinking_history_support(provider_name, model_name);

        Self {
            supports_temperature,
            supports_thinking_history,
        }
    }

    /// Create capabilities with conservative defaults.
    ///
    /// This is useful when the model name is not known at client creation time.
    /// Returns capabilities that are safe for most models.
    pub fn conservative_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_thinking_history: false,
        }
    }

    /// Create capabilities for Anthropic models.
    ///
    /// All Anthropic models support temperature and thinking history.
    pub fn anthropic_defaults() -> Self {
        Self {
            supports_temperature: true,
            supports_thinking_history: true,
        }
    }
}

/// Detect if a model supports thinking history based on provider and model name.
fn detect_thinking_history_support(provider_name: &str, model_name: &str) -> bool {
    let model_lower = model_name.to_lowercase();

    match provider_name {
        // All Anthropic models support extended thinking
        "anthropic" | "anthropic_vertex" | "vertex_ai_anthropic" | "vertex_ai" => true,

        // OpenAI Responses API: Enable thinking mode for reasoning capabilities.
        // Note: While the API generates reasoning IDs (rs_...), these should NOT be included
        // in conversation history across turns. Including them causes:
        // "Item 'rs_...' of type 'reasoning' was provided without its required following item."
        // The agentic_loop handles this by checking provider_name and excluding reasoning from
        // history for openai_responses. This flag enables thinking mode for the current turn only.
        "openai_responses" => true,

        // OpenAI Chat Completions API: Reasoning models (o-series and gpt-5 series)
        // produce reasoning items that need to be preserved.
        "openai" => is_reasoning_model(model_name),

        // Gemini: Only the thinking-exp model
        "gemini" => model_lower.contains("thinking"),

        // Z.AI: GLM-4.7 supports preserved thinking mode via reasoning_content field
        // The provider always sends thinking: true
        // GLM-4.5 supports interleaved thinking but not explicit thinking config
        "zai" | "zai_sdk" => model_lower.contains("glm-4.7"),

        // All other providers: no thinking history support
        _ => false,
    }
}

/// Check if a model supports the temperature parameter.
///
/// # Arguments
/// * `provider` - The provider name (e.g., "openai", "anthropic", "vertex_ai")
/// * `model` - The model identifier
///
/// # Returns
/// `true` if the model supports temperature, `false` otherwise.
///
/// # Examples
/// ```
/// use qbit_llm_providers::model_supports_temperature;
///
/// assert!(model_supports_temperature("openai", "gpt-4o"));
/// assert!(model_supports_temperature("openai", "gpt-4.1"));
/// assert!(!model_supports_temperature("openai", "o3"));
/// assert!(!model_supports_temperature("openai", "gpt-5"));
/// assert!(!model_supports_temperature("openai", "gpt-5.2"));
/// assert!(!model_supports_temperature("openai", "codex-mini"));
/// assert!(model_supports_temperature("anthropic", "claude-3-opus"));
/// ```
pub fn model_supports_temperature(provider: &str, model: &str) -> bool {
    match provider {
        "openai" | "openai_responses" => {
            let model_lower = model.to_lowercase();

            // Codex models don't support temperature (any variant)
            if model_lower.contains("codex") {
                return false;
            }

            // Reasoning models (o-series and gpt-5 series) don't support temperature
            if is_reasoning_model(model) {
                return false;
            }

            // All other OpenAI models support temperature
            true
        }
        // All other providers support temperature
        _ => true,
    }
}

/// OpenAI models that support the web_search_preview tool.
///
/// Based on OpenAI's documentation, web search is available for:
/// - GPT-4o series (gpt-4o, gpt-4o-mini, chatgpt-4o-latest)
/// - GPT-4.1 series (gpt-4.1, gpt-4.1-mini, gpt-4.1-nano)
/// - GPT-5 series (gpt-5, gpt-5.1, gpt-5.2, gpt-5-mini, gpt-5-nano)
const OPENAI_WEB_SEARCH_MODELS: &[&str] = &[
    // GPT-4o series
    "gpt-4o",
    "gpt-4o-mini",
    "chatgpt-4o-latest",
    // GPT-4.1 series
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4.1-nano",
    // GPT-5 series
    "gpt-5",
    "gpt-5.1",
    "gpt-5.2",
    "gpt-5-mini",
    "gpt-5-nano",
];

/// Check if an OpenAI model supports the native web search tool.
///
/// # Arguments
/// * `model` - The model identifier
///
/// # Returns
/// `true` if the model supports web search, `false` otherwise.
///
/// # Examples
/// ```
/// use qbit_llm_providers::openai_supports_web_search;
///
/// assert!(openai_supports_web_search("gpt-4o"));
/// assert!(openai_supports_web_search("gpt-5.1"));
/// assert!(!openai_supports_web_search("o3"));
/// ```
pub fn openai_supports_web_search(model: &str) -> bool {
    OPENAI_WEB_SEARCH_MODELS
        .iter()
        .any(|m| model.to_lowercase().contains(&m.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== ModelCapabilities::detect() tests ==========

    #[test]
    fn test_model_capabilities_anthropic() {
        // All Anthropic models support both temperature and thinking history
        let caps = ModelCapabilities::detect("anthropic", "claude-3-opus");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("anthropic", "claude-3-sonnet");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("vertex_ai_anthropic", "claude-3-5-sonnet");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("vertex_ai", "claude-opus-4-5");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_openai_reasoning_models() {
        // OpenAI reasoning models: no temperature, yes thinking history
        let caps = ModelCapabilities::detect("openai", "o1");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "o1-preview");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "o3");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "o3-mini");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "o4-mini");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai_responses", "o3");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_openai_regular_models() {
        // Regular OpenAI Chat Completions models: yes temperature, no thinking history
        let caps = ModelCapabilities::detect("openai", "gpt-4o");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "gpt-4o-mini");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "gpt-4.1");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_openai_gpt5_models() {
        // GPT-5 series are reasoning models: no temperature, yes thinking history
        let caps = ModelCapabilities::detect("openai", "gpt-5");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "gpt-5.2");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai", "gpt-5-mini");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_openai_responses_api() {
        // OpenAI Responses API: ALWAYS supports thinking history regardless of model
        // This is because the Responses API generates internal reasoning IDs that
        // function calls reference, and these must be preserved across turns.
        let caps = ModelCapabilities::detect("openai_responses", "gpt-4.1");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openai_responses", "gpt-4o");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // GPT-5 series are reasoning models - no temperature, yes thinking
        let caps = ModelCapabilities::detect("openai_responses", "gpt-5.2");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // o-series reasoning models don't support temperature
        let caps = ModelCapabilities::detect("openai_responses", "o3-mini");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // Codex models don't support temperature
        let caps = ModelCapabilities::detect("openai_responses", "gpt-5.1-codex-max");
        assert!(!caps.supports_temperature);
        assert!(caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_gemini() {
        // Gemini thinking model: yes temperature, yes thinking history
        let caps = ModelCapabilities::detect("gemini", "gemini-2.0-flash-thinking-exp");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // Regular Gemini: yes temperature, no thinking history
        let caps = ModelCapabilities::detect("gemini", "gemini-2.5-pro");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("gemini", "gemini-1.5-flash");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_zai() {
        // Z.AI GLM-4.7: yes temperature, yes thinking history (preserved thinking)
        let caps = ModelCapabilities::detect("zai", "GLM-4.7");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // Case insensitive
        let caps = ModelCapabilities::detect("zai", "glm-4.7");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // GLM-4.5-air: yes temperature, no explicit thinking config
        let caps = ModelCapabilities::detect("zai", "GLM-4.5-air");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_zai_sdk() {
        // zai_sdk provider (same behavior as zai)
        // GLM-4.7: yes temperature, yes thinking history (preserved thinking)
        let caps = ModelCapabilities::detect("zai_sdk", "GLM-4.7");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("zai_sdk", "glm-4.7");
        assert!(caps.supports_temperature);
        assert!(caps.supports_thinking_history);

        // GLM-4.5-air: yes temperature, no explicit thinking config
        let caps = ModelCapabilities::detect("zai_sdk", "GLM-4.5-air");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        // GLM-4-assistant: yes temperature, no thinking history
        let caps = ModelCapabilities::detect("zai_sdk", "glm-4-assistant");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_other_providers() {
        // Other providers: yes temperature, no thinking history
        let caps = ModelCapabilities::detect("groq", "llama-3.3-70b");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("ollama", "llama3.2");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("xai", "grok-2");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        // Note: Z.AI GLM-4.7 does support thinking history - tested separately below
        let caps = ModelCapabilities::detect("zai", "glm-4.5-air");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);

        let caps = ModelCapabilities::detect("openrouter", "anthropic/claude-3-opus");
        assert!(caps.supports_temperature);
        assert!(!caps.supports_thinking_history);
    }

    #[test]
    fn test_model_capabilities_defaults() {
        let conservative = ModelCapabilities::conservative_defaults();
        assert!(conservative.supports_temperature);
        assert!(!conservative.supports_thinking_history);

        let anthropic = ModelCapabilities::anthropic_defaults();
        assert!(anthropic.supports_temperature);
        assert!(anthropic.supports_thinking_history);

        let default = ModelCapabilities::default();
        assert!(!default.supports_temperature);
        assert!(!default.supports_thinking_history);
    }

    // ========== Legacy function tests ==========

    #[test]
    fn test_openai_temperature_support() {
        // Models that DO support temperature
        assert!(model_supports_temperature("openai", "gpt-4o"));
        assert!(model_supports_temperature("openai", "gpt-4o-mini"));
        assert!(model_supports_temperature("openai", "gpt-4.1"));
        assert!(model_supports_temperature("openai", "gpt-4.1-mini"));
        assert!(model_supports_temperature("openai", "chatgpt-4o-latest"));

        // Models that do NOT support temperature (reasoning models)
        // o-series
        assert!(!model_supports_temperature("openai", "o1"));
        assert!(!model_supports_temperature("openai", "o3"));
        assert!(!model_supports_temperature("openai", "o3-mini"));
        assert!(!model_supports_temperature("openai", "o4-mini"));
        // GPT-5 series (all are reasoning models)
        assert!(!model_supports_temperature("openai", "gpt-5"));
        assert!(!model_supports_temperature("openai", "gpt-5.1"));
        assert!(!model_supports_temperature("openai", "gpt-5.2"));
        assert!(!model_supports_temperature("openai", "gpt-5-mini"));
        assert!(!model_supports_temperature("openai", "gpt-5-nano"));

        // Codex models - any variant should NOT support temperature
        assert!(!model_supports_temperature("openai", "gpt-5.1-codex"));
        assert!(!model_supports_temperature("openai", "gpt-5.1-codex-max"));
        assert!(!model_supports_temperature("openai", "codex-mini-latest"));
        assert!(!model_supports_temperature("openai", "codex-mini"));
        assert!(!model_supports_temperature("openai", "codex"));
        assert!(!model_supports_temperature("openai", "CODEX-MINI")); // case insensitive
        assert!(!model_supports_temperature(
            "openai_responses",
            "gpt-5.1-codex-max"
        )); // responses API variant
    }

    #[test]
    fn test_other_providers_always_support_temperature() {
        assert!(model_supports_temperature("anthropic", "claude-3-opus"));
        assert!(model_supports_temperature("vertex_ai", "claude-opus-4-5"));
        assert!(model_supports_temperature("gemini", "gemini-2.5-pro"));
        assert!(model_supports_temperature("groq", "llama-3.3-70b"));
        assert!(model_supports_temperature("ollama", "llama3.2"));
        assert!(model_supports_temperature("xai", "grok-2"));
        assert!(model_supports_temperature("zai", "glm-4.7"));
        assert!(model_supports_temperature("zai_sdk", "glm-4.7"));
        assert!(model_supports_temperature("zai_sdk", "glm-4.6v"));
    }

    #[test]
    fn test_openai_web_search_support() {
        // Models that DO support web search
        assert!(openai_supports_web_search("gpt-4o"));
        assert!(openai_supports_web_search("gpt-4o-mini"));
        assert!(openai_supports_web_search("chatgpt-4o-latest"));
        assert!(openai_supports_web_search("gpt-4.1"));
        assert!(openai_supports_web_search("gpt-4.1-mini"));
        assert!(openai_supports_web_search("gpt-5"));
        assert!(openai_supports_web_search("gpt-5.1"));
        assert!(openai_supports_web_search("gpt-5.2"));

        // Models that do NOT support web search (reasoning models, etc.)
        assert!(!openai_supports_web_search("o1"));
        assert!(!openai_supports_web_search("o3"));
        assert!(!openai_supports_web_search("o3-mini"));
        assert!(!openai_supports_web_search("o4-mini"));
        assert!(!openai_supports_web_search("codex-mini"));
        assert!(!openai_supports_web_search("gpt-3.5-turbo"));
    }

    // ========== VisionCapabilities::detect() tests ==========

    #[test]
    fn test_vision_capabilities_zai() {
        // Vision models have "v" suffix
        let caps = VisionCapabilities::detect("zai", "glm-4v");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 10 * 1024 * 1024);
        assert!(!caps.supported_formats.is_empty());

        let caps = VisionCapabilities::detect("zai", "glm-4.6v");
        assert!(caps.supports_vision);

        let caps = VisionCapabilities::detect("zai", "GLM-4V"); // case insensitive
        assert!(caps.supports_vision);

        // Non-vision models
        let caps = VisionCapabilities::detect("zai", "glm-4.7");
        assert!(!caps.supports_vision);

        let caps = VisionCapabilities::detect("zai", "glm-4-assistant");
        assert!(!caps.supports_vision);

        let caps = VisionCapabilities::detect("zai", "GLM-4.5-air");
        assert!(!caps.supports_vision);
    }

    #[test]
    fn test_vision_capabilities_zai_sdk() {
        // zai_sdk provider (same behavior as zai)
        // Vision models have "v" suffix
        let caps = VisionCapabilities::detect("zai_sdk", "glm-4v");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 10 * 1024 * 1024);

        let caps = VisionCapabilities::detect("zai_sdk", "glm-4.6v");
        assert!(caps.supports_vision);

        // Non-vision models
        let caps = VisionCapabilities::detect("zai_sdk", "glm-4.7");
        assert!(!caps.supports_vision);

        let caps = VisionCapabilities::detect("zai_sdk", "glm-4-assistant");
        assert!(!caps.supports_vision);
    }

    #[test]
    fn test_vision_capabilities_anthropic() {
        // Claude 3+ models support vision
        let caps = VisionCapabilities::detect("anthropic", "claude-3-opus");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 5 * 1024 * 1024);

        let caps = VisionCapabilities::detect("anthropic", "claude-sonnet-4-5");
        assert!(caps.supports_vision);

        let caps = VisionCapabilities::detect("vertex_ai", "claude-opus-4-5");
        assert!(caps.supports_vision);
    }

    #[test]
    fn test_vision_capabilities_openai() {
        // GPT-4+ and o-series support vision
        let caps = VisionCapabilities::detect("openai", "gpt-4o");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 20 * 1024 * 1024);

        let caps = VisionCapabilities::detect("openai", "o3-mini");
        assert!(caps.supports_vision);

        // GPT-3.5 doesn't support vision
        let caps = VisionCapabilities::detect("openai", "gpt-3.5-turbo");
        assert!(!caps.supports_vision);

        // openai_reasoning provider (used for o-series models)
        let caps = VisionCapabilities::detect("openai_reasoning", "o3-mini");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 20 * 1024 * 1024);

        let caps = VisionCapabilities::detect("openai_reasoning", "o1");
        assert!(caps.supports_vision);
    }

    #[test]
    fn test_vision_capabilities_gemini() {
        // All Gemini models support vision
        let caps = VisionCapabilities::detect("gemini", "gemini-2.5-pro");
        assert!(caps.supports_vision);
        assert_eq!(caps.max_image_size_bytes, 20 * 1024 * 1024);
    }

    #[test]
    fn test_vision_capabilities_no_support() {
        // Providers without vision support
        let caps = VisionCapabilities::detect("ollama", "llama3.2");
        assert!(!caps.supports_vision);
        assert!(caps.supported_formats.is_empty());

        let caps = VisionCapabilities::detect("groq", "llama-3.3-70b");
        assert!(!caps.supports_vision);

        let caps = VisionCapabilities::detect("xai", "grok-2");
        assert!(!caps.supports_vision);
    }
}
