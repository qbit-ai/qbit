//! LLM provider abstraction for Qbit.
//!
//! This crate provides a unified interface for interacting with different LLM providers:
//! - OpenRouter via rig-core (supports tools and system prompts)
//! - Anthropic on Vertex AI via rig-anthropic-vertex
//! - OpenAI via rig-core
//! - Ollama local inference via rig-core
//! - Gemini via rig-core
//! - Groq via rig-core
//! - xAI (Grok) via rig-core
//! - Direct Anthropic API via rig-core
//! - Z.AI (GLM models) via rig-zai-sdk (native SDK implementation)
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: rig-core, rig-anthropic-vertex
//! - Used by: qbit-ai (agent orchestration)

mod model_capabilities;
mod openai_config;
mod provider_trait;
mod reasoning_models;

pub use model_capabilities::*;
pub use openai_config::*;
pub use provider_trait::*;
pub use reasoning_models::*;

use std::path::PathBuf;

use rig::providers::anthropic as rig_anthropic;
use rig::providers::gemini as rig_gemini;
use rig::providers::groq as rig_groq;
use rig::providers::ollama as rig_ollama;
use rig::providers::openai as rig_openai;
use rig::providers::openrouter as rig_openrouter;
use rig::providers::xai as rig_xai;
use serde::Deserialize;

/// Convert settings-level [`OpenRouterProviderPreferences`](qbit_settings::OpenRouterProviderPreferences)
/// into the JSON value expected by the OpenRouter API, using rig-core's native
/// [`ProviderPreferences`](rig_openrouter::ProviderPreferences) types for type-safe serialization.
///
/// The settings struct stores values as flat strings/numbers for TOML ergonomics.
/// This function maps those values into rig's typed enums (`DataCollection`,
/// `ProviderSortStrategy`, `Quantization`, `MaxPrice`, etc.) and delegates
/// JSON serialization to `ProviderPreferences::to_json()`.
pub fn openrouter_preferences_to_json(
    prefs: &qbit_settings::schema::OpenRouterProviderPreferences,
) -> serde_json::Value {
    use rig_openrouter::{
        DataCollection, LatencyThreshold, MaxPrice, ProviderPreferences, ProviderSortStrategy,
        Quantization, ThroughputThreshold,
    };

    let mut rig_prefs = ProviderPreferences::new();

    if let Some(ref order) = prefs.order {
        rig_prefs = rig_prefs.order(order.iter().cloned());
    }
    if let Some(ref only) = prefs.only {
        rig_prefs = rig_prefs.only(only.iter().cloned());
    }
    if let Some(ref ignore) = prefs.ignore {
        rig_prefs = rig_prefs.ignore(ignore.iter().cloned());
    }
    if let Some(allow_fallbacks) = prefs.allow_fallbacks {
        rig_prefs = rig_prefs.allow_fallbacks(allow_fallbacks);
    }
    if let Some(require_parameters) = prefs.require_parameters {
        rig_prefs = rig_prefs.require_parameters(require_parameters);
    }
    if let Some(ref data_collection) = prefs.data_collection {
        let dc = match data_collection.to_lowercase().as_str() {
            "deny" => DataCollection::Deny,
            _ => DataCollection::Allow,
        };
        rig_prefs = rig_prefs.data_collection(dc);
    }
    if let Some(zdr) = prefs.zdr {
        rig_prefs = rig_prefs.zdr(zdr);
    }
    if let Some(ref sort) = prefs.sort {
        if let Some(strategy) = match sort.to_lowercase().as_str() {
            "price" => Some(ProviderSortStrategy::Price),
            "throughput" => Some(ProviderSortStrategy::Throughput),
            "latency" => Some(ProviderSortStrategy::Latency),
            _ => None,
        } {
            rig_prefs = rig_prefs.sort(strategy);
        }
    }
    if let Some(throughput) = prefs.preferred_min_throughput {
        rig_prefs = rig_prefs.preferred_min_throughput(ThroughputThreshold::Simple(throughput));
    }
    if let Some(latency) = prefs.preferred_max_latency {
        rig_prefs = rig_prefs.preferred_max_latency(LatencyThreshold::Simple(latency));
    }

    if prefs.max_price_prompt.is_some() || prefs.max_price_completion.is_some() {
        let mut max_price = MaxPrice::new();
        if let Some(prompt) = prefs.max_price_prompt {
            max_price = max_price.prompt(prompt);
        }
        if let Some(completion) = prefs.max_price_completion {
            max_price = max_price.completion(completion);
        }
        rig_prefs = rig_prefs.max_price(max_price);
    }

    if let Some(ref quantizations) = prefs.quantizations {
        let quants: Vec<Quantization> = quantizations
            .iter()
            .filter_map(|q: &String| match q.to_lowercase().as_str() {
                "int4" => Some(Quantization::Int4),
                "int8" => Some(Quantization::Int8),
                "fp8" => Some(Quantization::Fp8),
                "fp16" => Some(Quantization::Fp16),
                "bf16" => Some(Quantization::Bf16),
                "fp32" => Some(Quantization::Fp32),
                "unknown" => Some(Quantization::Unknown),
                _ => None,
            })
            .collect();
        if !quants.is_empty() {
            rig_prefs = rig_prefs.quantizations(quants);
        }
    }

    rig_prefs.to_json()
}

// Re-export for external use
pub use rig_gemini_vertex;
pub use rig_openai_responses;
pub use rig_zai_sdk;

/// LLM client abstraction for different providers
pub enum LlmClient {
    /// Anthropic on Vertex AI via rig-anthropic-vertex
    VertexAnthropic(rig_anthropic_vertex::CompletionModel),
    /// Gemini on Vertex AI via rig-gemini-vertex
    VertexGemini(rig_gemini_vertex::CompletionModel),
    /// OpenRouter via rig-core (supports tools and system prompts)
    RigOpenRouter(rig_openrouter::CompletionModel),
    /// OpenAI via rig-core (uses Chat Completions API - may have tool issues)
    RigOpenAi(rig_openai::completion::CompletionModel),
    /// OpenAI via rig-core (uses Responses API - better tool support)
    RigOpenAiResponses(rig_openai::responses_api::ResponsesCompletionModel),
    /// OpenAI reasoning models via custom provider with explicit streaming event separation.
    /// Used for o1, o3, o4, gpt-5.x models where reasoning deltas must be kept separate from text.
    OpenAiReasoning(rig_openai_responses::CompletionModel),
    /// Direct Anthropic API via rig-core
    RigAnthropic(rig_anthropic::completion::CompletionModel),
    /// Ollama local inference via rig-core
    RigOllama(rig_ollama::CompletionModel<reqwest::Client>),
    /// Gemini via rig-core
    RigGemini(rig_gemini::completion::CompletionModel),
    /// Groq via rig-core
    RigGroq(rig_groq::CompletionModel<reqwest::Client>),
    /// xAI (Grok) via rig-core
    RigXai(rig_xai::completion::CompletionModel<reqwest::Client>),
    /// Z.AI via native SDK implementation
    RigZaiSdk(rig_zai_sdk::CompletionModel),
    /// Mock client for testing (doesn't require credentials)
    /// This variant is always available for integration testing across crates.
    Mock,
}

// Note: A `complete!` macro was attempted here to unify completion calls across providers,
// but it cannot work because rig_anthropic_vertex returns a different CompletionResponse type
// than the standard rig providers. Each call site must use explicit match statements.

impl LlmClient {
    /// Check if this client uses an Anthropic model (Vertex AI, direct API, or Z.AI Anthropic).
    ///
    /// Returns true for providers that support Anthropic-specific features
    /// like extended thinking and native web tools.
    pub fn is_anthropic(&self) -> bool {
        matches!(
            self,
            LlmClient::VertexAnthropic(_) | LlmClient::RigAnthropic(_)
        )
    }

    /// Check if this client supports Claude's native web tools.
    ///
    /// Native web tools (web_search_20250305, web_fetch_20250910) are server-side
    /// tools that Claude executes automatically. They're only supported on
    /// Vertex AI Anthropic for now (direct Anthropic API support may come later).
    pub fn supports_native_web_tools(&self) -> bool {
        match self {
            LlmClient::VertexAnthropic(_) => true,
            LlmClient::Mock => false,
            _ => false,
        }
    }

    /// Get the provider name for logging and debugging.
    pub fn provider_name(&self) -> &'static str {
        match self {
            LlmClient::VertexAnthropic(_) => "vertex_ai_anthropic",
            LlmClient::VertexGemini(_) => "vertex_ai_gemini",
            LlmClient::RigOpenRouter(_) => "openrouter",
            LlmClient::RigOpenAi(_) => "openai",
            LlmClient::RigOpenAiResponses(_) => "openai_responses",
            LlmClient::OpenAiReasoning(_) => "openai_reasoning",
            LlmClient::RigAnthropic(_) => "anthropic",
            LlmClient::RigOllama(_) => "ollama",
            LlmClient::RigGemini(_) => "gemini",
            LlmClient::RigGroq(_) => "groq",
            LlmClient::RigXai(_) => "xai",
            LlmClient::RigZaiSdk(_) => "zai_sdk",
            LlmClient::Mock => "mock",
        }
    }

    /// Check if this client uses a Gemini model on Vertex AI.
    pub fn is_vertex_gemini(&self) -> bool {
        matches!(self, LlmClient::VertexGemini(_))
    }

    /// Check if this client is an OpenAI provider.
    ///
    /// Returns true for Chat Completions API, Responses API, and reasoning model variants.
    pub fn is_openai(&self) -> bool {
        matches!(
            self,
            LlmClient::RigOpenAi(_)
                | LlmClient::RigOpenAiResponses(_)
                | LlmClient::OpenAiReasoning(_)
        )
    }

    /// Check if this client supports OpenAI's native web search tool.
    ///
    /// The web_search_preview tool is a server-side tool that OpenAI
    /// executes during inference, similar to Claude's native web tools.
    pub fn supports_openai_web_search(&self) -> bool {
        matches!(
            self,
            LlmClient::RigOpenAi(_)
                | LlmClient::RigOpenAiResponses(_)
                | LlmClient::OpenAiReasoning(_)
        )
    }

    /// Check if this client uses an OpenAI reasoning model (o1, o3, gpt-5.x).
    ///
    /// These models have explicit reasoning events that must be handled separately.
    pub fn is_reasoning_model(&self) -> bool {
        matches!(self, LlmClient::OpenAiReasoning(_))
    }
}

/// Configuration for creating an AgentBridge with OpenRouter
pub struct OpenRouterClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    /// Provider preferences for routing and filtering (optional).
    pub provider_preferences: Option<serde_json::Value>,
}

/// Configuration for creating an AgentBridge with Vertex AI Anthropic
pub struct VertexAnthropicClientConfig<'a> {
    pub workspace: PathBuf,
    /// Path to service account JSON file. If None, uses application default credentials.
    pub credentials_path: Option<&'a str>,
    pub project_id: &'a str,
    pub location: &'a str,
    pub model: &'a str,
}

/// Configuration for creating an AgentBridge with Vertex AI Gemini
pub struct VertexGeminiClientConfig<'a> {
    pub workspace: PathBuf,
    /// Path to service account JSON file. If None, uses application default credentials.
    pub credentials_path: Option<&'a str>,
    pub project_id: &'a str,
    pub location: &'a str,
    pub model: &'a str,
    /// Whether to include thoughts in the response (for thinking models)
    pub include_thoughts: bool,
}

/// Configuration for creating an AgentBridge with OpenAI
pub struct OpenAiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    pub base_url: Option<&'a str>,
    /// Reasoning effort level for reasoning models (e.g., "low", "medium", "high").
    /// Reserved for future use with models that support reasoning effort configuration.
    pub reasoning_effort: Option<&'a str>,
    /// Enable OpenAI's native web search tool (web_search_preview).
    pub enable_web_search: bool,
    /// Web search context size: "low", "medium", or "high".
    pub web_search_context_size: &'a str,
}

/// Configuration for creating an AgentBridge with direct Anthropic API
pub struct AnthropicClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
}

/// Configuration for creating an AgentBridge with Ollama
pub struct OllamaClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub base_url: Option<&'a str>,
}

/// Configuration for creating an AgentBridge with Gemini
pub struct GeminiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
}

/// Configuration for creating an AgentBridge with Groq
pub struct GroqClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
}

/// Configuration for creating an AgentBridge with xAI (Grok)
pub struct XaiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
}

/// Configuration for creating an AgentBridge with Z.AI via native SDK
pub struct ZaiSdkClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    /// Custom base URL (if None, uses default Z.AI endpoint)
    pub base_url: Option<&'a str>,
    /// Source channel identifier for request tracking
    pub source_channel: Option<&'a str>,
}

fn default_web_search_context_size() -> String {
    "medium".to_string()
}

fn default_include_thoughts() -> bool {
    true
}

/// Unified configuration for all LLM providers.
///
/// Uses serde tag discrimination for clean JSON/frontend integration.
/// This enables a single Tauri command to handle all provider initialization.
#[allow(dead_code)] // Config enum for future multi-provider support
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderConfig {
    /// Anthropic Claude on Google Cloud Vertex AI
    VertexAi {
        workspace: String,
        model: String,
        #[serde(default)]
        credentials_path: Option<String>,
        project_id: String,
        location: String,
    },
    /// Google Gemini on Vertex AI
    VertexGemini {
        workspace: String,
        model: String,
        #[serde(default)]
        credentials_path: Option<String>,
        project_id: String,
        location: String,
        #[serde(default = "default_include_thoughts")]
        include_thoughts: bool,
    },
    /// OpenRouter API (access to multiple providers)
    Openrouter {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default)]
        provider_preferences: Option<serde_json::Value>,
    },
    /// OpenAI API (GPT models)
    Openai {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default)]
        base_url: Option<String>,
        #[serde(default)]
        reasoning_effort: Option<String>,
        #[serde(default)]
        enable_web_search: bool,
        #[serde(default = "default_web_search_context_size")]
        web_search_context_size: String,
    },
    /// Direct Anthropic API
    Anthropic {
        workspace: String,
        model: String,
        api_key: String,
    },
    /// Ollama local inference
    Ollama {
        workspace: String,
        model: String,
        #[serde(default)]
        base_url: Option<String>,
    },
    /// Google Gemini
    Gemini {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default = "default_include_thoughts")]
        include_thoughts: bool,
    },
    /// Groq (fast inference)
    Groq {
        workspace: String,
        model: String,
        api_key: String,
    },
    /// xAI (Grok models)
    Xai {
        workspace: String,
        model: String,
        api_key: String,
    },
    /// Z.AI via native SDK
    ZaiSdk {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default)]
        base_url: Option<String>,
        #[serde(default)]
        source_channel: Option<String>,
    },
}

#[allow(dead_code)] // Methods for future multi-provider config support
impl ProviderConfig {
    /// Get the workspace path from any variant.
    pub fn workspace(&self) -> &str {
        match self {
            Self::VertexAi { workspace, .. } => workspace,
            Self::VertexGemini { workspace, .. } => workspace,
            Self::Openrouter { workspace, .. } => workspace,
            Self::Openai { workspace, .. } => workspace,
            Self::Anthropic { workspace, .. } => workspace,
            Self::Ollama { workspace, .. } => workspace,
            Self::Gemini { workspace, .. } => workspace,
            Self::Groq { workspace, .. } => workspace,
            Self::Xai { workspace, .. } => workspace,
            Self::ZaiSdk { workspace, .. } => workspace,
        }
    }

    /// Get the model name from any variant.
    pub fn model(&self) -> &str {
        match self {
            Self::VertexAi { model, .. } => model,
            Self::VertexGemini { model, .. } => model,
            Self::Openrouter { model, .. } => model,
            Self::Openai { model, .. } => model,
            Self::Anthropic { model, .. } => model,
            Self::Ollama { model, .. } => model,
            Self::Gemini { model, .. } => model,
            Self::Groq { model, .. } => model,
            Self::Xai { model, .. } => model,
            Self::ZaiSdk { model, .. } => model,
        }
    }

    /// Get the provider name as a string.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::VertexAi { .. } => "vertex_ai",
            Self::VertexGemini { .. } => "vertex_gemini",
            Self::Openrouter { .. } => "openrouter",
            Self::Openai { .. } => "openai",
            Self::Anthropic { .. } => "anthropic",
            Self::Ollama { .. } => "ollama",
            Self::Gemini { .. } => "gemini",
            Self::Groq { .. } => "groq",
            Self::Xai { .. } => "xai",
            Self::ZaiSdk { .. } => "zai_sdk",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbit_settings::schema::OpenRouterProviderPreferences;

    #[test]
    fn test_preferences_to_json_basic() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.order = Some(vec!["deepinfra".into(), "deepseek".into()]);
        prefs.sort = Some("throughput".into());

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        assert_eq!(provider.get("order").unwrap(), &serde_json::json!(["deepinfra", "deepseek"]));
        assert_eq!(provider.get("sort").unwrap(), &serde_json::json!("throughput"));
    }

    #[test]
    fn test_preferences_to_json_filters() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.only = Some(vec!["deepinfra".into()]);
        prefs.ignore = Some(vec!["google vertex".into()]);
        prefs.allow_fallbacks = Some(false);
        prefs.zdr = Some(true);
        prefs.data_collection = Some("deny".into());

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        assert_eq!(provider.get("only").unwrap(), &serde_json::json!(["deepinfra"]));
        assert_eq!(provider.get("ignore").unwrap(), &serde_json::json!(["google vertex"]));
        assert_eq!(provider.get("allow_fallbacks").unwrap(), &serde_json::json!(false));
        assert_eq!(provider.get("zdr").unwrap(), &serde_json::json!(true));
        assert_eq!(provider.get("data_collection").unwrap(), &serde_json::json!("deny"));
    }

    #[test]
    fn test_preferences_to_json_max_price() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.max_price_prompt = Some(0.30);
        prefs.max_price_completion = Some(0.50);

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        let max_price = provider.get("max_price").unwrap().as_object().unwrap();
        assert_eq!(max_price.get("prompt").unwrap(), &serde_json::json!(0.30));
        assert_eq!(max_price.get("completion").unwrap(), &serde_json::json!(0.50));
    }

    #[test]
    fn test_preferences_to_json_quantizations() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.quantizations = Some(vec!["fp8".into(), "fp16".into()]);

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        assert_eq!(provider.get("quantizations").unwrap(), &serde_json::json!(["fp8", "fp16"]));
    }

    #[test]
    fn test_preferences_to_json_empty() {
        let prefs = OpenRouterProviderPreferences::default();
        let json = openrouter_preferences_to_json(&prefs);
        assert!(json.get("provider").is_some());
    }

    #[test]
    fn test_preferences_to_json_invalid_sort_ignored() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.sort = Some("invalid_sort".into());

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        assert!(provider.get("sort").is_none());
    }

    #[test]
    fn test_preferences_to_json_invalid_quantization_ignored() {
        let mut prefs = OpenRouterProviderPreferences::default();
        prefs.quantizations = Some(vec!["invalid_quant".into()]);

        let json = openrouter_preferences_to_json(&prefs);
        let provider = json.get("provider").unwrap().as_object().unwrap();
        assert!(provider.get("quantizations").is_none());
    }
}
