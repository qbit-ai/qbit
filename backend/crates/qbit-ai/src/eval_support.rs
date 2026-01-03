//! Evaluation support for running the unified agentic loop in test/eval contexts.
//!
//! This module provides a simplified entry point for evaluations to use the same
//! agentic loop as the main application, ensuring evals test actual behavior.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rig::completion::{CompletionModel as RigCompletionModel, Message};
use tokio::sync::{mpsc, oneshot, RwLock};

use qbit_context::{ContextManager, ContextManagerConfig};
use qbit_core::events::AiEvent;
use qbit_core::hitl::ApprovalDecision;
use qbit_hitl::ApprovalRecorder;
use qbit_llm_providers::{LlmClient, ModelCapabilities};
use qbit_loop_detection::LoopDetector;
use qbit_planner::PlanManager;
use qbit_sub_agents::{SubAgentContext, SubAgentRegistry};
use qbit_tool_policy::ToolPolicyManager;
use qbit_tools::ToolRegistry;

use crate::agent_mode::AgentMode;
use crate::agentic_loop::{AgenticLoopConfig, AgenticLoopContext};
use crate::tool_definitions::ToolConfig;

/// Output from an eval agentic loop run.
#[derive(Debug, Clone)]
pub struct EvalAgentOutput {
    /// Final text response from the agent.
    pub response: String,
    /// Message history from the conversation.
    pub history: Vec<Message>,
    /// Token usage statistics.
    pub tokens_used: Option<qbit_context::token_budget::TokenUsage>,
    /// Events emitted during execution.
    pub events: Vec<AiEvent>,
}

/// Configuration for eval execution.
#[derive(Debug, Clone)]
pub struct EvalConfig {
    /// Provider name for capability detection (e.g., "openai", "anthropic")
    pub provider_name: String,
    /// Model name for capability detection
    pub model_name: String,
    /// Whether to require HITL (always false for evals - auto-approve)
    pub require_hitl: bool,
    /// Workspace directory for tool execution
    pub workspace: PathBuf,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            provider_name: "anthropic".to_string(),
            model_name: "claude-3-sonnet".to_string(),
            require_hitl: false,
            workspace: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl EvalConfig {
    /// Create config for OpenAI provider.
    pub fn openai(model_name: &str, workspace: PathBuf) -> Self {
        Self {
            provider_name: "openai".to_string(),
            model_name: model_name.to_string(),
            require_hitl: false,
            workspace,
        }
    }

    /// Create config for Anthropic provider.
    pub fn anthropic(model_name: &str, workspace: PathBuf) -> Self {
        Self {
            provider_name: "anthropic".to_string(),
            model_name: model_name.to_string(),
            require_hitl: false,
            workspace,
        }
    }

    /// Create config for Vertex AI provider.
    pub fn vertex_ai(model_name: &str, workspace: PathBuf) -> Self {
        Self {
            provider_name: "vertex_ai".to_string(),
            model_name: model_name.to_string(),
            require_hitl: false,
            workspace,
        }
    }
}

/// Run the unified agentic loop for evaluation purposes.
///
/// This function sets up minimal mock dependencies and runs the same agentic loop
/// used by the main application, ensuring evaluations test real behavior.
///
/// # Arguments
/// * `model` - The completion model to use
/// * `system_prompt` - System prompt for the agent
/// * `user_prompt` - Initial user prompt
/// * `config` - Eval configuration
///
/// # Returns
/// * `EvalAgentOutput` containing response, history, tokens, and events
pub async fn run_eval_agentic_loop<M>(
    model: &M,
    system_prompt: &str,
    user_prompt: &str,
    config: EvalConfig,
) -> Result<EvalAgentOutput>
where
    M: RigCompletionModel + Sync,
{
    // Create event channel to capture events
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AiEvent>();

    // Create tool registry for the workspace
    let tool_registry = Arc::new(RwLock::new(
        ToolRegistry::new(config.workspace.clone()).await,
    ));

    // Create empty sub-agent registry (no sub-agents in evals)
    let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));

    // Create approval recorder (uses temp dir for storage)
    let temp_dir = std::env::temp_dir().join("qbit-eval");
    std::fs::create_dir_all(&temp_dir).ok();
    let approval_recorder = Arc::new(ApprovalRecorder::new(temp_dir.clone()).await);

    // Create empty pending approvals
    let pending_approvals: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Create permissive tool policy manager
    let tool_policy_manager = Arc::new(ToolPolicyManager::new(&config.workspace).await);

    // Create context manager with high limits (no pruning in evals)
    let context_manager = Arc::new(ContextManager::with_config(
        &config.model_name,
        ContextManagerConfig {
            enabled: false, // Disable pruning for evals
            ..Default::default()
        },
    ));

    // Create loop detector with default config
    let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));

    // Create agent mode set to auto-approve
    let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));

    // Create plan manager
    let plan_manager = Arc::new(PlanManager::new());

    // Create workspace Arc
    let workspace = Arc::new(RwLock::new(config.workspace.clone()));

    // Create a mock LLM client (used only to check supports_native_web_tools)
    let llm_client = Arc::new(RwLock::new(LlmClient::Mock));

    // Tool config - enable all tools
    let tool_config = ToolConfig::default();

    // Build the context
    let ctx = AgenticLoopContext {
        event_tx: &event_tx,
        tool_registry: &tool_registry,
        sub_agent_registry: &sub_agent_registry,
        indexer_state: None,
        tavily_state: None,
        workspace: &workspace,
        client: &llm_client,
        approval_recorder: &approval_recorder,
        pending_approvals: &pending_approvals,
        tool_policy_manager: &tool_policy_manager,
        context_manager: &context_manager,
        loop_detector: &loop_detector,
        tool_config: &tool_config,
        sidecar_state: None,
        runtime: None,
        agent_mode: &agent_mode,
        plan_manager: &plan_manager,
        provider_name: &config.provider_name,
        model_name: &config.model_name,
        openai_web_search_config: None,
    };

    // Detect capabilities from provider/model
    let capabilities = ModelCapabilities::detect(&config.provider_name, &config.model_name);

    let loop_config = AgenticLoopConfig {
        capabilities,
        require_hitl: config.require_hitl,
        is_sub_agent: false,
    };

    // Create initial history with user prompt
    let initial_history = vec![Message::User {
        content: rig::one_or_many::OneOrMany::one(rig::message::UserContent::Text(
            rig::message::Text {
                text: user_prompt.to_string(),
            },
        )),
    }];

    // Create sub-agent context
    let sub_agent_context = SubAgentContext {
        original_request: user_prompt.to_string(),
        ..Default::default()
    };

    // Run the unified loop
    let (response, history, tokens) = crate::agentic_loop::run_agentic_loop_unified(
        model,
        system_prompt,
        initial_history,
        sub_agent_context,
        &ctx,
        loop_config,
    )
    .await?;

    // Collect all events
    drop(event_tx); // Close sender so receiver can drain
    let mut events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        events.push(event);
    }

    Ok(EvalAgentOutput {
        response,
        history,
        tokens_used: tokens,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_config_default() {
        let config = EvalConfig::default();
        assert_eq!(config.provider_name, "anthropic");
        assert!(!config.require_hitl);
    }

    #[test]
    fn test_eval_config_openai() {
        let config = EvalConfig::openai("gpt-5.1", PathBuf::from("/tmp"));
        assert_eq!(config.provider_name, "openai");
        assert_eq!(config.model_name, "gpt-5.1");
    }
}
