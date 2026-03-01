//! Provider-specific model definitions.
//!
//! This module contains all static model definitions organized by provider.

use qbit_settings::schema::AiProvider;
use serde::{Deserialize, Serialize};

use crate::capabilities::ModelCapabilities;
use crate::registry::ModelDefinition;

/// Vertex AI (Anthropic Claude) model definitions.
pub fn vertex_ai_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "claude-opus-4-6@default",
            display_name: "Claude Opus 4.6",
            provider: AiProvider::VertexAi,
            capabilities: ModelCapabilities::anthropic_opus_4_6(),
            aliases: &[],
        },
        ModelDefinition {
            id: "claude-sonnet-4-6@default",
            display_name: "Claude Sonnet 4.6",
            provider: AiProvider::VertexAi,
            capabilities: ModelCapabilities::anthropic_sonnet_4_6(),
            aliases: &[],
        },
        ModelDefinition {
            id: "claude-opus-4-5@20251101",
            display_name: "Claude Opus 4.5",
            provider: AiProvider::VertexAi,
            capabilities: ModelCapabilities::anthropic_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "claude-sonnet-4-5@20250929",
            display_name: "Claude Sonnet 4.5",
            provider: AiProvider::VertexAi,
            capabilities: ModelCapabilities::anthropic_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "claude-haiku-4-5@20251001",
            display_name: "Claude Haiku 4.5",
            provider: AiProvider::VertexAi,
            capabilities: ModelCapabilities {
                max_output_tokens: 4_096, // Haiku has smaller output
                ..ModelCapabilities::anthropic_defaults()
            },
            aliases: &[],
        },
    ]
}

/// Direct Anthropic API model definitions.
pub fn anthropic_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "claude-sonnet-4-6-20260217",
            display_name: "Claude Sonnet 4.6",
            provider: AiProvider::Anthropic,
            capabilities: ModelCapabilities::anthropic_sonnet_4_6(),
            aliases: &["claude-sonnet-4-6"],
        },
        ModelDefinition {
            id: "claude-opus-4-5-20251101",
            display_name: "Claude Opus 4.5",
            provider: AiProvider::Anthropic,
            capabilities: ModelCapabilities::anthropic_defaults(),
            aliases: &["claude-opus-4-5"],
        },
        ModelDefinition {
            id: "claude-sonnet-4-5-20250929",
            display_name: "Claude Sonnet 4.5",
            provider: AiProvider::Anthropic,
            capabilities: ModelCapabilities::anthropic_defaults(),
            aliases: &["claude-sonnet-4-5"],
        },
        ModelDefinition {
            id: "claude-haiku-4-5-20251001",
            display_name: "Claude Haiku 4.5",
            provider: AiProvider::Anthropic,
            capabilities: ModelCapabilities {
                max_output_tokens: 4_096,
                ..ModelCapabilities::anthropic_defaults()
            },
            aliases: &["claude-haiku-4-5"],
        },
    ]
}

/// OpenAI model definitions.
pub fn openai_models() -> Vec<ModelDefinition> {
    vec![
        // GPT-5 series (reasoning models) - 400k context, 128k output
        ModelDefinition {
            id: "gpt-5.2",
            display_name: "GPT 5.2",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5.1",
            display_name: "GPT 5.1",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5",
            display_name: "GPT 5",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5-mini",
            display_name: "GPT 5 Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5-nano",
            display_name: "GPT 5 Nano",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
        // GPT-4.1 series
        ModelDefinition {
            id: "gpt-4.1",
            display_name: "GPT 4.1",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-4.1-mini",
            display_name: "GPT 4.1 Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-4.1-nano",
            display_name: "GPT 4.1 Nano",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        // GPT-4o series
        ModelDefinition {
            id: "gpt-4o",
            display_name: "GPT 4o",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-4o-mini",
            display_name: "GPT 4o Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "chatgpt-4o-latest",
            display_name: "ChatGPT 4o Latest",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_gpt4_defaults(),
            aliases: &[],
        },
        // o-series reasoning models - 200k context, 100k output
        ModelDefinition {
            id: "o4-mini",
            display_name: "o4 Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_o_series_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "o3",
            display_name: "o3",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_o_series_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "o3-mini",
            display_name: "o3 Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_o_series_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "o1",
            display_name: "o1",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_o_series_defaults(),
            aliases: &["o1-preview"],
        },
        // Codex models
        ModelDefinition {
            id: "gpt-5.2-codex",
            display_name: "GPT 5.2 Codex",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_codex_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5.1-codex",
            display_name: "GPT 5.1 Codex",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_codex_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5.1-codex-max",
            display_name: "GPT 5.1 Codex Max",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_codex_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gpt-5.1-codex-mini",
            display_name: "GPT 5.1 Codex Mini",
            provider: AiProvider::Openai,
            capabilities: ModelCapabilities::openai_codex_defaults(),
            aliases: &[],
        },
    ]
}

/// Gemini model definitions.
pub fn gemini_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "gemini-3-pro-preview",
            display_name: "Gemini 3 Pro Preview",
            provider: AiProvider::Gemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-pro",
            display_name: "Gemini 2.5 Pro",
            provider: AiProvider::Gemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-flash",
            display_name: "Gemini 2.5 Flash",
            provider: AiProvider::Gemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-flash-lite",
            display_name: "Gemini 2.5 Flash Lite",
            provider: AiProvider::Gemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.0-flash-thinking-exp",
            display_name: "Gemini 2.0 Flash Thinking",
            provider: AiProvider::Gemini,
            capabilities: ModelCapabilities {
                supports_thinking_history: true,
                ..ModelCapabilities::gemini_defaults()
            },
            aliases: &[],
        },
    ]
}

/// Vertex AI Gemini model definitions.
///
/// These are Gemini models accessed via Google Cloud Vertex AI (using
/// service account or ADC authentication), as opposed to the `gemini_models()`
/// which use the AI Studio API.
pub fn vertex_gemini_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "gemini-3-pro-preview",
            display_name: "Gemini 3 Pro Preview",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-3-flash-preview",
            display_name: "Gemini 3 Flash Preview",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-pro",
            display_name: "Gemini 2.5 Pro",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-flash",
            display_name: "Gemini 2.5 Flash",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.5-flash-lite",
            display_name: "Gemini 2.5 Flash Lite",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.0-flash",
            display_name: "Gemini 2.0 Flash",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "gemini-2.0-flash-lite",
            display_name: "Gemini 2.0 Flash Lite",
            provider: AiProvider::VertexGemini,
            capabilities: ModelCapabilities::gemini_2_0_flash_lite_defaults(),
            aliases: &[],
        },
    ]
}

/// Groq model definitions.
pub fn groq_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "meta-llama/llama-4-scout-17b-16e-instruct",
            display_name: "Llama 4 Scout 17B",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &["llama-4-scout"],
        },
        ModelDefinition {
            id: "meta-llama/llama-4-maverick-17b-128e-instruct",
            display_name: "Llama 4 Maverick 17B",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &["llama-4-maverick"],
        },
        ModelDefinition {
            id: "llama-3.3-70b-versatile",
            display_name: "Llama 3.3 70B",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "llama-3.1-8b-instant",
            display_name: "Llama 3.1 8B Instant",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "openai/gpt-oss-120b",
            display_name: "GPT OSS 120B",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &["gpt-oss-120b"],
        },
        ModelDefinition {
            id: "openai/gpt-oss-20b",
            display_name: "GPT OSS 20B",
            provider: AiProvider::Groq,
            capabilities: ModelCapabilities::groq_defaults(),
            aliases: &["gpt-oss-20b"],
        },
    ]
}

/// xAI (Grok) model definitions.
pub fn xai_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "grok-4-1-fast-reasoning",
            display_name: "Grok 4.1 Fast (Reasoning)",
            provider: AiProvider::Xai,
            capabilities: ModelCapabilities {
                supports_thinking_history: true,
                ..ModelCapabilities::xai_defaults()
            },
            aliases: &[],
        },
        ModelDefinition {
            id: "grok-4-1-fast-non-reasoning",
            display_name: "Grok 4.1 Fast",
            provider: AiProvider::Xai,
            capabilities: ModelCapabilities::xai_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "grok-4-fast-reasoning",
            display_name: "Grok 4 (Reasoning)",
            provider: AiProvider::Xai,
            capabilities: ModelCapabilities {
                supports_thinking_history: true,
                ..ModelCapabilities::xai_defaults()
            },
            aliases: &[],
        },
        ModelDefinition {
            id: "grok-4-fast-non-reasoning",
            display_name: "Grok 4",
            provider: AiProvider::Xai,
            capabilities: ModelCapabilities::xai_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "grok-code-fast-1",
            display_name: "Grok Code",
            provider: AiProvider::Xai,
            capabilities: ModelCapabilities::xai_defaults(),
            aliases: &[],
        },
    ]
}

/// Z.AI SDK model definitions.
pub fn zai_sdk_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "glm-5",
            display_name: "GLM 5",
            provider: AiProvider::ZaiSdk,
            capabilities: ModelCapabilities::zai_thinking_defaults(),
            aliases: &["GLM-5"],
        },
        ModelDefinition {
            id: "glm-4.7",
            display_name: "GLM 4.7",
            provider: AiProvider::ZaiSdk,
            capabilities: ModelCapabilities::zai_thinking_defaults(),
            aliases: &["GLM-4.7"],
        },
        ModelDefinition {
            id: "glm-4.6v",
            display_name: "GLM 4.6v",
            provider: AiProvider::ZaiSdk,
            capabilities: ModelCapabilities::zai_vision_defaults(),
            aliases: &["GLM-4.6v"],
        },
        ModelDefinition {
            id: "glm-4.5-air",
            display_name: "GLM 4.5 Air",
            provider: AiProvider::ZaiSdk,
            capabilities: ModelCapabilities::zai_defaults(),
            aliases: &["GLM-4.5-air"],
        },
        ModelDefinition {
            id: "glm-4-flash",
            display_name: "GLM 4 Flash",
            provider: AiProvider::ZaiSdk,
            capabilities: ModelCapabilities::zai_defaults(),
            aliases: &["GLM-4-flash"],
        },
    ]
}

/// Ollama default model definitions.
///
/// Note: Ollama models vary by installation. These are common defaults.
/// Use `discover_ollama_models()` in the registry module for dynamic discovery.
pub fn ollama_default_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "llama3.2",
            display_name: "Llama 3.2",
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
            aliases: &["llama3.2:latest"],
        },
        ModelDefinition {
            id: "llama3.1",
            display_name: "Llama 3.1",
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
            aliases: &["llama3.1:latest"],
        },
        ModelDefinition {
            id: "qwen2.5",
            display_name: "Qwen 2.5",
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
            aliases: &["qwen2.5:latest"],
        },
        ModelDefinition {
            id: "mistral",
            display_name: "Mistral",
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
            aliases: &["mistral:latest"],
        },
        ModelDefinition {
            id: "codellama",
            display_name: "CodeLlama",
            provider: AiProvider::Ollama,
            capabilities: ModelCapabilities::ollama_defaults(),
            aliases: &["codellama:latest"],
        },
    ]
}

/// OpenRouter model definitions.
///
/// Note: OpenRouter provides access to many models. These are curated defaults.
pub fn openrouter_models() -> Vec<ModelDefinition> {
    vec![
        ModelDefinition {
            id: "mistralai/devstral-2512",
            display_name: "Devstral 2512",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "deepseek/deepseek-v3.2",
            display_name: "Deepseek v3.2",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "z-ai/glm-4.6",
            display_name: "GLM 4.6",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "x-ai/grok-code-fast-1",
            display_name: "Grok Code Fast 1",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "openai/gpt-oss-20b",
            display_name: "GPT OSS 20B",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "openai/gpt-oss-120b",
            display_name: "GPT OSS 120B",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::conservative_defaults(),
            aliases: &[],
        },
        ModelDefinition {
            id: "openai/gpt-5.2",
            display_name: "GPT 5.2",
            provider: AiProvider::Openrouter,
            capabilities: ModelCapabilities::openai_gpt5_defaults(),
            aliases: &[],
        },
    ]
}

/// Provider metadata for UI display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub provider: AiProvider,
    pub name: &'static str,
    pub icon: &'static str,
    pub description: &'static str,
}

/// Get provider metadata.
pub fn get_provider_info(provider: AiProvider) -> ProviderInfo {
    match provider {
        AiProvider::VertexAi => ProviderInfo {
            provider,
            name: "Vertex AI",
            icon: "🔷",
            description: "Claude on Google Cloud",
        },
        AiProvider::Anthropic => ProviderInfo {
            provider,
            name: "Anthropic",
            icon: "🔶",
            description: "Direct Claude API access",
        },
        AiProvider::Openai => ProviderInfo {
            provider,
            name: "OpenAI",
            icon: "⚪",
            description: "GPT and o-series models",
        },
        AiProvider::Gemini => ProviderInfo {
            provider,
            name: "Gemini",
            icon: "💎",
            description: "Google Gemini models",
        },
        AiProvider::Groq => ProviderInfo {
            provider,
            name: "Groq",
            icon: "⚡",
            description: "Fast inference for open models",
        },
        AiProvider::Xai => ProviderInfo {
            provider,
            name: "xAI",
            icon: "𝕏",
            description: "Grok models",
        },
        AiProvider::ZaiSdk => ProviderInfo {
            provider,
            name: "Z.AI SDK",
            icon: "🤖",
            description: "Z.AI GLM models",
        },
        AiProvider::Ollama => ProviderInfo {
            provider,
            name: "Ollama",
            icon: "🦙",
            description: "Local inference",
        },
        AiProvider::Openrouter => ProviderInfo {
            provider,
            name: "OpenRouter",
            icon: "🔀",
            description: "Access multiple providers",
        },
        AiProvider::VertexGemini => ProviderInfo {
            provider,
            name: "Vertex Gemini",
            icon: "🔷",
            description: "Gemini on Google Cloud",
        },
    }
}

/// Get all provider metadata.
pub fn get_all_provider_info() -> Vec<ProviderInfo> {
    vec![
        get_provider_info(AiProvider::VertexAi),
        get_provider_info(AiProvider::VertexGemini),
        get_provider_info(AiProvider::Anthropic),
        get_provider_info(AiProvider::Openai),
        get_provider_info(AiProvider::Gemini),
        get_provider_info(AiProvider::Groq),
        get_provider_info(AiProvider::Xai),
        get_provider_info(AiProvider::ZaiSdk),
        get_provider_info(AiProvider::Ollama),
        get_provider_info(AiProvider::Openrouter),
    ]
}
