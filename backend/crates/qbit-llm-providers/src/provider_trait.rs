//! Provider trait abstraction for unified LLM client creation.
//!
//! This module provides a trait-based abstraction over different LLM providers,
//! eliminating code duplication between `create_*_components()` functions and
//! `LlmClientFactory::create_client()`.

use anyhow::Result;
use async_trait::async_trait;
use qbit_models::{get_model_capabilities, AiProvider, ModelCapabilities};
use rig::client::CompletionClient;

use crate::LlmClient;

/// Trait for LLM provider implementations.
///
/// Each provider implements this trait to encapsulate its specific
/// client creation logic. The trait uses the model registry for
/// capability detection instead of string matching.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider type enum value.
    fn provider_type(&self) -> AiProvider;

    /// Get the provider name for logging.
    fn provider_name(&self) -> &'static str;

    /// Create an LLM client for the given model.
    ///
    /// Uses the model registry to look up capabilities and determine
    /// the appropriate client variant (e.g., reasoning vs standard OpenAI).
    async fn create_client(&self, model: &str) -> Result<LlmClient>;

    /// Validate that the provider has valid credentials configured.
    fn validate_credentials(&self) -> Result<()>;

    /// Get model capabilities from the registry.
    ///
    /// Falls back to provider defaults if the model is not in the registry.
    fn get_capabilities(&self, model: &str) -> ModelCapabilities {
        get_model_capabilities(self.provider_type(), model)
    }
}

/// Configuration for creating providers from settings.
#[derive(Clone)]
pub struct ProviderSettings {
    /// API key (for providers that require one).
    pub api_key: Option<String>,
    /// Base URL override (for providers that support it).
    pub base_url: Option<String>,
    /// Additional provider-specific settings.
    pub extra: ProviderExtraSettings,
}

/// Provider-specific extra settings.
#[derive(Clone, Default)]
pub struct ProviderExtraSettings {
    // Vertex AI specific
    pub credentials_path: Option<String>,
    pub project_id: Option<String>,
    pub location: Option<String>,

    // OpenAI specific
    pub reasoning_effort: Option<String>,
    pub enable_web_search: bool,
    pub web_search_context_size: String,

    // Z.AI SDK specific
    pub source_channel: Option<String>,
}

impl Default for ProviderSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            extra: ProviderExtraSettings {
                web_search_context_size: "medium".to_string(),
                ..Default::default()
            },
        }
    }
}

// =============================================================================
// Provider Implementations
// =============================================================================

/// OpenAI provider implementation.
pub struct OpenAiProviderImpl {
    pub api_key: String,
    pub base_url: Option<String>,
    pub reasoning_effort: Option<String>,
    pub enable_web_search: bool,
    pub web_search_context_size: String,
}

#[async_trait]
impl LlmProvider for OpenAiProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Openai
    }

    fn provider_name(&self) -> &'static str {
        "openai"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use crate::rig_openai_responses;
        use rig::providers::openai as rig_openai;

        let capabilities = self.get_capabilities(model);

        tracing::info!(
            target: "qbit::provider",
            "[OpenAiProvider] Creating client for model={} is_reasoning={}",
            model,
            capabilities.is_reasoning_model
        );

        if capabilities.is_reasoning_model {
            let client = rig_openai_responses::Client::new(&self.api_key);
            let mut completion_model = client.completion_model(model);

            // Set reasoning effort if provided
            if let Some(ref effort_str) = self.reasoning_effort {
                let effort = match effort_str.to_lowercase().as_str() {
                    "low" => rig_openai_responses::ReasoningEffort::Low,
                    "high" => rig_openai_responses::ReasoningEffort::High,
                    _ => rig_openai_responses::ReasoningEffort::Medium,
                };
                completion_model = completion_model.with_reasoning_effort(effort);
            }

            Ok(LlmClient::OpenAiReasoning(completion_model))
        } else {
            let client = rig_openai::Client::new(&self.api_key)
                .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
            let completion_model = client.completion_model(model);
            Ok(LlmClient::RigOpenAiResponses(completion_model))
        }
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("OpenAI API key not configured");
        }
        Ok(())
    }
}

/// Anthropic provider implementation (direct API).
pub struct AnthropicProviderImpl {
    pub api_key: String,
}

#[async_trait]
impl LlmProvider for AnthropicProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Anthropic
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::anthropic as rig_anthropic;

        let client = rig_anthropic::Client::new(&self.api_key)
            .map_err(|e| anyhow::anyhow!("Failed to create Anthropic client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigAnthropic(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("Anthropic API key not configured");
        }
        Ok(())
    }
}

/// Vertex AI (Anthropic Claude on Google Cloud) provider implementation.
pub struct VertexAiProviderImpl {
    pub credentials_path: Option<String>,
    pub project_id: String,
    pub location: String,
}

#[async_trait]
impl LlmProvider for VertexAiProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::VertexAi
    }

    fn provider_name(&self) -> &'static str {
        "vertex_ai"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        let vertex_client = match &self.credentials_path {
            Some(path) => rig_anthropic_vertex::Client::from_service_account(
                path,
                &self.project_id,
                &self.location,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Vertex AI client: {}", e))?,
            None => {
                rig_anthropic_vertex::Client::from_env(&self.project_id, &self.location)
                    .await
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to create Vertex AI client from env: {}", e)
                    })?
            }
        };

        // Enable extended thinking and web search for Claude on Vertex
        let completion_model = vertex_client
            .completion_model(model)
            .with_default_thinking()
            .with_web_search();

        Ok(LlmClient::VertexAnthropic(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.project_id.is_empty() {
            anyhow::bail!("Vertex AI project_id not configured");
        }
        if self.location.is_empty() {
            anyhow::bail!("Vertex AI location not configured");
        }
        Ok(())
    }
}

/// OpenRouter provider implementation.
pub struct OpenRouterProviderImpl {
    pub api_key: String,
}

#[async_trait]
impl LlmProvider for OpenRouterProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Openrouter
    }

    fn provider_name(&self) -> &'static str {
        "openrouter"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::openrouter as rig_openrouter;

        let client = rig_openrouter::Client::new(&self.api_key)
            .map_err(|e| anyhow::anyhow!("Failed to create OpenRouter client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigOpenRouter(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("OpenRouter API key not configured");
        }
        Ok(())
    }
}

/// Ollama provider implementation (local inference).
pub struct OllamaProviderImpl {
    pub base_url: Option<String>,
}

#[async_trait]
impl LlmProvider for OllamaProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Ollama
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::ollama as rig_ollama;

        // TODO: Support custom base_url when rig-ollama adds support
        if self.base_url.is_some() {
            tracing::warn!("Custom base_url is not yet supported for Ollama provider, ignoring");
        }

        let client = rig_ollama::Client::builder()
            .api_key(rig::client::Nothing)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create Ollama client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigOllama(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        // Ollama doesn't require credentials
        Ok(())
    }
}

/// Gemini provider implementation.
pub struct GeminiProviderImpl {
    pub api_key: String,
}

#[async_trait]
impl LlmProvider for GeminiProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Gemini
    }

    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::gemini as rig_gemini;

        let client = rig_gemini::Client::new(&self.api_key)
            .map_err(|e| anyhow::anyhow!("Failed to create Gemini client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigGemini(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("Gemini API key not configured");
        }
        Ok(())
    }
}

/// Groq provider implementation.
pub struct GroqProviderImpl {
    pub api_key: String,
}

#[async_trait]
impl LlmProvider for GroqProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Groq
    }

    fn provider_name(&self) -> &'static str {
        "groq"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::groq as rig_groq;

        let client = rig_groq::Client::builder()
            .api_key(&self.api_key)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create Groq client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigGroq(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("Groq API key not configured");
        }
        Ok(())
    }
}

/// xAI (Grok) provider implementation.
pub struct XaiProviderImpl {
    pub api_key: String,
}

#[async_trait]
impl LlmProvider for XaiProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::Xai
    }

    fn provider_name(&self) -> &'static str {
        "xai"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use rig::providers::xai as rig_xai;

        let client = rig_xai::Client::builder()
            .api_key(&self.api_key)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create xAI client: {}", e))?;
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigXai(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("xAI API key not configured");
        }
        Ok(())
    }
}

/// Z.AI SDK provider implementation.
pub struct ZaiSdkProviderImpl {
    pub api_key: String,
    pub base_url: Option<String>,
    pub source_channel: Option<String>,
}

#[async_trait]
impl LlmProvider for ZaiSdkProviderImpl {
    fn provider_type(&self) -> AiProvider {
        AiProvider::ZaiSdk
    }

    fn provider_name(&self) -> &'static str {
        "zai_sdk"
    }

    async fn create_client(&self, model: &str) -> Result<LlmClient> {
        use crate::rig_zai_sdk;

        let client = rig_zai_sdk::Client::with_config(
            &self.api_key,
            self.base_url.clone(),
            self.source_channel.clone(),
        );
        let completion_model = client.completion_model(model);

        Ok(LlmClient::RigZaiSdk(completion_model))
    }

    fn validate_credentials(&self) -> Result<()> {
        if self.api_key.is_empty() {
            anyhow::bail!("Z.AI API key not configured");
        }
        Ok(())
    }
}

// =============================================================================
// Provider Factory
// =============================================================================

/// Create a provider implementation from settings.
pub fn create_provider(
    provider_type: AiProvider,
    settings: &ProviderSettings,
) -> Result<Box<dyn LlmProvider>> {
    match provider_type {
        AiProvider::Openai => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("OpenAI API key required"))?;
            Ok(Box::new(OpenAiProviderImpl {
                api_key,
                base_url: settings.base_url.clone(),
                reasoning_effort: settings.extra.reasoning_effort.clone(),
                enable_web_search: settings.extra.enable_web_search,
                web_search_context_size: settings.extra.web_search_context_size.clone(),
            }))
        }
        AiProvider::Anthropic => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Anthropic API key required"))?;
            Ok(Box::new(AnthropicProviderImpl { api_key }))
        }
        AiProvider::VertexAi => {
            let project_id = settings
                .extra
                .project_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Vertex AI project_id required"))?;
            let location = settings
                .extra
                .location
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Vertex AI location required"))?;
            Ok(Box::new(VertexAiProviderImpl {
                credentials_path: settings.extra.credentials_path.clone(),
                project_id,
                location,
            }))
        }
        AiProvider::Openrouter => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("OpenRouter API key required"))?;
            Ok(Box::new(OpenRouterProviderImpl { api_key }))
        }
        AiProvider::Ollama => Ok(Box::new(OllamaProviderImpl {
            base_url: settings.base_url.clone(),
        })),
        AiProvider::Gemini => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Gemini API key required"))?;
            Ok(Box::new(GeminiProviderImpl { api_key }))
        }
        AiProvider::Groq => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Groq API key required"))?;
            Ok(Box::new(GroqProviderImpl { api_key }))
        }
        AiProvider::Xai => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("xAI API key required"))?;
            Ok(Box::new(XaiProviderImpl { api_key }))
        }
        AiProvider::ZaiSdk => {
            let api_key = settings
                .api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("Z.AI API key required"))?;
            Ok(Box::new(ZaiSdkProviderImpl {
                api_key,
                base_url: settings.base_url.clone(),
                source_channel: settings.extra.source_channel.clone(),
            }))
        }
    }
}

/// Extract ProviderSettings from QbitSettings for a given provider.
///
/// This helper function maps the typed settings from `QbitSettings` to the
/// unified `ProviderSettings` structure used by the provider trait.
pub fn extract_provider_settings(
    provider_type: AiProvider,
    settings: &qbit_settings::QbitSettings,
) -> ProviderSettings {
    match provider_type {
        AiProvider::Openai => ProviderSettings {
            api_key: settings.ai.openai.api_key.clone(),
            base_url: settings.ai.openai.base_url.clone(),
            extra: ProviderExtraSettings {
                enable_web_search: settings.ai.openai.enable_web_search,
                web_search_context_size: settings.ai.openai.web_search_context_size.clone(),
                ..Default::default()
            },
        },
        AiProvider::Anthropic => ProviderSettings {
            api_key: settings.ai.anthropic.api_key.clone(),
            ..Default::default()
        },
        AiProvider::VertexAi => ProviderSettings {
            extra: ProviderExtraSettings {
                credentials_path: settings.ai.vertex_ai.credentials_path.clone(),
                project_id: settings.ai.vertex_ai.project_id.clone(),
                location: settings.ai.vertex_ai.location.clone(),
                ..Default::default()
            },
            ..Default::default()
        },
        AiProvider::Openrouter => ProviderSettings {
            api_key: settings.ai.openrouter.api_key.clone(),
            ..Default::default()
        },
        AiProvider::Ollama => ProviderSettings {
            // Ollama base_url is a String, wrap in Option
            base_url: Some(settings.ai.ollama.base_url.clone()),
            ..Default::default()
        },
        AiProvider::Gemini => ProviderSettings {
            api_key: settings.ai.gemini.api_key.clone(),
            ..Default::default()
        },
        AiProvider::Groq => ProviderSettings {
            api_key: settings.ai.groq.api_key.clone(),
            ..Default::default()
        },
        AiProvider::Xai => ProviderSettings {
            api_key: settings.ai.xai.api_key.clone(),
            ..Default::default()
        },
        AiProvider::ZaiSdk => ProviderSettings {
            api_key: settings.ai.zai_sdk.api_key.clone(),
            base_url: settings.ai.zai_sdk.base_url.clone(),
            ..Default::default()
        },
    }
}

/// Create a provider and immediately create a client for the given model.
///
/// This is a convenience function that combines `create_provider()` and
/// `LlmProvider::create_client()` into a single call.
pub async fn create_client_for_model(
    provider_type: AiProvider,
    model: &str,
    settings: &qbit_settings::QbitSettings,
) -> Result<crate::LlmClient> {
    let provider_settings = extract_provider_settings(provider_type, settings);
    let provider = create_provider(provider_type, &provider_settings)?;
    provider.create_client(model).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_types() {
        let openai = OpenAiProviderImpl {
            api_key: "test".to_string(),
            base_url: None,
            reasoning_effort: None,
            enable_web_search: false,
            web_search_context_size: "medium".to_string(),
        };
        assert_eq!(openai.provider_type(), AiProvider::Openai);
        assert_eq!(openai.provider_name(), "openai");

        let anthropic = AnthropicProviderImpl {
            api_key: "test".to_string(),
        };
        assert_eq!(anthropic.provider_type(), AiProvider::Anthropic);
        assert_eq!(anthropic.provider_name(), "anthropic");
    }

    #[test]
    fn test_validate_credentials() {
        let empty_openai = OpenAiProviderImpl {
            api_key: "".to_string(),
            base_url: None,
            reasoning_effort: None,
            enable_web_search: false,
            web_search_context_size: "medium".to_string(),
        };
        assert!(empty_openai.validate_credentials().is_err());

        let valid_openai = OpenAiProviderImpl {
            api_key: "sk-test".to_string(),
            base_url: None,
            reasoning_effort: None,
            enable_web_search: false,
            web_search_context_size: "medium".to_string(),
        };
        assert!(valid_openai.validate_credentials().is_ok());

        // Ollama doesn't require credentials
        let ollama = OllamaProviderImpl { base_url: None };
        assert!(ollama.validate_credentials().is_ok());
    }

    #[test]
    fn test_create_provider() {
        let settings = ProviderSettings {
            api_key: Some("test-key".to_string()),
            ..Default::default()
        };

        let provider = create_provider(AiProvider::Openai, &settings).unwrap();
        assert_eq!(provider.provider_type(), AiProvider::Openai);

        // Missing API key should fail
        let empty_settings = ProviderSettings::default();
        assert!(create_provider(AiProvider::Openai, &empty_settings).is_err());

        // Ollama doesn't require API key
        assert!(create_provider(AiProvider::Ollama, &empty_settings).is_ok());
    }
}
