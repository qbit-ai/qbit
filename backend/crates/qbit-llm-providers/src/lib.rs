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
//! - Z.AI (GLM models) via rig-zai
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: rig-core, rig-anthropic-vertex
//! - Used by: qbit-ai (agent orchestration)

use std::path::PathBuf;

use rig::providers::anthropic as rig_anthropic;
use rig::providers::gemini as rig_gemini;
use rig::providers::groq as rig_groq;
use rig::providers::ollama as rig_ollama;
use rig::providers::openai as rig_openai;
use rig::providers::openrouter as rig_openrouter;
use rig::providers::xai as rig_xai;
use serde::Deserialize;

/// LLM client abstraction for different providers
pub enum LlmClient {
    /// Anthropic on Vertex AI via rig-anthropic-vertex
    VertexAnthropic(rig_anthropic_vertex::CompletionModel),
    /// OpenRouter via rig-core (supports tools and system prompts)
    RigOpenRouter(rig_openrouter::CompletionModel),
    /// OpenAI via rig-core (uses Chat Completions API - may have tool issues)
    RigOpenAi(rig_openai::completion::CompletionModel),
    /// OpenAI via rig-core (uses Responses API - better tool support)
    RigOpenAiResponses(rig_openai::responses_api::ResponsesCompletionModel),
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
    /// Z.AI (GLM models) via rig-zai
    RigZai(rig_zai::CompletionModel<reqwest::Client>),
}

// Note: A `complete!` macro was attempted here to unify completion calls across providers,
// but it cannot work because rig_anthropic_vertex returns a different CompletionResponse type
// than the standard rig providers. Each call site must use explicit match statements.

impl LlmClient {
    /// Check if this client uses an Anthropic model (Vertex AI or direct API).
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
        matches!(self, LlmClient::VertexAnthropic(_))
    }

    /// Get the provider name for logging and debugging.
    pub fn provider_name(&self) -> &'static str {
        match self {
            LlmClient::VertexAnthropic(_) => "vertex_ai_anthropic",
            LlmClient::RigOpenRouter(_) => "openrouter",
            LlmClient::RigOpenAi(_) => "openai",
            LlmClient::RigOpenAiResponses(_) => "openai_responses",
            LlmClient::RigAnthropic(_) => "anthropic",
            LlmClient::RigOllama(_) => "ollama",
            LlmClient::RigGemini(_) => "gemini",
            LlmClient::RigGroq(_) => "groq",
            LlmClient::RigXai(_) => "xai",
            LlmClient::RigZai(_) => "zai",
        }
    }
}

/// Configuration for creating an AgentBridge with OpenRouter
pub struct OpenRouterClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
}

/// Configuration for creating an AgentBridge with Vertex AI Anthropic
pub struct VertexAnthropicClientConfig<'a> {
    pub workspace: PathBuf,
    pub credentials_path: &'a str,
    pub project_id: &'a str,
    pub location: &'a str,
    pub model: &'a str,
}

/// Configuration for creating an AgentBridge with OpenAI
#[allow(dead_code)]
pub struct OpenAiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    pub base_url: Option<&'a str>,
    /// Reasoning effort level for reasoning models (e.g., "low", "medium", "high").
    /// Reserved for future use with models that support reasoning effort configuration.
    pub reasoning_effort: Option<&'a str>,
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

/// Configuration for creating an AgentBridge with Z.AI (GLM models)
pub struct ZaiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    /// Whether to use the coding-optimized endpoint
    pub use_coding_endpoint: bool,
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
        credentials_path: String,
        project_id: String,
        location: String,
    },
    /// OpenRouter API (access to multiple providers)
    Openrouter {
        workspace: String,
        model: String,
        api_key: String,
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
    /// Z.AI (GLM models)
    Zai {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default)]
        use_coding_endpoint: bool,
    },
}

#[allow(dead_code)] // Methods for future multi-provider config support
impl ProviderConfig {
    /// Get the workspace path from any variant.
    pub fn workspace(&self) -> &str {
        match self {
            Self::VertexAi { workspace, .. } => workspace,
            Self::Openrouter { workspace, .. } => workspace,
            Self::Openai { workspace, .. } => workspace,
            Self::Anthropic { workspace, .. } => workspace,
            Self::Ollama { workspace, .. } => workspace,
            Self::Gemini { workspace, .. } => workspace,
            Self::Groq { workspace, .. } => workspace,
            Self::Xai { workspace, .. } => workspace,
            Self::Zai { workspace, .. } => workspace,
        }
    }

    /// Get the model name from any variant.
    pub fn model(&self) -> &str {
        match self {
            Self::VertexAi { model, .. } => model,
            Self::Openrouter { model, .. } => model,
            Self::Openai { model, .. } => model,
            Self::Anthropic { model, .. } => model,
            Self::Ollama { model, .. } => model,
            Self::Gemini { model, .. } => model,
            Self::Groq { model, .. } => model,
            Self::Xai { model, .. } => model,
            Self::Zai { model, .. } => model,
        }
    }

    /// Get the provider name as a string.
    pub fn provider_name(&self) -> &'static str {
        match self {
            Self::VertexAi { .. } => "vertex_ai",
            Self::Openrouter { .. } => "openrouter",
            Self::Openai { .. } => "openai",
            Self::Anthropic { .. } => "anthropic",
            Self::Ollama { .. } => "ollama",
            Self::Gemini { .. } => "gemini",
            Self::Groq { .. } => "groq",
            Self::Xai { .. } => "xai",
            Self::Zai { .. } => "zai",
        }
    }
}
