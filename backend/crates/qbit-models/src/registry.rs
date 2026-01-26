//! Model registry and lookup functions.
//!
//! The central `MODEL_REGISTRY` provides a single source of truth for all
//! known model definitions. Models can be looked up by ID or filtered by provider.

use once_cell::sync::Lazy;
use qbit_settings::schema::AiProvider;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use ts_rs::TS;

use crate::capabilities::ModelCapabilities;
use crate::providers::*;

/// Definition of an LLM model with its capabilities.
///
/// Note: This uses `&'static str` for efficiency in the registry.
/// For serialization to frontend, use `OwnedModelDefinition` instead.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelDefinition {
    /// Unique model identifier (e.g., "gpt-5.2", "claude-opus-4-5@20251101")
    pub id: &'static str,
    /// Human-readable display name (e.g., "GPT 5.2", "Claude Opus 4.5")
    pub display_name: &'static str,
    /// Provider this model belongs to
    pub provider: AiProvider,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Alternative IDs that resolve to this model
    #[serde(skip)]
    pub aliases: &'static [&'static str],
}

/// A dynamically discovered model (e.g., from Ollama).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "generated/")]
pub struct DynamicModelDefinition {
    /// Model identifier
    pub id: String,
    /// Human-readable display name
    pub display_name: String,
    /// Provider this model belongs to
    pub provider: AiProvider,
    /// Model capabilities (uses defaults if unknown)
    pub capabilities: ModelCapabilities,
}

impl From<DynamicModelDefinition> for OwnedModelDefinition {
    fn from(d: DynamicModelDefinition) -> Self {
        OwnedModelDefinition {
            id: d.id,
            display_name: d.display_name,
            provider: d.provider,
            capabilities: d.capabilities,
        }
    }
}

/// Owned version of ModelDefinition for serialization to frontend.
///
/// This is the primary type exposed via Tauri commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "generated/")]
pub struct OwnedModelDefinition {
    pub id: String,
    pub display_name: String,
    pub provider: AiProvider,
    pub capabilities: ModelCapabilities,
}

impl From<&ModelDefinition> for OwnedModelDefinition {
    fn from(m: &ModelDefinition) -> Self {
        OwnedModelDefinition {
            id: m.id.to_string(),
            display_name: m.display_name.to_string(),
            provider: m.provider,
            capabilities: m.capabilities.clone(),
        }
    }
}

/// The global model registry containing all known models.
static MODEL_REGISTRY: Lazy<Vec<ModelDefinition>> = Lazy::new(|| {
    let mut models = Vec::new();
    models.extend(vertex_ai_models());
    models.extend(anthropic_models());
    models.extend(openai_models());
    models.extend(gemini_models());
    models.extend(groq_models());
    models.extend(xai_models());
    models.extend(zai_sdk_models());
    models.extend(ollama_default_models());
    models.extend(openrouter_models());
    models
});

/// Index for fast model lookup by ID.
static MODEL_INDEX: Lazy<HashMap<&'static str, usize>> = Lazy::new(|| {
    let mut index = HashMap::new();
    for (i, model) in MODEL_REGISTRY.iter().enumerate() {
        index.insert(model.id, i);
        for alias in model.aliases {
            index.insert(*alias, i);
        }
    }
    index
});

/// Dynamic models discovered at runtime (e.g., from Ollama).
static DYNAMIC_MODELS: Lazy<RwLock<Vec<DynamicModelDefinition>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

/// Look up a model by ID from the static registry.
///
/// Returns `None` if the model is not found.
///
/// # Examples
///
/// ```
/// use qbit_models::get_model;
///
/// if let Some(model) = get_model("gpt-5.2") {
///     println!("{} supports temperature: {}", model.display_name, model.capabilities.supports_temperature);
/// }
/// ```
pub fn get_model(id: &str) -> Option<&'static ModelDefinition> {
    // First check static registry
    MODEL_INDEX.get(id).map(|&i| &MODEL_REGISTRY[i])
}

/// Look up a model by ID, checking both static and dynamic registries.
///
/// Returns an owned copy suitable for serialization.
pub fn get_model_owned(id: &str) -> Option<OwnedModelDefinition> {
    // Check static registry first
    if let Some(model) = get_model(id) {
        return Some(model.into());
    }

    // Check dynamic models
    let dynamics = DYNAMIC_MODELS.read().ok()?;
    dynamics
        .iter()
        .find(|m| m.id == id)
        .map(|m| m.clone().into())
}

/// Get all models for a specific provider from the static registry.
///
/// # Examples
///
/// ```
/// use qbit_models::get_models_for_provider;
/// use qbit_settings::schema::AiProvider;
///
/// let anthropic_models = get_models_for_provider(AiProvider::Anthropic);
/// for model in anthropic_models {
///     println!("- {}", model.display_name);
/// }
/// ```
pub fn get_models_for_provider(provider: AiProvider) -> Vec<&'static ModelDefinition> {
    MODEL_REGISTRY
        .iter()
        .filter(|m| m.provider == provider)
        .collect()
}

/// Get all models for a specific provider, including dynamic models.
///
/// Returns owned copies suitable for serialization.
pub fn get_models_for_provider_owned(provider: AiProvider) -> Vec<OwnedModelDefinition> {
    let mut models: Vec<OwnedModelDefinition> = MODEL_REGISTRY
        .iter()
        .filter(|m| m.provider == provider)
        .map(|m| m.into())
        .collect();

    // Add dynamic models for this provider
    if let Ok(dynamics) = DYNAMIC_MODELS.read() {
        for model in dynamics.iter() {
            if model.provider == provider {
                models.push(model.clone().into());
            }
        }
    }

    models
}

/// Get all models from all providers.
pub fn get_all_models() -> Vec<&'static ModelDefinition> {
    MODEL_REGISTRY.iter().collect()
}

/// Get all models from all providers as owned copies.
pub fn get_all_models_owned() -> Vec<OwnedModelDefinition> {
    let mut models: Vec<OwnedModelDefinition> = MODEL_REGISTRY.iter().map(|m| m.into()).collect();

    // Add dynamic models
    if let Ok(dynamics) = DYNAMIC_MODELS.read() {
        for model in dynamics.iter() {
            models.push(model.clone().into());
        }
    }

    models
}

/// Register a dynamically discovered model.
///
/// This is used for models discovered at runtime, such as Ollama models
/// from the `/api/tags` endpoint.
pub fn register_dynamic_model(model: DynamicModelDefinition) {
    if let Ok(mut dynamics) = DYNAMIC_MODELS.write() {
        // Don't add duplicates
        if !dynamics.iter().any(|m| m.id == model.id) {
            dynamics.push(model);
        }
    }
}

/// Clear all dynamically registered models for a provider.
///
/// This is useful when refreshing the list of available models.
pub fn clear_dynamic_models(provider: AiProvider) {
    if let Ok(mut dynamics) = DYNAMIC_MODELS.write() {
        dynamics.retain(|m| m.provider != provider);
    }
}

/// Create a model definition for an unknown model.
///
/// This provides conservative defaults for models not in the registry.
/// The model will still work, but capabilities may not be accurate.
pub fn create_unknown_model(id: &str, provider: AiProvider) -> OwnedModelDefinition {
    OwnedModelDefinition {
        id: id.to_string(),
        display_name: id.to_string(),
        provider,
        capabilities: ModelCapabilities::conservative_defaults(),
    }
}

/// Check if a model ID is known (static or dynamic).
pub fn is_known_model(id: &str) -> bool {
    if MODEL_INDEX.contains_key(id) {
        return true;
    }

    DYNAMIC_MODELS
        .read()
        .map(|d| d.iter().any(|m| m.id == id))
        .unwrap_or(false)
}

/// Get model capabilities, falling back to conservative defaults for unknown models.
pub fn get_model_capabilities(provider: AiProvider, model: &str) -> ModelCapabilities {
    // First check static registry
    if let Some(model_def) = get_model(model) {
        return model_def.capabilities.clone();
    }

    // Check dynamic models
    if let Ok(dynamics) = DYNAMIC_MODELS.read() {
        if let Some(model_def) = dynamics.iter().find(|m| m.id == model) {
            return model_def.capabilities.clone();
        }
    }

    // Fall back to provider-specific defaults
    match provider {
        AiProvider::VertexAi | AiProvider::Anthropic => ModelCapabilities::anthropic_defaults(),
        AiProvider::Openai => {
            // Detect reasoning models by prefix
            let model_lower = model.to_lowercase();
            if model_lower.starts_with("o1")
                || model_lower.starts_with("o3")
                || model_lower.starts_with("o4")
                || model_lower.starts_with("gpt-5")
            {
                if model_lower.contains("codex") {
                    ModelCapabilities::openai_codex_defaults()
                } else {
                    ModelCapabilities::openai_reasoning_defaults()
                }
            } else {
                ModelCapabilities::openai_gpt4_defaults()
            }
        }
        AiProvider::Gemini => ModelCapabilities::gemini_defaults(),
        AiProvider::Groq => ModelCapabilities::groq_defaults(),
        AiProvider::Xai => ModelCapabilities::xai_defaults(),
        AiProvider::ZaiSdk => ModelCapabilities::zai_defaults(),
        AiProvider::Ollama => ModelCapabilities::ollama_defaults(),
        AiProvider::Openrouter => ModelCapabilities::conservative_defaults(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_model() {
        let model = get_model("gpt-5.2");
        assert!(model.is_some());
        let model = model.unwrap();
        assert_eq!(model.display_name, "GPT 5.2");
        assert!(!model.capabilities.supports_temperature);
        assert!(model.capabilities.is_reasoning_model);
    }

    #[test]
    fn test_get_model_by_alias() {
        let model = get_model("claude-opus-4-5");
        assert!(model.is_some());
        assert_eq!(model.unwrap().id, "claude-opus-4-5-20251101");
    }

    #[test]
    fn test_get_models_for_provider() {
        let anthropic_models = get_models_for_provider(AiProvider::Anthropic);
        assert_eq!(anthropic_models.len(), 3);

        let openai_models = get_models_for_provider(AiProvider::Openai);
        assert!(openai_models.len() >= 10);
    }

    #[test]
    fn test_unknown_model() {
        let model = get_model("some-unknown-model");
        assert!(model.is_none());

        assert!(!is_known_model("some-unknown-model"));
    }

    #[test]
    fn test_dynamic_models() {
        // Clear any existing Ollama dynamics
        clear_dynamic_models(AiProvider::Ollama);

        let dynamic = DynamicModelDefinition {
            id: "custom-model:latest".to_string(),
            display_name: "Custom Model".to_string(),
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
        };

        register_dynamic_model(dynamic);

        assert!(is_known_model("custom-model:latest"));

        let model = get_model_owned("custom-model:latest");
        assert!(model.is_some());
        assert_eq!(model.unwrap().display_name, "Custom Model");

        clear_dynamic_models(AiProvider::Ollama);
        assert!(!is_known_model("custom-model:latest"));
    }

    #[test]
    fn test_get_model_capabilities_fallback() {
        // Known model
        let caps = get_model_capabilities(AiProvider::Openai, "gpt-5.2");
        assert!(!caps.supports_temperature);
        assert!(caps.is_reasoning_model);

        // Unknown but detectable reasoning model
        let caps = get_model_capabilities(AiProvider::Openai, "gpt-5.9-future");
        assert!(!caps.supports_temperature);
        assert!(caps.is_reasoning_model);

        // Unknown regular model
        let caps = get_model_capabilities(AiProvider::Openai, "gpt-4-turbo-2025");
        assert!(caps.supports_temperature);
        assert!(!caps.is_reasoning_model);
    }

    #[test]
    fn test_model_count() {
        let all_models = get_all_models();
        // We should have at least the models we defined
        assert!(all_models.len() >= 30);
    }
}
