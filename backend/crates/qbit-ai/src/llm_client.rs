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

use qbit_tools::{ToolRegistry, ToolRegistryConfig};

use qbit_context::ContextManager;
use qbit_hitl::ApprovalRecorder;
use qbit_loop_detection::LoopDetector;
use qbit_sub_agents::{create_default_sub_agents, SubAgentRegistry};
use qbit_tool_policy::ToolPolicyManager;

// Re-export types from qbit-llm-providers for backward compatibility
pub use qbit_llm_providers::{
    AnthropicClientConfig, GeminiClientConfig, GroqClientConfig, LlmClient, OllamaClientConfig,
    OpenAiClientConfig, OpenRouterClientConfig, ProviderConfig, VertexAnthropicClientConfig,
    XaiClientConfig, ZaiAnthropicClientConfig, ZaiClientConfig,
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
    /// OpenAI web search configuration (if enabled)
    pub openai_web_search_config: Option<qbit_llm_providers::OpenAiWebSearchConfig>,
    /// OpenAI reasoning effort level (if set)
    pub openai_reasoning_effort: Option<String>,
    /// Factory for creating sub-agent model override clients (optional, lazy-init)
    pub model_factory: Option<Arc<LlmClientFactory>>,
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

/// Configuration for shared components.
#[derive(Default, Clone)]
pub struct SharedComponentsConfig {
    /// Settings instance.
    pub settings: qbit_settings::QbitSettings,
    /// Context manager configuration.
    pub context_config: Option<ContextManagerConfig>,
}

/// Initialize shared components from a workspace path and model name.
///
/// If `context_config` is provided, the ContextManager will be created with those settings.
/// Otherwise, it will use the model's defaults (with context management disabled by default).
async fn create_shared_components(
    workspace: &Path,
    model: &str,
    config: SharedComponentsConfig,
) -> SharedComponents {
    // Create and populate the sub-agent registry
    let mut sub_agent_registry = SubAgentRegistry::new();
    sub_agent_registry.register_multiple(create_default_sub_agents());

    // Create context manager with config if provided, otherwise use model defaults
    let context_manager = match config.context_config {
        Some(ctx_config) => {
            tracing::debug!(
                "[context] Creating ContextManager with config: enabled={}, threshold={:.2}, protected_turns={}, cooldown={}s",
                ctx_config.enabled,
                ctx_config.compaction_threshold,
                ctx_config.protected_turns,
                ctx_config.cooldown_seconds
            );
            ContextManager::with_config(model, ctx_config)
        }
        None => {
            tracing::debug!(
                "[context] Creating ContextManager with model defaults (context management disabled)"
            );
            ContextManager::for_model(model)
        }
    };

    // Create tool registry with config options
    let tool_registry_config = ToolRegistryConfig {
        settings: config.settings.clone(),
    };
    if config.settings.terminal.shell.is_some() {
        tracing::debug!("[tools] Creating ToolRegistry with shell override from settings");
    }

    SharedComponents {
        tool_registry: Arc::new(RwLock::new(
            ToolRegistry::with_config(workspace.to_path_buf(), tool_registry_config).await,
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
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_openrouter_components(
    config: OpenRouterClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let openrouter_client = rig_openrouter::Client::new(config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenRouter client: {}", e))?;
    let completion_model = openrouter_client.completion_model(config.model);
    let client = LlmClient::RigOpenRouter(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for a Vertex AI Anthropic based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_vertex_components(
    config: VertexAnthropicClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let vertex_client = match config.credentials_path {
        Some(path) => rig_anthropic_vertex::Client::from_service_account(
            path,
            config.project_id,
            config.location,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create Vertex AI client: {}", e))?,
        None => rig_anthropic_vertex::Client::from_env(config.project_id, config.location)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Vertex AI client from env: {}", e))?,
    };

    // Enable extended thinking with default budget (10,000 tokens)
    // When thinking is enabled, temperature is automatically set to 1
    // Also enable Claude's native web search (web_search_20250305)
    // Note: web_fetch_20250910 requires a beta header not yet supported on Vertex AI
    let completion_model = vertex_client
        .completion_model(config.model)
        .with_default_thinking()
        .with_web_search();

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for an OpenAI-based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
///
/// For reasoning models (o1, o3, o4, gpt-5.x), this uses a custom provider with explicit
/// streaming event separation to ensure reasoning deltas are never mixed with text deltas.
pub async fn create_openai_components(
    config: OpenAiClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    // Note: rig-core's OpenAI client doesn't support custom base URLs directly.
    // The base_url config option is reserved for future use or alternative clients.
    if config.base_url.is_some() {
        tracing::warn!("Custom base_url is not yet supported for OpenAI provider, ignoring");
    }

    // Check if this is a reasoning model that needs special handling
    let is_reasoning = rig_openai_responses::is_reasoning_model(config.model);

    tracing::info!(
        target: "qbit::provider",
        "╔══════════════════════════════════════════════════════════════╗"
    );
    tracing::info!(
        target: "qbit::provider",
        "║ OpenAI Provider Selection                                    ║"
    );
    tracing::info!(
        target: "qbit::provider",
        "╠══════════════════════════════════════════════════════════════╣"
    );
    tracing::info!(
        target: "qbit::provider",
        "║ Model: {:<54}║",
        config.model
    );
    tracing::info!(
        target: "qbit::provider",
        "║ Is Reasoning Model: {:<41}║",
        if is_reasoning { "YES" } else { "NO" }
    );

    let (client, provider_name) = if is_reasoning {
        tracing::info!(
            target: "qbit::provider",
            "║ Provider: rig-openai-responses (custom)                      ║"
        );
        tracing::info!(
            target: "qbit::provider",
            "║ Features: Explicit reasoning/text event separation           ║"
        );

        let openai_client = rig_openai_responses::Client::new(config.api_key);
        let mut completion_model = openai_client.completion_model(config.model);

        // Set reasoning effort if provided
        if let Some(effort_str) = config.reasoning_effort {
            let effort = match effort_str.to_lowercase().as_str() {
                "low" => rig_openai_responses::ReasoningEffort::Low,
                "high" => rig_openai_responses::ReasoningEffort::High,
                _ => rig_openai_responses::ReasoningEffort::Medium,
            };
            completion_model = completion_model.with_reasoning_effort(effort);
            tracing::info!(
                target: "qbit::provider",
                "║ Reasoning Effort: {:<43}║",
                effort_str.to_uppercase()
            );
        }

        tracing::info!(
            target: "qbit::provider",
            "╚══════════════════════════════════════════════════════════════╝"
        );

        (
            LlmClient::OpenAiReasoning(completion_model),
            "openai_reasoning".to_string(),
        )
    } else {
        tracing::info!(
            target: "qbit::provider",
            "║ Provider: rig-core responses_api (built-in)                  ║"
        );
        tracing::info!(
            target: "qbit::provider",
            "╚══════════════════════════════════════════════════════════════╝"
        );

        // Use rig-core's built-in Responses API for non-reasoning models
        let openai_client = rig_openai::Client::new(config.api_key)
            .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;

        // Create the completion model using Responses API (default)
        // The Responses API has better tool support. Our sanitize_schema function handles
        // strict mode compatibility by making optional parameters nullable.
        let completion_model = openai_client.completion_model(config.model);
        (
            LlmClient::RigOpenAiResponses(completion_model),
            "openai_responses".to_string(),
        )
    };

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

    // Create web search config if enabled
    let openai_web_search_config = if config.enable_web_search {
        tracing::info!(
            "OpenAI web search enabled with context_size={}",
            config.web_search_context_size
        );
        Some(qbit_llm_providers::OpenAiWebSearchConfig {
            search_context_size: config.web_search_context_size.to_string(),
            user_location: None, // Could add user location from settings later
        })
    } else {
        None
    };

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        // Provider name distinguishes between reasoning and non-reasoning variants
        provider_name,
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
        openai_web_search_config,
        openai_reasoning_effort: config.reasoning_effort.map(|s| s.to_string()),
        model_factory: None,
    })
}

/// Create components for a direct Anthropic API client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_anthropic_components(
    config: AnthropicClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let anthropic_client = rig_anthropic::Client::new(config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create Anthropic client: {}", e))?;
    let completion_model = anthropic_client.completion_model(config.model);
    let client = LlmClient::RigAnthropic(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for an Ollama-based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_ollama_components(
    config: OllamaClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    // Note: rig-core's Ollama client only supports the default localhost:11434 endpoint.
    // The base_url config option is reserved for future use when rig-core adds this feature.
    if config.base_url.is_some() {
        tracing::warn!(
            "Custom base_url is not yet supported for Ollama provider (rig-core defaults to http://localhost:11434), ignoring"
        );
    }

    // Create Ollama client using builder (defaults to http://localhost:11434)
    // Ollama doesn't require an API key, so we use client::Nothing
    let ollama_client = rig_ollama::Client::builder()
        .api_key(rig::client::Nothing)
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to create Ollama client: {}", e))?;
    let completion_model = ollama_client.completion_model(config.model);
    let client = LlmClient::RigOllama(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for a Gemini-based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_gemini_components(
    config: GeminiClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let gemini_client = rig_gemini::Client::new(config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create Gemini client: {}", e))?;
    let completion_model = gemini_client.completion_model(config.model);
    let client = LlmClient::RigGemini(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for a Groq-based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_groq_components(
    config: GroqClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let groq_client = rig_groq::Client::new(config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create Groq client: {}", e))?;
    let completion_model = groq_client.completion_model(config.model);
    let client = LlmClient::RigGroq(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for an xAI (Grok) based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_xai_components(
    config: XaiClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    let xai_client = rig_xai::Client::new(config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create xAI client: {}", e))?;
    let completion_model = xai_client.completion_model(config.model);
    let client = LlmClient::RigXai(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create components for a Z.AI (GLM models) based client.
///
/// The `shared_config` parameter allows configuring context management and shell override.
/// If not provided, defaults are used (context management disabled, no shell override).
pub async fn create_zai_components(
    config: ZaiClientConfig<'_>,
    shared_config: SharedComponentsConfig,
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

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

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
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

/// Create AgentBridge components for Z.AI via Anthropic-compatible API.
///
/// Uses debug logging to capture raw HTTP responses for troubleshooting.
pub async fn create_zai_anthropic_components(
    config: ZaiAnthropicClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    // Use logging client to debug Z.AI response format issues
    let zai_client = rig_zai_anthropic::new_with_logging(config.api_key);
    let completion_model = zai_client.completion_model(config.model);
    let client = LlmClient::RigZaiAnthropicLogging(completion_model);

    let shared = create_shared_components(&config.workspace, config.model, shared_config).await;

    Ok(AgentBridgeComponents {
        workspace: Arc::new(RwLock::new(config.workspace)),
        provider_name: "zai_anthropic".to_string(),
        model_name: config.model.to_string(),
        tool_registry: shared.tool_registry,
        client: Arc::new(RwLock::new(client)),
        sub_agent_registry: shared.sub_agent_registry,
        approval_recorder: shared.approval_recorder,
        tool_policy_manager: shared.tool_policy_manager,
        context_manager: shared.context_manager,
        loop_detector: shared.loop_detector,
        openai_web_search_config: None,
        openai_reasoning_effort: None,
        model_factory: None,
    })
}

// =============================================================================
// LLM Client Factory for Sub-Agent Model Overrides
// =============================================================================

use qbit_settings::schema::AiProvider;
use std::collections::HashMap;

/// Factory for creating and caching LLM client instances.
///
/// Used primarily for sub-agent model overrides, where a sub-agent might use
/// a different model than the main agent.
pub struct LlmClientFactory {
    /// Cached clients by (provider_name, model_name) key
    cache: RwLock<HashMap<(String, String), Arc<LlmClient>>>,
    /// Settings manager for credential lookup
    settings_manager: Arc<qbit_settings::SettingsManager>,
    /// Workspace path for shared components (reserved for future use)
    #[allow(dead_code)]
    workspace: PathBuf,
}

impl LlmClientFactory {
    /// Create a new factory with settings manager and workspace.
    pub fn new(settings_manager: Arc<qbit_settings::SettingsManager>, workspace: PathBuf) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            settings_manager,
            workspace,
        }
    }

    /// Get or create an LLM client for the specified provider and model.
    ///
    /// Clients are cached by (provider, model) to avoid recreating them.
    /// Returns an error if credentials are missing or invalid.
    pub async fn get_or_create(&self, provider: &str, model: &str) -> Result<Arc<LlmClient>> {
        let key = (provider.to_string(), model.to_string());

        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(client) = cache.get(&key) {
                tracing::debug!("LlmClientFactory: cache hit for {}/{}", provider, model);
                return Ok(client.clone());
            }
        }

        // Create new client
        tracing::info!(
            "LlmClientFactory: creating client for {}/{}",
            provider,
            model
        );
        let client = self.create_client(provider, model).await?;
        let client = Arc::new(client);

        // Cache it
        self.cache.write().await.insert(key, client.clone());
        Ok(client)
    }

    /// Create a new LLM client for the given provider and model.
    async fn create_client(&self, provider: &str, model: &str) -> Result<LlmClient> {
        let settings = self.settings_manager.get().await;

        // Parse provider string to AiProvider enum
        let ai_provider: AiProvider = provider
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid provider '{}': {}", provider, e))?;

        match ai_provider {
            AiProvider::VertexAi => {
                let vertex_settings = &settings.ai.vertex_ai;
                let project_id = vertex_settings
                    .project_id
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Vertex AI project_id not configured"))?;
                let location = vertex_settings
                    .location
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Vertex AI location not configured"))?;

                let vertex_client = match &vertex_settings.credentials_path {
                    Some(path) => rig_anthropic_vertex::Client::from_service_account(
                        path, project_id, location,
                    )
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create Vertex AI client: {}", e))?,
                    None => rig_anthropic_vertex::Client::from_env(project_id, location)
                        .await
                        .map_err(|e| {
                            anyhow::anyhow!("Failed to create Vertex AI client from env: {}", e)
                        })?,
                };

                let completion_model = vertex_client
                    .completion_model(model)
                    .with_default_thinking()
                    .with_web_search();

                Ok(LlmClient::VertexAnthropic(completion_model))
            }
            AiProvider::Openrouter => {
                let api_key = settings
                    .ai
                    .openrouter
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OpenRouter API key not configured"))?;

                let client = rig_openrouter::Client::new(api_key)
                    .map_err(|e| anyhow::anyhow!("Failed to create OpenRouter client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigOpenRouter(completion_model))
            }
            AiProvider::Anthropic => {
                let api_key = settings
                    .ai
                    .anthropic
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Anthropic API key not configured"))?;

                let client = rig_anthropic::Client::new(api_key)
                    .map_err(|e| anyhow::anyhow!("Failed to create Anthropic client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigAnthropic(completion_model))
            }
            AiProvider::Openai => {
                let api_key = settings
                    .ai
                    .openai
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("OpenAI API key not configured"))?;

                // Use custom provider for reasoning models
                let is_reasoning = rig_openai_responses::is_reasoning_model(model);
                tracing::info!(
                    target: "qbit::provider",
                    "[LlmClientFactory] OpenAI model={} is_reasoning={} provider={}",
                    model,
                    is_reasoning,
                    if is_reasoning { "rig-openai-responses" } else { "rig-core" }
                );

                if is_reasoning {
                    let client = rig_openai_responses::Client::new(api_key);
                    let completion_model = client.completion_model(model);
                    Ok(LlmClient::OpenAiReasoning(completion_model))
                } else {
                    let client = rig_openai::Client::new(api_key)
                        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
                    let completion_model = client.completion_model(model);
                    Ok(LlmClient::RigOpenAiResponses(completion_model))
                }
            }
            AiProvider::Ollama => {
                let client = rig_ollama::Client::builder()
                    .api_key(rig::client::Nothing)
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to create Ollama client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigOllama(completion_model))
            }
            AiProvider::Gemini => {
                let api_key = settings
                    .ai
                    .gemini
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Gemini API key not configured"))?;

                let client = rig_gemini::Client::new(api_key)
                    .map_err(|e| anyhow::anyhow!("Failed to create Gemini client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigGemini(completion_model))
            }
            AiProvider::Groq => {
                let api_key = settings
                    .ai
                    .groq
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Groq API key not configured"))?;

                let client = rig_groq::Client::new(api_key)
                    .map_err(|e| anyhow::anyhow!("Failed to create Groq client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigGroq(completion_model))
            }
            AiProvider::Xai => {
                let api_key = settings
                    .ai
                    .xai
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("xAI API key not configured"))?;

                let client = rig_xai::Client::new(api_key)
                    .map_err(|e| anyhow::anyhow!("Failed to create xAI client: {}", e))?;
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigXai(completion_model))
            }
            AiProvider::Zai => {
                let api_key = settings
                    .ai
                    .zai
                    .api_key
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Z.AI API key not configured"))?;

                let client = rig_zai::Client::new(api_key);
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigZai(completion_model))
            }
            AiProvider::ZaiAnthropic => {
                let api_key =
                    settings.ai.zai_anthropic.api_key.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("Z.AI (Anthropic) API key not configured")
                    })?;

                let client = rig_zai_anthropic::new(api_key);
                let completion_model = client.completion_model(model);

                Ok(LlmClient::RigZaiAnthropic(completion_model))
            }
        }
    }

    /// Clear the cache (useful for testing or when settings change).
    #[allow(dead_code)]
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }
}
