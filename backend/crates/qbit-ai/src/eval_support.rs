//! Evaluation support for running the unified agentic loop in test/eval contexts.
//!
//! This module provides a simplified entry point for evaluations to use the same
//! agentic loop as the main application, ensuring evals test actual behavior.
//!
//! The key function is `run_eval_agentic_loop` which:
//! 1. Sets up minimal mock dependencies (auto-approve mode, no HITL)
//! 2. Runs the same `run_agentic_loop_unified` as the main agent
//! 3. Captures events from the channel
//! 4. Extracts tool calls and file modifications from events
//! 5. Returns structured output for eval assertions

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use rig::completion::{CompletionModel as RigCompletionModel, Message};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, RwLock};

use qbit_context::{CompactionState, ContextManager, ContextManagerConfig};
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

/// A tool call captured during eval execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalToolCall {
    /// Name of the tool that was called
    pub name: String,
    /// Input arguments to the tool
    pub input: serde_json::Value,
    /// Output from the tool (if available)
    pub output: Option<String>,
    /// Whether the tool execution was successful
    pub success: bool,
}

/// Output from an eval agentic loop run.
#[derive(Debug, Clone)]
pub struct EvalAgentOutput {
    /// Final text response from the agent.
    pub response: String,
    /// All tool calls made during execution.
    pub tool_calls: Vec<EvalToolCall>,
    /// Files that were modified during execution.
    pub files_modified: Vec<PathBuf>,
    /// Duration of execution in milliseconds.
    pub duration_ms: u64,
    /// Token usage (total tokens used).
    pub tokens_used: Option<u32>,
    /// Message history from the conversation.
    pub history: Vec<Message>,
    /// Raw events emitted during execution (for debugging).
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
/// * `EvalAgentOutput` containing response, tool calls, files modified, etc.
pub async fn run_eval_agentic_loop<M>(
    model: &M,
    system_prompt: &str,
    user_prompt: &str,
    config: EvalConfig,
) -> Result<EvalAgentOutput>
where
    M: RigCompletionModel + Sync,
{
    let start = Instant::now();

    // Create event channel to capture events
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AiEvent>();

    // Create tool registry for the workspace
    let tool_registry = Arc::new(RwLock::new(
        ToolRegistry::new(config.workspace.clone()).await,
    ));

    // Create sub-agent registry with default sub-agents (coder, analyzer, explorer, researcher, executor)
    let mut registry = SubAgentRegistry::new();
    registry.register_multiple(qbit_sub_agents::create_default_sub_agents());
    let sub_agent_registry = Arc::new(RwLock::new(registry));

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

    // Create compaction state
    let compaction_state = Arc::new(RwLock::new(CompactionState::new()));

    // Create agent mode set to auto-approve
    let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));

    // Create plan manager
    let plan_manager = Arc::new(PlanManager::new());

    // Create workspace Arc
    let workspace_arc = Arc::new(RwLock::new(config.workspace.clone()));

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
        workspace: &workspace_arc,
        client: &llm_client,
        approval_recorder: &approval_recorder,
        pending_approvals: &pending_approvals,
        tool_policy_manager: &tool_policy_manager,
        context_manager: &context_manager,
        compaction_state: &compaction_state,
        loop_detector: &loop_detector,
        tool_config: &tool_config,
        sidecar_state: None,
        runtime: None,
        agent_mode: &agent_mode,
        plan_manager: &plan_manager,
        provider_name: &config.provider_name,
        model_name: &config.model_name,
        openai_web_search_config: None,
        model_factory: None,
        session_id: None,
        transcript_writer: None,
        transcript_base_dir: None,
        continuation_summary: None,
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

    let duration_ms = start.elapsed().as_millis() as u64;

    // Collect all events
    drop(event_tx); // Close sender so receiver can drain
    let mut events = Vec::new();
    while let Ok(event) = event_rx.try_recv() {
        events.push(event);
    }

    // Extract tool calls and file modifications from events
    let (tool_calls, files_modified) = extract_tool_calls_and_files(&events, &config.workspace);

    // Convert token usage (sum of input and output tokens)
    let tokens_used = tokens.map(|t| (t.input_tokens + t.output_tokens) as u32);

    Ok(EvalAgentOutput {
        response,
        tool_calls,
        files_modified,
        duration_ms,
        tokens_used,
        history,
        events,
    })
}

/// Extract tool calls and modified files from captured events.
///
/// This function processes the event stream to build:
/// 1. A list of all tool calls with their inputs and outputs
/// 2. A list of files that were modified by write operations
fn extract_tool_calls_and_files(
    events: &[AiEvent],
    workspace: &Path,
) -> (Vec<EvalToolCall>, Vec<PathBuf>) {
    // Map from request_id to tool args (captured from ToolAutoApproved)
    let mut args_by_request: HashMap<String, serde_json::Value> = HashMap::new();

    // First pass: collect args from ToolAutoApproved events
    for event in events {
        if let AiEvent::ToolAutoApproved {
            request_id, args, ..
        } = event
        {
            args_by_request.insert(request_id.clone(), args.clone());
        }
    }

    let mut tool_calls = Vec::new();
    let mut files_modified = Vec::new();

    // Second pass: build tool calls from ToolResult events
    for event in events {
        if let AiEvent::ToolResult {
            tool_name,
            result,
            success,
            request_id,
            ..
        } = event
        {
            // Get args from the corresponding ToolAutoApproved event
            let input = args_by_request
                .get(request_id)
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            tool_calls.push(EvalToolCall {
                name: tool_name.clone(),
                input: input.clone(),
                output: Some(serde_json::to_string(result).unwrap_or_default()),
                success: *success,
            });

            // Track files modified by write operations
            if *success && is_write_tool(tool_name) {
                if let Some(path) = extract_file_path(tool_name, &input) {
                    let full_path = workspace.join(&path);
                    if !files_modified.contains(&full_path) {
                        files_modified.push(full_path);
                    }
                }
            }
        }
    }

    (tool_calls, files_modified)
}

/// Check if a tool modifies files.
fn is_write_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write_file" | "create_file" | "edit_file" | "delete_file" | "ast_grep_replace"
    )
}

/// Extract file path from tool arguments.
fn extract_file_path(tool_name: &str, args: &serde_json::Value) -> Option<String> {
    match tool_name {
        "write_file" | "create_file" | "edit_file" | "delete_file" => args
            .get("path")
            .or_else(|| args.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Output from a multi-turn eval agentic loop run.
#[derive(Debug, Clone)]
pub struct MultiTurnEvalOutput {
    /// Outputs from each turn in order.
    pub turns: Vec<EvalAgentOutput>,
    /// Total duration of all turns in milliseconds.
    pub total_duration_ms: u64,
    /// Final message history after all turns.
    pub final_history: Vec<Message>,
}

/// Run a multi-turn evaluation to test conversation history handling.
///
/// This is critical for testing OpenAI Responses API reasoning item preservation,
/// as the bug only manifests across multiple turns where reasoning IDs must be
/// preserved in history.
///
/// # Arguments
/// * `model` - The completion model to use
/// * `system_prompt` - System prompt for the agent
/// * `user_prompts` - Sequence of user prompts for each turn
/// * `config` - Eval configuration
///
/// # Returns
/// * `MultiTurnEvalOutput` containing outputs from each turn
pub async fn run_multi_turn_eval<M>(
    model: &M,
    system_prompt: &str,
    user_prompts: &[&str],
    config: EvalConfig,
) -> Result<MultiTurnEvalOutput>
where
    M: RigCompletionModel + Sync,
{
    let total_start = Instant::now();
    let mut turns = Vec::new();
    let mut current_history: Vec<Message> = Vec::new();

    // Create shared resources that persist across turns
    let tool_registry = Arc::new(RwLock::new(
        ToolRegistry::new(config.workspace.clone()).await,
    ));
    // Create sub-agent registry with default sub-agents
    let mut registry = SubAgentRegistry::new();
    registry.register_multiple(qbit_sub_agents::create_default_sub_agents());
    let sub_agent_registry = Arc::new(RwLock::new(registry));
    let temp_dir = std::env::temp_dir().join("qbit-eval-multiturn");
    std::fs::create_dir_all(&temp_dir).ok();
    let approval_recorder = Arc::new(ApprovalRecorder::new(temp_dir.clone()).await);
    let pending_approvals: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let tool_policy_manager = Arc::new(ToolPolicyManager::new(&config.workspace).await);
    let context_manager = Arc::new(ContextManager::with_config(
        &config.model_name,
        ContextManagerConfig {
            enabled: false,
            ..Default::default()
        },
    ));
    let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));
    let compaction_state = Arc::new(RwLock::new(CompactionState::new()));
    let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));
    let plan_manager = Arc::new(PlanManager::new());
    let workspace_arc = Arc::new(RwLock::new(config.workspace.clone()));
    let llm_client = Arc::new(RwLock::new(LlmClient::Mock));
    let tool_config = ToolConfig::default();
    let capabilities = ModelCapabilities::detect(&config.provider_name, &config.model_name);

    for (turn_idx, user_prompt) in user_prompts.iter().enumerate() {
        let turn_start = Instant::now();
        tracing::info!(
            "Starting multi-turn eval turn {}/{}",
            turn_idx + 1,
            user_prompts.len()
        );

        // Create event channel for this turn
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AiEvent>();

        let ctx = AgenticLoopContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            sub_agent_registry: &sub_agent_registry,
            indexer_state: None,
            workspace: &workspace_arc,
            client: &llm_client,
            approval_recorder: &approval_recorder,
            pending_approvals: &pending_approvals,
            tool_policy_manager: &tool_policy_manager,
            context_manager: &context_manager,
            compaction_state: &compaction_state,
            loop_detector: &loop_detector,
            tool_config: &tool_config,
            sidecar_state: None,
            runtime: None,
            agent_mode: &agent_mode,
            plan_manager: &plan_manager,
            provider_name: &config.provider_name,
            model_name: &config.model_name,
            openai_web_search_config: None,
            model_factory: None,
            session_id: None,
            transcript_writer: None,
            transcript_base_dir: None,
            continuation_summary: None,
        };

        let loop_config = AgenticLoopConfig {
            capabilities: capabilities.clone(),
            require_hitl: config.require_hitl,
            is_sub_agent: false,
        };

        // Add user message to current history
        current_history.push(Message::User {
            content: rig::one_or_many::OneOrMany::one(rig::message::UserContent::Text(
                rig::message::Text {
                    text: user_prompt.to_string(),
                },
            )),
        });

        let sub_agent_context = SubAgentContext {
            original_request: user_prompt.to_string(),
            ..Default::default()
        };

        // Run the unified loop with accumulated history
        let (response, new_history, tokens) = crate::agentic_loop::run_agentic_loop_unified(
            model,
            system_prompt,
            current_history.clone(),
            sub_agent_context,
            &ctx,
            loop_config,
        )
        .await?;

        // Update history with the new history from this turn
        current_history = new_history.clone();

        let turn_duration_ms = turn_start.elapsed().as_millis() as u64;

        // Collect events for this turn
        drop(event_tx);
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        let (tool_calls, files_modified) = extract_tool_calls_and_files(&events, &config.workspace);

        let tokens_used = tokens.map(|t| (t.input_tokens + t.output_tokens) as u32);

        turns.push(EvalAgentOutput {
            response,
            tool_calls,
            files_modified,
            duration_ms: turn_duration_ms,
            tokens_used,
            history: new_history,
            events,
        });

        tracing::info!(
            "Completed multi-turn eval turn {}/{} in {}ms",
            turn_idx + 1,
            user_prompts.len(),
            turn_duration_ms
        );
    }

    let total_duration_ms = total_start.elapsed().as_millis() as u64;

    Ok(MultiTurnEvalOutput {
        turns,
        total_duration_ms,
        final_history: current_history,
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
