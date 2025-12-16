//! LLM client abstraction for the agent system.
//!
//! This module provides a unified interface for interacting with different LLM providers:
//! - OpenRouter via rig-core (supports tools and system prompts)
//! - Anthropic on Vertex AI via rig-anthropic-vertex
//! - OpenAI via rig-core

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use rig::client::CompletionClient;
use rig::providers::openai as rig_openai;
use rig::providers::openrouter as rig_openrouter;
use tokio::sync::RwLock;

use crate::compat::tools::ToolRegistry;

use super::context_manager::ContextManager;
use super::hitl::ApprovalRecorder;
use super::loop_detection::LoopDetector;
use super::sub_agent::{create_default_sub_agents, SubAgentRegistry};
use super::tool_policy::ToolPolicyManager;

/// LLM client abstraction for different providers
pub enum LlmClient {
    /// Anthropic on Vertex AI via rig-anthropic-vertex
    VertexAnthropic(rig_anthropic_vertex::CompletionModel),
    /// OpenRouter via rig-core (supports tools and system prompts)
    RigOpenRouter(rig_openrouter::CompletionModel),
    /// OpenAI via rig-core (uses Chat Completions API for better compatibility)
    RigOpenAi(rig_openai::completion::CompletionModel),
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
async fn create_shared_components(workspace: &Path, model: &str) -> SharedComponents {
    // Create and populate the sub-agent registry
    let mut sub_agent_registry = SubAgentRegistry::new();
    sub_agent_registry.register_multiple(create_default_sub_agents());

    SharedComponents {
        tool_registry: Arc::new(RwLock::new(
            ToolRegistry::new(workspace.to_path_buf()).await,
        )),
        sub_agent_registry: Arc::new(RwLock::new(sub_agent_registry)),
        approval_recorder: Arc::new(
            ApprovalRecorder::new(workspace.join(".qbit").join("hitl")).await,
        ),
        tool_policy_manager: Arc::new(ToolPolicyManager::new(workspace).await),
        context_manager: Arc::new(ContextManager::for_model(model)),
        loop_detector: Arc::new(RwLock::new(LoopDetector::with_defaults())),
    }
}

/// Create components for an OpenRouter-based client.
pub async fn create_openrouter_components(
    config: OpenRouterClientConfig<'_>,
) -> Result<AgentBridgeComponents> {
    let openrouter_client = rig_openrouter::Client::new(config.api_key);
    let completion_model = openrouter_client.completion_model(config.model);
    let client = LlmClient::RigOpenRouter(completion_model);

    let shared = create_shared_components(&config.workspace, config.model).await;

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
pub async fn create_vertex_components(
    config: VertexAnthropicClientConfig<'_>,
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

    let shared = create_shared_components(&config.workspace, config.model).await;

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
pub async fn create_openai_components(
    config: OpenAiClientConfig<'_>,
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
    let completion_model = openai_client.completion_model(config.model).completions_api();
    let client = LlmClient::RigOpenAi(completion_model);

    let shared = create_shared_components(&config.workspace, config.model).await;

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
