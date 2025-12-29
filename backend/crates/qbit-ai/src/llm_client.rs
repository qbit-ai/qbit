//! LLM client abstraction for the agent system.
//!
//! This module re-exports types from `qbit-llm-providers` and provides
//! component creation functions for the agent system.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rig::client::CompletionClient;
use rig::providers::anthropic as rig_anthropic;
use rig::providers::gemini as rig_gemini;
use rig::providers::groq as rig_groq;
use rig::providers::ollama as rig_ollama;
use rig::providers::openai as rig_openai;
use rig::providers::openrouter as rig_openrouter;
use rig::providers::xai as rig_xai;
use tokio::sync::RwLock;

use vtcode_core::tools::ToolRegistry;

use qbit_context::ContextManager;
use qbit_hitl::ApprovalRecorder;
use qbit_loop_detection::LoopDetector;
use qbit_sub_agents::{create_default_sub_agents, SubAgentRegistry};
use qbit_tool_policy::ToolPolicyManager;

// Re-export types from qbit-llm-providers for backward compatibility
pub use qbit_llm_providers::{
    AnthropicClientConfig, GeminiClientConfig, GroqClientConfig, LlmClient, OllamaClientConfig,
    OpenAiClientConfig, OpenRouterClientConfig, ProviderConfig, VertexAnthropicClientConfig,
    XaiClientConfig, ZaiClientConfig,
};

// Re-export ContextManagerConfig for convenience (also used internally)
pub use qbit_context::ContextManagerConfig;

/// Common initialization result containing shared components
pub struct AgentBridgeComponents {
    pub workspace: Arc<RwLock<PathBuf>>,
    pub provider_name: String,
    pub model_name: String,
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    pub client: Arc<RwLock<LlmClient>>,
    pub sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,
    pub approval_recorder: Arc<ApprovalRecorder>,
    pub tool_policy_manager: Arc<ToolPolicyManager>,
    pub context_manager: Arc<ContextManager>,
    pub loop_detector: Arc<RwLock<LoopDetector>>,
}

/// Shared components that are common to all LLM providers.
struct SharedComponents {
    tool_registry: Arc<RwLock<ToolRegistry>>,
    sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,
    approval_recorder: Arc<ApprovalRecorder>,
    tool_policy_manager: Arc<ToolPolicyManager>,
    context_manager: Arc<ContextManager>,
    loop_detector: Arc<RwLock<LoopDetector>>,
}

/// Initialize shared components from a workspace path and model name.
///
/// If `context_config` is provided, the ContextManager will be created with those settings.
/// Otherwise, it will use the model's defaults (with context management disabled by default).
async fn create_shared_components(
    workspace: &Path,
    model: &str,
    context_config: Option<ContextManagerConfig>,
) -> SharedComponents {
    // Create and populate the sub-agent registry
    let mut sub_agent_registry = SubAgentRegistry::new();
    sub_agent_registry.register_multiple(create_default_sub_agents());

    // Create context manager with config if provided, otherwise use model defaults
    let context_manager = match context_config {
        Some(config) => {
            tracing::debug!(
                "[context] Creating ContextManager with config: enabled={}, threshold={:.2}, protected_turns={}, cooldown={}s",
                config.enabled,
                config.compaction_threshold,
                config.protected_turns,
                config.cooldown_seconds
            );
            ContextManager::with_config(model, config)
        }
        None => {
            tracing::debug!(
                "[context] Creating ContextManager with model defaults (context management disabled)"
            );
            ContextManager::for_model(model)
        }
    };

    SharedComponents {
        tool_registry: Arc::new(RwLock::new(
            ToolRegistry::new(workspace.to_path_buf()).await,
        )),
        sub_agent_registry: Arc::new(RwLock::new(sub_agent_registry)),
        approval_recorder: Arc::new(
            ApprovalRecorder::new(workspace.join(".qbit").join("hitl")).await,
        ),
        tool_policy_manager: Arc::new(ToolPolicyManager::new(workspace).await),
        context_manager: Arc::new(context_manager),
        loop_detector: Arc::new(RwLock::new(LoopDetector::with_defaults())),
    }
}

/// Create components for an OpenRouter-based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_openrouter_components(
    config: OpenRouterClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let openrouter_client = rig_openrouter::Client::new(config.api_key);
    let completion_model = openrouter_client.completion_model(config.model);
    let client = LlmClient::RigOpenRouter(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "openrouter".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for a Vertex AI Anthropic based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_vertex_components(
    config: VertexAnthropicClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let vertex_client = rig_anthropic_vertex::Client::from_service_account(
        config.credentials_path,
        config.project_id,
        config.location,
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create Vertex AI client: {}", e))?;

    // Enable extended thinking with default budget (10,000 tokens)
    // When thinking is enabled, temperature is automatically set to 1
    let completion_model = vertex_client
        .completion_model(config.model)
        .with_default_thinking();

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "anthropic_vertex".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(LlmClient::VertexAnthropic(completion_model))),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for an OpenAI-based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_openai_components(
    config: OpenAiClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    // Note: rig-core's OpenAI client doesn't support custom base URLs directly.
    // The base_url config option is reserved for future use or alternative clients.
    if config.base_url.is_some() {
        tracing::warn!("Custom base_url is not yet supported for OpenAI provider, ignoring");
    }

    // Create OpenAI client
    let openai_client = rig_openai::Client::new(config.api_key);

    // Create the completion model using Chat Completions API (not Responses API)
    // The Chat Completions API has better compatibility with GPT-5.x models
    // Note: reasoning_effort is stored in config but applied at request time if needed
    let completion_model = openai_client
        .completion_model(config.model)
        .completions_api();
    let client = LlmClient::RigOpenAi(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "openai".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for a direct Anthropic API client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_anthropic_components(
    config: AnthropicClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let anthropic_client = rig_anthropic::Client::new(config.api_key);
    let completion_model = anthropic_client.completion_model(config.model);
    let client = LlmClient::RigAnthropic(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "anthropic".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for an Ollama-based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_ollama_components(
    config: OllamaClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    // Note: rig-core's Ollama client only supports the default localhost:11434 endpoint.
    // The base_url config option is reserved for future use when rig-core adds this feature.
    if config.base_url.is_some() {
        tracing::warn!(
            "Custom base_url is not yet supported for Ollama provider (rig-core defaults to http://localhost:11434), ignoring"
        );
    }

    // Create Ollama client (defaults to http://localhost:11434)
    let ollama_client = rig_ollama::Client::new();
    let completion_model = ollama_client.completion_model(config.model);
    let client = LlmClient::RigOllama(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "ollama".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for a Gemini-based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_gemini_components(
    config: GeminiClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let gemini_client = rig_gemini::Client::new(config.api_key);
    let completion_model = gemini_client.completion_model(config.model);
    let client = LlmClient::RigGemini(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "gemini".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for a Groq-based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_groq_components(
    config: GroqClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let groq_client = rig_groq::Client::new(config.api_key);
    let completion_model = groq_client.completion_model(config.model);
    let client = LlmClient::RigGroq(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "groq".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for an xAI (Grok) based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_xai_components(
    config: XaiClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    let xai_client = rig_xai::Client::new(config.api_key);
    let completion_model = xai_client.completion_model(config.model);
    let client = LlmClient::RigXai(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "xai".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}

/// Create components for a Z.AI (GLM models) based client.
///
/// If `context_config` is provided, the ContextManager will use those settings.
/// Otherwise, it will use the model's defaults (context management disabled).
pub async fn create_zai_components(
    config: ZaiClientConfig<'_>,
    context_config: Option<ContextManagerConfig>,
) -> Result<AgentBridgeComponents> {
    // The rig-zai client defaults to the coding API endpoint
    // The use_coding_endpoint flag is for future extensibility (e.g., general API)
    // Currently, all Z.AI requests use the coding API
    let zai_client = if config.use_coding_endpoint {
        rig_zai::Client::new(config.api_key)
    } else {
        // For non-coding endpoint, we'd use a different base URL
        // Currently defaults to coding API as it's the primary use case
        tracing::warn!("Non-coding Z.AI endpoint not yet implemented, using coding API");
        rig_zai::Client::new(config.api_key)
    };
    let completion_model = zai_client.completion_model(config.model);
    let client = LlmClient::RigZai(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, context_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "zai".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
    })
}
