//! Model capability detection for LLM providers.
//!
//! This module provides functions to determine what features are supported
//! by different models. This is particularly important for OpenAI models
//! where some (reasoning models, codex) don't support the temperature parameter.

/// OpenAI models that do NOT support the temperature parameter.
///
/// These include:
/// - o-series reasoning models (o1, o3, o4-mini)
/// - GPT-5 base models (gpt-5, gpt-5-mini, gpt-5-nano)
/// - Codex models (gpt-5.1-codex, gpt-5.1-codex-max, codex-mini-latest)
const OPENAI_NO_TEMPERATURE_MODELS: &[&str] = &[
    // o-series reasoning models
    "o1",
    "o1-preview",
    "o3",
    "o3-mini",
    "o4-mini",
    // GPT-5 base models (not the .1 or .2 variants)
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    // Codex models
    "gpt-5.1-codex",
    "gpt-5.1-codex-max",
    "codex-mini-latest",
];

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
/// assert!(model_supports_temperature("openai", "gpt-5.2"));
/// assert!(!model_supports_temperature("openai", "o3"));
/// assert!(!model_supports_temperature("openai", "gpt-5"));
/// assert!(model_supports_temperature("anthropic", "claude-3-opus"));
/// ```
pub fn model_supports_temperature(provider: &str, model: &str) -> bool {
    match provider {
        "openai" | "openai_responses" => {
            // Check if this model is in our no-temperature list
            !OPENAI_NO_TEMPERATURE_MODELS
                .iter()
                .any(|m| model.to_lowercase() == m.to_lowercase())
        }
        // All other providers support temperature
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_temperature_support() {
        // Models that DO support temperature
        assert!(model_supports_temperature("openai", "gpt-4o"));
        assert!(model_supports_temperature("openai", "gpt-4o-mini"));
        assert!(model_supports_temperature("openai", "gpt-5.2"));
        assert!(model_supports_temperature("openai", "gpt-5.1"));
        assert!(model_supports_temperature("openai", "gpt-4.1"));
        assert!(model_supports_temperature("openai", "chatgpt-4o-latest"));

        // Models that do NOT support temperature
        assert!(!model_supports_temperature("openai", "o1"));
        assert!(!model_supports_temperature("openai", "o3"));
        assert!(!model_supports_temperature("openai", "o3-mini"));
        assert!(!model_supports_temperature("openai", "o4-mini"));
        assert!(!model_supports_temperature("openai", "gpt-5"));
        assert!(!model_supports_temperature("openai", "gpt-5-mini"));
        assert!(!model_supports_temperature("openai", "gpt-5-nano"));
        assert!(!model_supports_temperature("openai", "gpt-5.1-codex"));
        assert!(!model_supports_temperature("openai", "gpt-5.1-codex-max"));
        assert!(!model_supports_temperature("openai", "codex-mini-latest"));
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
    }
}
