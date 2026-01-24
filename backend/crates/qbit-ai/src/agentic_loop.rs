//! Agentic tool loop for LLM execution.
//!
//! This module contains the main agentic loop that handles:
//! - Tool execution with HITL approval
//! - Loop detection and prevention
//! - Context window management
//! - Message history management
//! - Extended thinking (streaming reasoning content)

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use rig::completion::{
    AssistantContent, CompletionModel as RigCompletionModel, GetTokenUsage, Message,
};
use rig::message::{Reasoning, Text, ToolCall, ToolResult, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;
use rig::streaming::StreamedAssistantContent;
use serde_json::json;
use tokio::sync::{mpsc, oneshot, RwLock};
use tracing::Instrument;

use qbit_tools::ToolRegistry;

use super::system_hooks::{format_system_hooks, HookRegistry, PostToolContext};
use super::tool_definitions::{
    get_all_tool_definitions_with_config, get_run_command_tool_definition,
    get_sub_agent_tool_definitions, ToolConfig,
};
use super::tool_executors::{
    execute_indexer_tool, execute_plan_tool, execute_web_fetch_tool, normalize_run_pty_cmd_args,
};
use super::tool_provider_impl::DefaultToolProvider;
use qbit_context::token_budget::TokenUsage;
use qbit_context::{CompactionState, ContextManager};
use qbit_core::events::AiEvent;
use qbit_core::hitl::{ApprovalDecision, RiskLevel};
use qbit_core::runtime::QbitRuntime;
use qbit_core::utils::truncate_str;
use qbit_hitl::ApprovalRecorder;
use qbit_indexer::IndexerState;
use qbit_llm_providers::ModelCapabilities;
use qbit_loop_detection::{LoopDetectionResult, LoopDetector};
use qbit_sidecar::{CaptureContext, SidecarState};
use qbit_sub_agents::{
    execute_sub_agent, SubAgentContext, SubAgentExecutorContext, SubAgentRegistry, MAX_AGENT_DEPTH,
};
use qbit_tool_policy::{PolicyConstraintResult, ToolPolicy, ToolPolicyManager};

use crate::event_coordinator::CoordinatorHandle;

/// Maximum number of tool call iterations before stopping
pub const MAX_TOOL_ITERATIONS: usize = 100;

// =============================================================================
// Sub-agent model dispatch helper
// =============================================================================

/// Execute a sub-agent with an LlmClient by dispatching to the correct model type.
///
/// This function matches on the LlmClient variant and calls execute_sub_agent
/// with the appropriate inner model type.
async fn execute_sub_agent_with_client(
    agent_def: &qbit_sub_agents::SubAgentDefinition,
    args: &serde_json::Value,
    context: &SubAgentContext,
    client: &qbit_llm_providers::LlmClient,
    ctx: SubAgentExecutorContext<'_>,
    tool_provider: &DefaultToolProvider,
    parent_request_id: &str,
) -> anyhow::Result<qbit_sub_agents::SubAgentResult> {
    use qbit_llm_providers::LlmClient;

    match client {
        LlmClient::VertexAnthropic(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigOpenRouter(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigOpenAi(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigOpenAiResponses(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::OpenAiReasoning(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigAnthropic(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigOllama(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigGemini(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigGroq(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigXai(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::RigZaiSdk(model) => {
            execute_sub_agent(
                agent_def,
                args,
                context,
                model,
                ctx,
                tool_provider,
                parent_request_id,
            )
            .await
        }
        LlmClient::Mock => Err(anyhow::anyhow!("Cannot execute sub-agent with Mock client")),
    }
}

/// Timeout for approval requests in seconds (5 minutes)
pub const APPROVAL_TIMEOUT_SECS: u64 = 300;

/// Maximum tokens for a single completion request
pub const MAX_COMPLETION_TOKENS: u32 = 10_000;

/// Context for the agentic loop execution.
pub struct AgenticLoopContext<'a> {
    pub event_tx: &'a mpsc::UnboundedSender<AiEvent>,
    pub tool_registry: &'a Arc<RwLock<ToolRegistry>>,
    pub sub_agent_registry: &'a Arc<RwLock<SubAgentRegistry>>,
    pub indexer_state: Option<&'a Arc<IndexerState>>,
    #[cfg_attr(not(feature = "tauri"), allow(dead_code))]
    pub workspace: &'a Arc<RwLock<std::path::PathBuf>>,
    #[cfg_attr(not(feature = "tauri"), allow(dead_code))]
    pub client: &'a Arc<RwLock<qbit_llm_providers::LlmClient>>,
    pub approval_recorder: &'a Arc<ApprovalRecorder>,
    pub pending_approvals: &'a Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>>,
    pub tool_policy_manager: &'a Arc<ToolPolicyManager>,
    pub context_manager: &'a Arc<ContextManager>,
    pub loop_detector: &'a Arc<RwLock<LoopDetector>>,
    /// Compaction state for tracking token usage and triggering context compaction
    pub compaction_state: &'a Arc<RwLock<CompactionState>>,
    /// Tool configuration for filtering available tools
    pub tool_config: &'a ToolConfig,
    /// Sidecar state for context capture (optional)
    pub sidecar_state: Option<&'a Arc<SidecarState>>,
    /// Runtime for auto-approve checks (optional for backward compatibility)
    pub runtime: Option<&'a Arc<dyn QbitRuntime>>,
    /// Agent mode for controlling tool approval behavior
    pub agent_mode: &'a Arc<RwLock<super::agent_mode::AgentMode>>,
    /// Plan manager for update_plan tool
    pub plan_manager: &'a Arc<qbit_planner::PlanManager>,
    /// Provider name for capability detection (e.g., "openai", "anthropic")
    pub provider_name: &'a str,
    /// Model name for capability detection
    pub model_name: &'a str,
    /// OpenAI web search config (if enabled)
    pub openai_web_search_config: Option<&'a qbit_llm_providers::OpenAiWebSearchConfig>,
    /// OpenAI reasoning effort level (if set)
    pub openai_reasoning_effort: Option<&'a str>,
    /// Factory for creating sub-agent model override clients (optional)
    pub model_factory: Option<&'a Arc<super::llm_client::LlmClientFactory>>,
    /// Session ID for Langfuse trace grouping (optional)
    pub session_id: Option<&'a str>,
    /// Transcript writer for persisting AI events (optional)
    pub transcript_writer: Option<&'a Arc<crate::transcript::TranscriptWriter>>,
    /// Base directory for transcript files (e.g., `~/.qbit/transcripts`)
    /// Used to create separate transcript files for sub-agent internal events.
    pub transcript_base_dir: Option<&'a std::path::Path>,
    /// Additional tool definitions to include (e.g., SWE-bench test tool).
    /// These are added to the tool list alongside the standard tools.
    pub additional_tool_definitions: Vec<rig::completion::ToolDefinition>,
    /// Custom tool executor for handling additional tools.
    /// If provided, this function is called for tools not handled by the standard executors.
    /// Returns `Some((result, success))` if the tool was handled, `None` otherwise.
    #[allow(clippy::type_complexity)]
    pub custom_tool_executor: Option<
        std::sync::Arc<
            dyn Fn(
                    &str,
                    &serde_json::Value,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Option<(serde_json::Value, bool)>> + Send>,
                > + Send
                + Sync,
        >,
    >,
    /// Event coordinator for message-passing based event management (optional).
    /// When available, approval registration uses the coordinator instead of pending_approvals.
    pub coordinator: Option<&'a CoordinatorHandle>,
}

/// Result of a single tool execution.
pub struct ToolExecutionResult {
    pub value: serde_json::Value,
    pub success: bool,
}

/// Wrapper for capture context that persists across the loop
pub struct LoopCaptureContext {
    inner: Option<CaptureContext>,
}

impl LoopCaptureContext {
    /// Create a new loop capture context
    pub fn new(sidecar: Option<&Arc<SidecarState>>) -> Self {
        Self {
            inner: sidecar.map(|s| CaptureContext::new(s.clone())),
        }
    }

    /// Process an event if capture is enabled
    pub fn process(&mut self, event: &AiEvent) {
        if let Some(ref mut capture) = self.inner {
            capture.process(event);
        }
    }
}

/// Returns true if this event should be written to the transcript.
/// Filters out streaming events (TextDelta, Reasoning) since their content
/// is captured in aggregate events (Completed contains full response).
fn should_transcript(event: &AiEvent) -> bool {
    // Skip streaming events and sub-agent internal events (they go to separate transcript files)
    !matches!(
        event,
        AiEvent::TextDelta { .. }
            | AiEvent::Reasoning { .. }
            | AiEvent::SubAgentToolRequest { .. }
            | AiEvent::SubAgentToolResult { .. }
    )
}

/// Helper to emit an event to frontend and transcript (but not sidecar)
/// Use this when sidecar capture is handled separately (e.g., with stateful capture_ctx)
fn emit_to_frontend(ctx: &AgenticLoopContext<'_>, event: AiEvent) {
    // Write to transcript if configured (skip streaming events)
    if let Some(writer) = ctx.transcript_writer {
        if should_transcript(&event) {
            let writer = Arc::clone(writer);
            let event_clone = event.clone();
            tokio::spawn(async move {
                if let Err(e) = writer.append(&event_clone).await {
                    tracing::warn!("Failed to write to transcript: {}", e);
                }
            });
        }
    }

    let _ = ctx.event_tx.send(event);
}

/// Helper to emit an event to both frontend and sidecar (stateless capture)
/// Use this for events that don't need state correlation (e.g., Reasoning)
fn emit_event(ctx: &AgenticLoopContext<'_>, event: AiEvent) {
    // Log reasoning events being emitted to frontend (trace level to reduce spam)
    if let AiEvent::Reasoning { ref content } = event {
        tracing::trace!(
            "[Thinking] Emitting reasoning event to frontend: {} chars",
            content.len()
        );
    }

    // Write to transcript if configured (skip streaming events)
    if let Some(writer) = ctx.transcript_writer {
        if should_transcript(&event) {
            let writer = Arc::clone(writer);
            let event_clone = event.clone();
            tokio::spawn(async move {
                if let Err(e) = writer.append(&event_clone).await {
                    tracing::warn!("Failed to write to transcript: {}", e);
                }
            });
        }
    }

    // Send to frontend
    let _ = ctx.event_tx.send(event.clone());

    // Capture in sidecar if available (stateless - creates fresh context each time)
    if let Some(sidecar) = ctx.sidecar_state {
        let mut capture = CaptureContext::new(sidecar.clone());
        capture.process(&event);
    }
}

/// Estimate the character count of a message for heuristic token estimation.
///
/// This is used as a fallback when the provider doesn't return token usage.
/// Uses the approximation of ~4 characters per token.
fn estimate_message_chars(message: &Message) -> usize {
    match message {
        Message::User { content } => content
            .iter()
            .map(|c| match c {
                UserContent::Text(text) => text.text.len(),
                UserContent::ToolResult(result) => {
                    // Estimate tool result size
                    result.id.len()
                        + result
                            .content
                            .iter()
                            .map(|r| match r {
                                ToolResultContent::Text(t) => t.text.len(),
                                ToolResultContent::Image(_) => 1000, // Rough estimate for images
                            })
                            .sum::<usize>()
                }
                UserContent::Image(_) => 1000, // Rough estimate for images
                UserContent::Audio(_) => 5000, // Rough estimate for audio
                UserContent::Video(_) => 10000, // Rough estimate for video
                UserContent::Document(_) => 5000, // Rough estimate for documents
            })
            .sum(),
        Message::Assistant { content, .. } => content
            .iter()
            .map(|c| match c {
                AssistantContent::Text(text) => text.text.len(),
                AssistantContent::ToolCall(call) => {
                    call.function.name.len()
                        + serde_json::to_string(&call.function.arguments)
                            .map(|s| s.len())
                            .unwrap_or(0)
                }
                AssistantContent::Reasoning(reasoning) => {
                    reasoning.reasoning.iter().map(|r| r.len()).sum::<usize>()
                }
                AssistantContent::Image(_) => 1000, // Rough estimate for images
            })
            .sum(),
    }
}

/// Handle loop detection result and create appropriate tool result if blocked.
///
/// `tool_id` is the main identifier (used for events/UI).
/// `tool_call_id` is used for the tool result's call_id (OpenAI uses call_* format).
pub fn handle_loop_detection(
    loop_result: &LoopDetectionResult,
    tool_id: &str,
    tool_call_id: &str,
    event_tx: &mpsc::UnboundedSender<AiEvent>,
) -> Option<UserContent> {
    match loop_result {
        LoopDetectionResult::Blocked {
            tool_name,
            repeat_count,
            max_count,
            message,
        } => {
            let _ = event_tx.send(AiEvent::LoopBlocked {
                tool_name: tool_name.clone(),
                repeat_count: *repeat_count,
                max_count: *max_count,
                message: message.clone(),
            });
            let result_text = serde_json::to_string(&json!({
                "error": message,
                "loop_detected": true,
                "repeat_count": repeat_count,
                "suggestion": "Try a different approach or modify the arguments"
            }))
            .unwrap_or_default();
            Some(UserContent::ToolResult(ToolResult {
                id: tool_id.to_string(),
                call_id: Some(tool_call_id.to_string()),
                content: OneOrMany::one(ToolResultContent::Text(Text { text: result_text })),
            }))
        }
        LoopDetectionResult::MaxIterationsReached {
            iterations,
            max_iterations,
            message,
        } => {
            let _ = event_tx.send(AiEvent::MaxIterationsReached {
                iterations: *iterations,
                max_iterations: *max_iterations,
                message: message.clone(),
            });
            let result_text = serde_json::to_string(&json!({
                "error": message,
                "max_iterations_reached": true,
                "suggestion": "Provide a final response to the user"
            }))
            .unwrap_or_default();
            Some(UserContent::ToolResult(ToolResult {
                id: tool_id.to_string(),
                call_id: Some(tool_call_id.to_string()),
                content: OneOrMany::one(ToolResultContent::Text(Text { text: result_text })),
            }))
        }
        LoopDetectionResult::Warning {
            tool_name,
            current_count,
            max_count,
            message,
        } => {
            let _ = event_tx.send(AiEvent::LoopWarning {
                tool_name: tool_name.clone(),
                current_count: *current_count,
                max_count: *max_count,
                message: message.clone(),
            });
            None // Warning doesn't block execution
        }
        LoopDetectionResult::Allowed => None,
    }
}

/// Execute the main agentic loop with tool calling.
///
/// This function runs the LLM completion loop, handling:
/// - Tool calls and results
/// - Loop detection
/// - Context window management
/// - HITL approval
/// - Extended thinking (streaming reasoning content)
///
/// Returns a tuple of (response_text, message_history, token_usage)
///
/// Note: This is the Anthropic-specific entry point that delegates to the unified loop
/// with thinking history support enabled.
///
/// Returns: (response, reasoning, history, token_usage)
pub async fn run_agentic_loop(
    model: &rig_anthropic_vertex::CompletionModel,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
) -> Result<(String, Option<String>, Vec<Message>, Option<TokenUsage>)> {
    // Delegate to unified loop with Anthropic configuration (thinking history enabled)
    run_agentic_loop_unified(
        model,
        system_prompt,
        initial_history,
        context,
        ctx,
        AgenticLoopConfig::main_agent_anthropic(),
    )
    .await
}

/// Execute a tool directly for generic models (after approval or auto-approved).
pub async fn execute_tool_direct_generic<M>(
    tool_name: &str,
    tool_args: &serde_json::Value,
    ctx: &AgenticLoopContext<'_>,
    model: &M,
    context: &SubAgentContext,
    tool_id: &str,
) -> Result<ToolExecutionResult>
where
    M: RigCompletionModel + Sync,
{
    // Check if this is an indexer tool call
    if tool_name.starts_with("indexer_") {
        let (value, success) = execute_indexer_tool(ctx.indexer_state, tool_name, tool_args).await;
        return Ok(ToolExecutionResult { value, success });
    }

    // Check if this is our custom web_fetch tool (with readability extraction)
    if tool_name == "web_fetch" {
        let (value, success) = execute_web_fetch_tool(tool_name, tool_args).await;
        return Ok(ToolExecutionResult { value, success });
    }

    // Check if this is an update_plan tool call
    if tool_name == "update_plan" {
        let (value, success) = execute_plan_tool(ctx.plan_manager, ctx.event_tx, tool_args).await;
        return Ok(ToolExecutionResult { value, success });
    }

    // Check if this is handled by a custom tool executor (e.g., SWE-bench test tool)
    if let Some(ref executor) = ctx.custom_tool_executor {
        if let Some((value, success)) = executor(tool_name, tool_args).await {
            return Ok(ToolExecutionResult { value, success });
        }
    }

    // Check if this is a sub-agent call
    if tool_name.starts_with("sub_agent_") {
        let agent_id = tool_name.strip_prefix("sub_agent_").unwrap_or("");

        // Get the agent definition
        let registry = ctx.sub_agent_registry.read().await;
        let agent_def = match registry.get(agent_id) {
            Some(def) => def.clone(),
            None => {
                return Ok(ToolExecutionResult {
                    value: json!({ "error": format!("Sub-agent '{}' not found", agent_id) }),
                    success: false,
                });
            }
        };
        drop(registry);

        let tool_provider = DefaultToolProvider::new();

        // Check if this sub-agent has a model override
        let result = if let Some((override_provider, override_model)) = &agent_def.model_override {
            // Try to get/create the override model client
            let override_client = if let Some(factory) = ctx.model_factory {
                match factory
                    .get_or_create(override_provider, override_model)
                    .await
                {
                    Ok(client) => Some(client),
                    Err(e) => {
                        tracing::warn!(
                            "Failed to create override model {}/{} for sub-agent '{}': {}. Using main model.",
                            override_provider, override_model, agent_id, e
                        );
                        None
                    }
                }
            } else {
                tracing::warn!(
                    "Sub-agent '{}' has model override but no factory available. Using main model.",
                    agent_id
                );
                None
            };

            if let Some(client) = override_client {
                // Execute with override model - dispatch based on LlmClient variant
                tracing::info!(
                    "[sub-agent:{}] Executing with override model: provider={}, model={}",
                    agent_id,
                    override_provider,
                    override_model
                );
                let sub_ctx = SubAgentExecutorContext {
                    event_tx: ctx.event_tx,
                    tool_registry: ctx.tool_registry,
                    workspace: ctx.workspace,
                    provider_name: override_provider,
                    model_name: override_model,
                    session_id: ctx.session_id,
                    transcript_base_dir: ctx.transcript_base_dir,
                };
                execute_sub_agent_with_client(
                    &agent_def,
                    tool_args,
                    context,
                    &client,
                    sub_ctx,
                    &tool_provider,
                    tool_id,
                )
                .await
            } else {
                // Fallback to main model
                tracing::info!(
                    "[sub-agent:{}] Executing with main model (override failed): provider={}, model={}",
                    agent_id,
                    ctx.provider_name,
                    ctx.model_name
                );
                let sub_ctx = SubAgentExecutorContext {
                    event_tx: ctx.event_tx,
                    tool_registry: ctx.tool_registry,
                    workspace: ctx.workspace,
                    provider_name: ctx.provider_name,
                    model_name: ctx.model_name,
                    session_id: ctx.session_id,
                    transcript_base_dir: ctx.transcript_base_dir,
                };
                execute_sub_agent(
                    &agent_def,
                    tool_args,
                    context,
                    model,
                    sub_ctx,
                    &tool_provider,
                    tool_id,
                )
                .await
            }
        } else {
            // No override - use main model (current behavior)
            tracing::info!(
                "[sub-agent:{}] Executing with main model (no override): provider={}, model={}",
                agent_id,
                ctx.provider_name,
                ctx.model_name
            );
            let sub_ctx = SubAgentExecutorContext {
                event_tx: ctx.event_tx,
                tool_registry: ctx.tool_registry,
                workspace: ctx.workspace,
                provider_name: ctx.provider_name,
                model_name: ctx.model_name,
                session_id: ctx.session_id,
                transcript_base_dir: ctx.transcript_base_dir,
            };
            execute_sub_agent(
                &agent_def,
                tool_args,
                context,
                model,
                sub_ctx,
                &tool_provider,
                tool_id,
            )
            .await
        };

        match result {
            Ok(result) => {
                return Ok(ToolExecutionResult {
                    value: json!({
                        "agent_id": result.agent_id,
                        "response": result.response,
                        "success": result.success,
                        "duration_ms": result.duration_ms,
                        "files_modified": result.files_modified
                    }),
                    success: result.success,
                });
            }
            Err(e) => {
                return Ok(ToolExecutionResult {
                    value: json!({ "error": e.to_string() }),
                    success: false,
                });
            }
        }
    }

    // Map run_command to run_pty_cmd (run_command is a user-friendly alias)
    let effective_tool_name = if tool_name == "run_command" {
        "run_pty_cmd"
    } else {
        tool_name
    };

    // Execute regular tool via registry
    let mut registry = ctx.tool_registry.write().await;
    let result = registry
        .execute_tool(effective_tool_name, tool_args.clone())
        .await;

    match &result {
        Ok(v) => {
            // Check for failure: exit_code != 0 OR presence of "error" field
            let is_failure_by_exit_code = v
                .get("exit_code")
                .and_then(|ec| ec.as_i64())
                .map(|ec| ec != 0)
                .unwrap_or(false);
            let has_error_field = v.get("error").is_some();
            let is_success = !is_failure_by_exit_code && !has_error_field;
            Ok(ToolExecutionResult {
                value: v.clone(),
                success: is_success,
            })
        }
        Err(e) => Ok(ToolExecutionResult {
            value: json!({"error": e.to_string()}),
            success: false,
        }),
    }
}

/// Execute a tool with HITL approval check for generic models.
pub async fn execute_with_hitl_generic<M>(
    tool_name: &str,
    tool_args: &serde_json::Value,
    tool_id: &str,
    ctx: &AgenticLoopContext<'_>,
    capture_ctx: &mut LoopCaptureContext,
    model: &M,
    context: &SubAgentContext,
) -> Result<ToolExecutionResult>
where
    M: RigCompletionModel + Sync,
{
    // Capture tool request for file tracking
    capture_ctx.process(&AiEvent::ToolRequest {
        request_id: tool_id.to_string(),
        tool_name: tool_name.to_string(),
        args: tool_args.clone(),
        source: qbit_core::events::ToolSource::Main,
    });

    // Step 0: Check agent mode for special handling
    let agent_mode = *ctx.agent_mode.read().await;

    // Check if auto-approve is enabled (via agent mode or runtime flag)
    // This is used to bypass policy deny checks while still enforcing constraints
    let is_auto_approve =
        agent_mode.is_auto_approve() || ctx.runtime.is_some_and(|r| r.auto_approve());

    // Step 0.1: Planning mode restrictions (read-only tools only)
    if agent_mode.is_planning() {
        // In planning mode, only allow read-only tools
        // Check against the ALLOW_TOOLS list from tool_policy
        use qbit_tool_policy::ALLOW_TOOLS;
        if !ALLOW_TOOLS.contains(&tool_name) {
            let denied_event = AiEvent::ToolDenied {
                request_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                args: tool_args.clone(),
                reason: "Planning mode: only read-only tools are allowed".to_string(),
                source: qbit_core::events::ToolSource::Main,
            };
            emit_to_frontend(ctx, denied_event.clone());
            capture_ctx.process(&denied_event);
            return Ok(ToolExecutionResult {
                value: json!({
                    "error": format!("Tool '{}' is not allowed in planning mode (read-only)", tool_name),
                    "planning_mode_denied": true
                }),
                success: false,
            });
        }
    }

    // Step 1: Check if tool is denied by policy
    // Skip this check if auto-approve is enabled (policy is bypassed, but constraints still apply)
    if !is_auto_approve && ctx.tool_policy_manager.is_denied(tool_name).await {
        let denied_event = AiEvent::ToolDenied {
            request_id: tool_id.to_string(),
            tool_name: tool_name.to_string(),
            args: tool_args.clone(),
            reason: "Tool is denied by policy".to_string(),
            source: qbit_core::events::ToolSource::Main,
        };
        emit_to_frontend(ctx, denied_event.clone());
        capture_ctx.process(&denied_event);
        return Ok(ToolExecutionResult {
            value: json!({
                "error": format!("Tool '{}' is denied by policy", tool_name),
                "denied_by_policy": true
            }),
            success: false,
        });
    }

    // Step 2: Apply constraints and check for violations
    let (effective_args, constraint_note) = match ctx
        .tool_policy_manager
        .apply_constraints(tool_name, tool_args)
        .await
    {
        PolicyConstraintResult::Allowed => (tool_args.clone(), None),
        PolicyConstraintResult::Violated(reason) => {
            emit_event(
                ctx,
                AiEvent::ToolDenied {
                    request_id: tool_id.to_string(),
                    tool_name: tool_name.to_string(),
                    args: tool_args.clone(),
                    reason: reason.clone(),
                    source: qbit_core::events::ToolSource::Main,
                },
            );
            return Ok(ToolExecutionResult {
                value: json!({
                    "error": format!("Tool constraint violated: {}", reason),
                    "constraint_violated": true
                }),
                success: false,
            });
        }
        PolicyConstraintResult::Modified(modified_args, note) => {
            tracing::info!("Tool '{}' args modified by constraint: {}", tool_name, note);
            (modified_args, Some(note))
        }
    };

    // Step 3: Check if tool is allowed by policy (bypasses HITL)
    let policy = ctx.tool_policy_manager.get_policy(tool_name).await;
    if policy == ToolPolicy::Allow {
        let reason = if let Some(note) = constraint_note {
            format!("Allowed by policy ({})", note)
        } else {
            "Allowed by tool policy".to_string()
        };
        emit_event(
            ctx,
            AiEvent::ToolAutoApproved {
                request_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                args: effective_args.clone(),
                reason,
                source: qbit_core::events::ToolSource::Main,
            },
        );

        return execute_tool_direct_generic(
            tool_name,
            &effective_args,
            ctx,
            model,
            context,
            tool_id,
        )
        .await;
    }

    // Step 4: Check if tool should be auto-approved based on learned patterns
    if ctx.approval_recorder.should_auto_approve(tool_name).await {
        emit_event(
            ctx,
            AiEvent::ToolAutoApproved {
                request_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                args: effective_args.clone(),
                reason: "Auto-approved based on learned patterns or always-allow list".to_string(),
                source: qbit_core::events::ToolSource::Main,
            },
        );

        return execute_tool_direct_generic(
            tool_name,
            &effective_args,
            ctx,
            model,
            context,
            tool_id,
        )
        .await;
    }

    // Step 4.4: Auto-approve if agent mode or runtime flag is set
    // This happens AFTER constraints are checked (Step 2) to ensure safety limits apply
    if is_auto_approve {
        let reason = if agent_mode.is_auto_approve() {
            "Auto-approved via agent mode"
        } else {
            "Auto-approved via --auto-approve flag"
        };
        emit_event(
            ctx,
            AiEvent::ToolAutoApproved {
                request_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                args: effective_args.clone(),
                reason: reason.to_string(),
                source: qbit_core::events::ToolSource::Main,
            },
        );

        return execute_tool_direct_generic(
            tool_name,
            &effective_args,
            ctx,
            model,
            context,
            tool_id,
        )
        .await;
    }

    // Step 5: Need approval - create request with stats
    let stats = ctx.approval_recorder.get_pattern(tool_name).await;
    let risk_level = RiskLevel::for_tool(tool_name);
    let config = ctx.approval_recorder.get_config().await;
    let can_learn = !config
        .always_require_approval
        .contains(&tool_name.to_string());
    let suggestion = ctx.approval_recorder.get_suggestion(tool_name).await;

    // Register approval request - use coordinator if available, otherwise legacy path
    let rx = if let Some(coordinator) = ctx.coordinator {
        // New path: register via coordinator
        coordinator.register_approval(tool_id.to_string())
    } else {
        // Legacy path: create oneshot channel and store sender
        let (tx, rx) = oneshot::channel::<ApprovalDecision>();
        {
            let mut pending = ctx.pending_approvals.write().await;
            pending.insert(tool_id.to_string(), tx);
        }
        rx
    };

    // Emit approval request event with HITL metadata
    emit_to_frontend(
        ctx,
        AiEvent::ToolApprovalRequest {
            request_id: tool_id.to_string(),
            tool_name: tool_name.to_string(),
            args: effective_args.clone(),
            stats,
            risk_level,
            can_learn,
            suggestion,
            source: qbit_core::events::ToolSource::Main,
        },
    );

    // Wait for approval response (with timeout)
    match tokio::time::timeout(std::time::Duration::from_secs(APPROVAL_TIMEOUT_SECS), rx).await {
        Ok(Ok(decision)) => {
            if decision.approved {
                let _ = ctx
                    .approval_recorder
                    .record_approval(tool_name, true, decision.reason, decision.always_allow)
                    .await;

                execute_tool_direct_generic(
                    tool_name,
                    &effective_args,
                    ctx,
                    model,
                    context,
                    tool_id,
                )
                .await
            } else {
                let _ = ctx
                    .approval_recorder
                    .record_approval(tool_name, false, decision.reason, false)
                    .await;

                Ok(ToolExecutionResult {
                    value: json!({"error": "Tool execution denied by user", "denied": true}),
                    success: false,
                })
            }
        }
        Ok(Err(_)) => Ok(ToolExecutionResult {
            value: json!({"error": "Approval request cancelled", "cancelled": true}),
            success: false,
        }),
        Err(_) => {
            // Only need to clean up pending_approvals in legacy path
            // Coordinator handles cleanup automatically
            if ctx.coordinator.is_none() {
                let mut pending = ctx.pending_approvals.write().await;
                pending.remove(tool_id);
            }

            Ok(ToolExecutionResult {
                value: json!({"error": format!("Approval request timed out after {} seconds", APPROVAL_TIMEOUT_SECS), "timeout": true}),
                success: false,
            })
        }
    }
}

/// Generic agentic loop that works with any rig CompletionModel.
///
/// This is a simplified version of `run_agentic_loop` that:
/// - Works with any model implementing `rig::completion::CompletionModel`
/// - Does NOT support extended thinking (Anthropic-specific)
/// - Supports sub-agent calls (uses the same model for sub-agents)
///
/// Returns: (response, reasoning, history, token_usage)
///
/// Note: This is the generic entry point that delegates to the unified loop.
/// Model capabilities are detected from the provider/model name in the context.
pub async fn run_agentic_loop_generic<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
) -> Result<(String, Option<String>, Vec<Message>, Option<TokenUsage>)>
where
    M: RigCompletionModel + Sync,
{
    // Detect capabilities from provider/model name for proper temperature handling
    let config = AgenticLoopConfig::with_detection(ctx.provider_name, ctx.model_name, false);

    // Delegate to unified loop with detected configuration
    run_agentic_loop_unified(model, system_prompt, initial_history, context, ctx, config).await
}

// ============================================================================
// UNIFIED AGENTIC LOOP (Phase 1.3)
// ============================================================================

/// Configuration for the unified agentic loop.
///
/// This struct controls model-specific behavior in the unified loop,
/// allowing it to handle both Anthropic-style (thinking-enabled) and
/// generic model execution paths.
#[derive(Debug, Clone)]
pub struct AgenticLoopConfig {
    /// Model capabilities (thinking support, temperature, etc.)
    pub capabilities: ModelCapabilities,
    /// Whether HITL approval is required for tool execution.
    pub require_hitl: bool,
    /// Whether this is a sub-agent execution (affects tool restrictions).
    pub is_sub_agent: bool,
}

impl AgenticLoopConfig {
    /// Create config for main agent with Anthropic model.
    ///
    /// Anthropic models support extended thinking (reasoning history tracking)
    /// and require HITL approval for tool execution.
    pub fn main_agent_anthropic() -> Self {
        Self {
            capabilities: ModelCapabilities::anthropic_defaults(),
            require_hitl: true,
            is_sub_agent: false,
        }
    }

    /// Create config for main agent with generic model.
    ///
    /// Generic models use conservative defaults (no thinking history tracking)
    /// and require HITL approval for tool execution.
    pub fn main_agent_generic() -> Self {
        Self {
            capabilities: ModelCapabilities::conservative_defaults(),
            require_hitl: true,
            is_sub_agent: false,
        }
    }

    /// Create config for sub-agent (trusted, no HITL).
    ///
    /// Sub-agents are trusted and do not require HITL approval.
    /// The capabilities should match the model being used.
    pub fn sub_agent(capabilities: ModelCapabilities) -> Self {
        Self {
            capabilities,
            require_hitl: false,
            is_sub_agent: true,
        }
    }

    /// Create config with detected capabilities based on provider and model name.
    ///
    /// This factory method detects capabilities automatically and is useful
    /// when calling from code that has provider/model info but not an LlmClient.
    pub fn with_detection(provider_name: &str, model_name: &str, is_sub_agent: bool) -> Self {
        Self {
            capabilities: ModelCapabilities::detect(provider_name, model_name),
            require_hitl: !is_sub_agent,
            is_sub_agent,
        }
    }
}

/// Unified agentic loop that handles all model types.
///
/// This function replaces both `run_agentic_loop` (Anthropic) and
/// `run_agentic_loop_generic` by using configuration to control behavior.
///
/// # Key Differences from Separate Loops
///
/// 1. **Thinking History**: When `config.capabilities.supports_thinking_history` is true,
///    reasoning content from the model is preserved in the message history
///    (required by Anthropic API when extended thinking is enabled).
///
/// 2. **HITL Approval**: When `config.require_hitl` is true, tool execution
///    requires human-in-the-loop approval (unless auto-approved by policy).
///
/// 3. **Sub-Agent Restrictions**: When `config.is_sub_agent` is true,
///    certain tool restrictions may apply.
///
/// # Arguments
/// * `model` - The completion model to use
/// * `system_prompt` - System prompt for the agent
/// * `initial_history` - Starting conversation history
/// * `sub_agent_context` - Sub-agent execution context (includes depth tracking)
/// * `ctx` - Agent loop context with dependencies
/// * `config` - Configuration controlling behavior
///
/// # Returns
/// Tuple of (response_text, updated_history, token_usage)
///
/// # Example
/// ```ignore
/// use qbit_ai::agentic_loop::{run_agentic_loop_unified, AgenticLoopConfig};
///
/// // For Anthropic models (with thinking support)
/// let config = AgenticLoopConfig::main_agent_anthropic();
/// let (response, history, usage) = run_agentic_loop_unified(
///     &model, system_prompt, history, context, &ctx, config
/// ).await?;
///
/// // For generic models (without thinking support)
/// let config = AgenticLoopConfig::main_agent_generic();
/// let (response, history, usage) = run_agentic_loop_unified(
///     &model, system_prompt, history, context, &ctx, config
/// ).await?;
/// ```
pub async fn run_agentic_loop_unified<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    sub_agent_context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
    config: AgenticLoopConfig,
) -> Result<(String, Option<String>, Vec<Message>, Option<TokenUsage>)>
where
    M: rig::completion::CompletionModel + Sync,
{
    let supports_thinking = config.capabilities.supports_thinking_history;

    let agent_label = if config.is_sub_agent {
        format!("sub-agent (depth={})", sub_agent_context.depth)
    } else {
        "main-agent".to_string()
    };

    tracing::info!(
        "[{}] Starting agentic loop: provider={}, model={}, thinking={}, temperature={}",
        agent_label,
        ctx.provider_name,
        ctx.model_name,
        supports_thinking,
        config.capabilities.supports_temperature
    );

    // Create root span for the entire agent turn (this becomes the Langfuse trace)
    // All child spans (llm_completion, tool_call) will be nested under this
    // Extract user input from initial history for the trace input
    let trace_input: String = initial_history
        .iter()
        .rev()
        .find_map(|msg| {
            if let Message::User { content } = msg {
                Some(
                    content
                        .iter()
                        .filter_map(|c| {
                            if let rig::message::UserContent::Text(text) = c {
                                Some(text.text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();
    let trace_input_truncated = if trace_input.len() > 2000 {
        format!("{}... [truncated]", &trace_input[..2000])
    } else {
        trace_input
    };

    // Create outer trace span (this becomes the Langfuse trace)
    let chat_message_span = tracing::info_span!(
        "chat_message",
        "langfuse.session.id" = ctx.session_id.unwrap_or(""),
        "langfuse.observation.input" = %trace_input_truncated,
        "langfuse.observation.output" = tracing::field::Empty,
    );

    // Create agent span as child of trace (this is the main agent observation)
    let agent_span = tracing::info_span!(
        parent: &chat_message_span,
        "agent",
        "langfuse.observation.type" = "agent",
        "langfuse.session.id" = ctx.session_id.unwrap_or(""),
        "langfuse.observation.input" = %trace_input_truncated,
        "langfuse.observation.output" = tracing::field::Empty,
        agent_type = %agent_label,
        model = %ctx.model_name,
        provider = %ctx.provider_name,
    );
    // Instrument the main loop body with both spans so they're properly exported to OpenTelemetry.
    // Using nested .instrument() ensures both spans are entered for the duration of the loop.
    let (accumulated_response, accumulated_thinking, chat_history, total_usage) = async {
        // Reset loop detector for new turn
        {
        let mut detector = ctx.loop_detector.write().await;
        detector.reset();
    }

    // Create persistent capture context for file event correlation
    let mut capture_ctx = LoopCaptureContext::new(ctx.sidecar_state);

    // Create hook registry for system hooks
    let hook_registry = HookRegistry::new();

    // Get all available tools (filtered by config + web search)
    let mut tools = get_all_tool_definitions_with_config(ctx.tool_config);

    // Add run_command (wrapper for run_pty_cmd with better naming)
    tools.push(get_run_command_tool_definition());

    // Add any additional tools (e.g., SWE-bench test tool)
    tools.extend(ctx.additional_tool_definitions.iter().cloned());

    tracing::debug!(
        "Available tools (unified loop): {:?}",
        tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>()
    );

    // Always add Tavily web tools from the registry if enabled (alongside native tools)
    {
        let registry = ctx.tool_registry.read().await;
        let registry_tools = registry.get_tool_definitions();
        drop(registry);

        for tool in registry_tools {
            if (tool.name.starts_with("tavily_"))
                && ctx.tool_config.is_tool_enabled(&tool.name)
            {
                tools.push(tool);
            }
        }
    }

    // Only add sub-agent tools if we're not at max depth
    // Sub-agents are controlled by the registry, not the tool config
    if sub_agent_context.depth < MAX_AGENT_DEPTH - 1 {
        let registry = ctx.sub_agent_registry.read().await;
        tools.extend(get_sub_agent_tool_definitions(&registry).await);
    }

    let mut chat_history = initial_history;

    // Update context manager with current history
    ctx.context_manager
        .update_from_messages(&chat_history)
        .await;

    // Note: Context compaction is now handled by the summarizer agent
    // which is triggered via should_compact() in the agentic loop

    let mut accumulated_response = String::new();
    // Thinking history tracking - only used when supports_thinking is true
    let mut accumulated_thinking = String::new();
    let mut total_usage = TokenUsage::default();
    let mut iteration = 0;

    loop {
        iteration += 1;

        // Reset compaction state for this turn (preserves last_input_tokens)
        {
            let mut compaction_state = ctx.compaction_state.write().await;
            compaction_state.reset_turn();
        }

        // Check for compaction at start of turn (using tokens from previous turn)
        // This is important when the agent completes in a single iteration
        if iteration == 1 {
            {
                let compaction_state = ctx.compaction_state.read().await;
                if compaction_state.last_input_tokens.is_some() {
                    tracing::info!(
                        "[compaction] Pre-turn check - tokens: {:?}, using_heuristic: {}",
                        compaction_state.last_input_tokens,
                        compaction_state.using_heuristic
                    );
                }
            }

            if let Some(session_id) = ctx.session_id {
                match maybe_compact(ctx, session_id, &mut chat_history).await {
                    Ok(Some(result)) => {
                        if result.success {
                            let _ = ctx.event_tx.send(AiEvent::CompactionCompleted {
                                tokens_before: result.tokens_before,
                                messages_before: result.messages_before,
                                messages_after: chat_history.len(),
                                summary_length: result.summary.as_ref().map(|s| s.len()).unwrap_or(0),
                            });
                            ctx.context_manager
                                .update_from_messages(&chat_history)
                                .await;
                        } else {
                            let _ = ctx.event_tx.send(AiEvent::CompactionFailed {
                                tokens_before: result.tokens_before,
                                messages_before: result.messages_before,
                                error: result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                            });
                        }
                    }
                    Ok(None) => {} // No compaction needed
                    Err(e) => {
                        tracing::error!("[compaction] Pre-turn compaction error: {}", e);
                    }
                }
            }
        }

        if iteration > MAX_TOOL_ITERATIONS {
            // Record max iterations event in Langfuse
            let _max_iter_event = tracing::info_span!(
                parent: &agent_span,
                "max_iterations_reached",
                "langfuse.observation.type" = "event",
                "langfuse.session.id" = ctx.session_id.unwrap_or(""),
                max_iterations = MAX_TOOL_ITERATIONS,
            );

            let _ = ctx.event_tx.send(AiEvent::Error {
                message: "Maximum tool iterations reached".to_string(),
                error_type: "max_iterations".to_string(),
            });
            break;
        }

        // Check for context compaction need (between turns, after iteration 1)
        if iteration > 1 {
            // Log compaction state at start of each iteration
            {
                let compaction_state = ctx.compaction_state.read().await;
                tracing::info!(
                    "[compaction] Iteration {} - tokens: {:?}, using_heuristic: {}, attempted: {}",
                    iteration,
                    compaction_state.last_input_tokens,
                    compaction_state.using_heuristic,
                    compaction_state.attempted_this_turn
                );
            }

            if let Some(session_id) = ctx.session_id {
                // Check if compaction is needed and perform it if so
                match maybe_compact(ctx, session_id, &mut chat_history).await {
                    Ok(Some(result)) => {
                        if result.success {
                            // Emit success event
                            let _ = ctx.event_tx.send(AiEvent::CompactionCompleted {
                                tokens_before: result.tokens_before,
                                messages_before: result.messages_before,
                                messages_after: chat_history.len(),
                                summary_length: result.summary.as_ref().map(|s| s.len()).unwrap_or(0),
                            });

                            // Update context manager with new (compacted) history
                            ctx.context_manager
                                .update_from_messages(&chat_history)
                                .await;
                        } else {
                            // Emit failure event
                            let _ = ctx.event_tx.send(AiEvent::CompactionFailed {
                                tokens_before: result.tokens_before,
                                messages_before: result.messages_before,
                                error: result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                            });

                            // Check if we're still over the limit after failed compaction
                            let compaction_state = ctx.compaction_state.read().await;
                            let check = ctx
                                .context_manager
                                .should_compact(&compaction_state, ctx.model_name);
                            drop(compaction_state);

                            if check.should_compact {
                                // We needed compaction but it failed, and we're still over limit
                                tracing::error!(
                                    "[compaction] Failed and context still exceeded: {} tokens",
                                    check.current_tokens
                                );
                                let _ = ctx.event_tx.send(AiEvent::Error {
                                    message: format!(
                                        "Context compaction failed and limit exceeded ({} tokens). {}",
                                        check.current_tokens,
                                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                                    ),
                                    error_type: "compaction_failed".to_string(),
                                });
                                return Err(anyhow::anyhow!(
                                    "Context compaction failed and limit exceeded"
                                ));
                            }
                        }
                    }
                    Ok(None) => {
                        // No compaction needed, continue normally
                    }
                    Err(e) => {
                        // Error checking compaction (non-fatal, log and continue)
                        tracing::warn!("[compaction] Error during compaction check: {}", e);
                    }
                }
            }
        }

        // Create span for Langfuse observability (child of agent_span)
        // Token usage fields are Empty and will be recorded when available
        // Note: Langfuse expects prompt_tokens/completion_tokens per GenAI semantic conventions
        // Using both gen_ai.* and langfuse.observation.* for maximum compatibility
        let llm_span = tracing::info_span!(
            parent: &agent_span,
            "llm_completion",
            "gen_ai.operation.name" = "chat_completion",
            "gen_ai.request.model" = %ctx.model_name,
            "gen_ai.system" = %ctx.provider_name,
            "gen_ai.request.temperature" = 0.3_f64,
            "gen_ai.request.max_tokens" = MAX_COMPLETION_TOKENS as i64,
            "langfuse.observation.type" = "generation",
            "langfuse.session.id" = ctx.session_id.unwrap_or(""),
            iteration = iteration,
            "gen_ai.usage.prompt_tokens" = tracing::field::Empty,
            "gen_ai.usage.completion_tokens" = tracing::field::Empty,
            // Use both gen_ai.* and langfuse.observation.* for input/output mapping
            "gen_ai.prompt" = tracing::field::Empty,
            "gen_ai.completion" = tracing::field::Empty,
            "langfuse.observation.input" = tracing::field::Empty,
            "langfuse.observation.output" = tracing::field::Empty,
        );
        // Note: We use explicit parent instead of span.enter() for async compatibility

        // Extract user text for Langfuse prompt tracking
        // Only record actual user text - tool results are already in previous tool spans
        let last_user_text: String = chat_history
            .iter()
            .rev()
            .find_map(|msg| {
                if let Message::User { content } = msg {
                    let text_parts: Vec<String> = content
                        .iter()
                        .filter_map(|c| {
                            if let rig::message::UserContent::Text(text) = c {
                                if !text.text.is_empty() {
                                    Some(text.text.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    if !text_parts.is_empty() {
                        Some(text_parts.join("\n"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .unwrap_or_default();

        // Only record input if there's actual user text (not just tool results)
        if !last_user_text.is_empty() {
            let prompt_for_span = if last_user_text.len() > 2000 {
                format!("{}... [truncated]", &last_user_text[..2000])
            } else {
                last_user_text
            };
            llm_span.record("gen_ai.prompt", prompt_for_span.as_str());
            llm_span.record("langfuse.observation.input", prompt_for_span.as_str());
        }
        // When continuing after tool results: don't record input, context is in previous spans

        // Build request - conditionally set temperature based on model support
        let temperature = if config.capabilities.supports_temperature {
            Some(0.3)
        } else {
            tracing::debug!(
                "Model {} does not support temperature parameter, omitting",
                ctx.model_name
            );
            None
        };

        // Build additional_params for OpenAI-specific features (web search, reasoning effort)
        let mut additional_params_json = serde_json::Map::new();

        // Add web search if enabled
        if let Some(web_config) = ctx.openai_web_search_config {
            tracing::info!(
                "Adding OpenAI web_search_preview tool with context_size={}",
                web_config.search_context_size
            );
            additional_params_json.insert(
                "tools".to_string(),
                json!([web_config.to_tool_json()]),
            );
        }

        // Add reasoning config if set (for OpenAI o-series and GPT-5 Codex models)
        // OpenAI Responses API expects a nested "reasoning" object with:
        // - effort: how much thinking the model should do
        // - summary: enables streaming reasoning text to the client ("detailed" shows full reasoning)
        if let Some(effort) = ctx.openai_reasoning_effort {
            tracing::info!("Setting OpenAI reasoning.effort={}, reasoning.summary=detailed", effort);
            additional_params_json.insert(
                "reasoning".to_string(),
                json!({
                    "effort": effort,
                    "summary": "detailed"
                }),
            );
        }

        let additional_params = if additional_params_json.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(additional_params_json))
        };

        // Log if any messages contain images (debugging multimodal)
        let image_count: usize = chat_history
            .iter()
            .map(|msg| {
                if let Message::User { content } = msg {
                    content
                        .iter()
                        .filter(|c| matches!(c, rig::message::UserContent::Image(_)))
                        .count()
                } else {
                    0
                }
            })
            .sum();
        if image_count > 0 {
            tracing::info!(
                "[Unified] Chat history contains {} image(s) across {} messages",
                image_count,
                chat_history.len()
            );
        }

        let request = rig::completion::CompletionRequest {
            preamble: Some(system_prompt.to_string()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: tools.clone(),
            temperature,
            max_tokens: Some(MAX_COMPLETION_TOKENS as u64),
            tool_choice: None,
            additional_params,
        };

        // Make streaming completion request (instrumented for Langfuse)
        // Diagnostic logging for OpenAI multi-turn debugging
        let has_reasoning_in_history = chat_history.iter().any(|m| {
            if let Message::Assistant { content, .. } = m {
                content
                    .iter()
                    .any(|c| matches!(c, AssistantContent::Reasoning(_)))
            } else {
                false
            }
        });
        tracing::info!(
            "[OpenAI Debug] Starting stream: iteration={}, history_len={}, provider={}, has_reasoning_history={}",
            iteration,
            chat_history.len(),
            ctx.provider_name,
            has_reasoning_in_history
        );
        tracing::debug!(
            "[Unified] Starting streaming completion request (iteration {}, thinking={})",
            iteration,
            supports_thinking
        );

        // Wrap stream request in timeout to prevent infinite hangs (3 minutes)
        let stream_timeout = std::time::Duration::from_secs(180);
        let stream_result = tokio::time::timeout(
            stream_timeout,
            async { model.stream(request).await }.instrument(llm_span.clone()),
        )
        .await;

        let mut stream = match stream_result {
            Ok(Ok(s)) => {
                tracing::info!("[OpenAI Debug] Stream created successfully, consuming chunks...");
                s
            }
            Ok(Err(e)) => {
                let error_str = e.to_string();
                tracing::error!("Failed to start stream: {}", error_str);

                // Parse error to provide user-friendly message
                let (error_type, user_message) = if error_str.contains("Prompt is too long")
                    || error_str.contains("too many tokens")
                    || error_str.contains("context_length_exceeded")
                {
                    (
                        "context_overflow",
                        "The conversation is too long. Please start a new chat or clear some history.",
                    )
                } else if error_str.contains("rate_limit") || error_str.contains("429") {
                    (
                        "rate_limit",
                        "Rate limit exceeded. Please wait a moment and try again.",
                    )
                } else if error_str.contains("authentication")
                    || error_str.contains("401")
                    || error_str.contains("403")
                {
                    (
                        "authentication",
                        "Authentication failed. Please check your API credentials.",
                    )
                } else if error_str.contains("timeout") || error_str.contains("timed out") {
                    ("timeout", "Request timed out. Please try again.")
                } else {
                    ("api_error", error_str.as_str())
                };

                let _ = ctx.event_tx.send(AiEvent::Error {
                    message: user_message.to_string(),
                    error_type: error_type.to_string(),
                });

                return Err(anyhow::anyhow!("{}", error_str));
            }
            Err(_elapsed) => {
                // Timeout occurred - stream request took too long
                tracing::error!(
                    "[OpenAI Debug] Stream request timed out after {}s (iteration={}, provider={}, history_len={})",
                    stream_timeout.as_secs(),
                    iteration,
                    ctx.provider_name,
                    chat_history.len()
                );
                let _ = ctx.event_tx.send(AiEvent::Error {
                    message: format!(
                        "Request timed out after {} seconds. The AI provider is not responding. This may indicate a connection issue or an API problem.",
                        stream_timeout.as_secs()
                    ),
                    error_type: "timeout".to_string(),
                });
                return Err(anyhow::anyhow!(
                    "Stream request timeout after {}s",
                    stream_timeout.as_secs()
                ));
            }
        };
        tracing::debug!("[Unified] Stream started - listening for content");

        // Process streaming response
        let mut has_tool_calls = false;
        let mut tool_calls_to_execute: Vec<ToolCall> = vec![];
        let mut text_content = String::new();
        // Per-iteration thinking tracking (for history building)
        let mut thinking_content = String::new();
        let mut thinking_signature: Option<String> = None;
        // Reasoning ID for OpenAI Responses API (rs_... IDs that function calls reference)
        let mut thinking_id: Option<String> = None;
        let mut chunk_count = 0;

        // Track tool call state for streaming
        let mut current_tool_id: Option<String> = None;
        let mut current_tool_name: Option<String> = None;
        let mut current_tool_args = String::new();

        while let Some(chunk_result) = stream.next().await {
            chunk_count += 1;
            // Log progress every 50 chunks to avoid spam but track stream activity
            if chunk_count % 50 == 0 {
                tracing::debug!(
                    "[OpenAI Debug] Stream progress: {} chunks processed",
                    chunk_count
                );
            }
            match chunk_result {
                Ok(chunk) => {
                    match chunk {
                        StreamedAssistantContent::Text(text_msg) => {
                            // Check if this is thinking content (prefixed by our streaming impl)
                            // This handles the case where thinking is sent as a [Thinking] prefixed message
                            if let Some(thinking) = text_msg.text.strip_prefix("[Thinking] ") {
                                if supports_thinking {
                                    tracing::trace!(
                                        "[Unified] Received [Thinking]-prefixed text chunk #{}: {} chars",
                                        chunk_count,
                                        thinking.len()
                                    );
                                    thinking_content.push_str(thinking);
                                    accumulated_thinking.push_str(thinking);
                                }
                                // Always emit reasoning event (to frontend and sidecar)
                                emit_event(
                                    ctx,
                                    AiEvent::Reasoning {
                                        content: thinking.to_string(),
                                    },
                                );
                            } else {
                                // Check for server tool result markers
                                if let Some(rest) =
                                    text_msg.text.strip_prefix("[WEB_SEARCH_RESULT:")
                                {
                                    // Parse: [WEB_SEARCH_RESULT:tool_use_id:json_results]
                                    if let Some(colon_pos) = rest.find(':') {
                                        let tool_use_id = &rest[..colon_pos];
                                        let json_rest = rest[colon_pos + 1..].trim_end_matches(']');
                                        if let Ok(results) =
                                            serde_json::from_str::<serde_json::Value>(json_rest)
                                        {
                                            tracing::info!(
                                                "Parsed web search results for {}",
                                                tool_use_id
                                            );
                                            emit_event(
                                                ctx,
                                                AiEvent::WebSearchResult {
                                                    request_id: tool_use_id.to_string(),
                                                    results,
                                                },
                                            );
                                        }
                                    }
                                } else if let Some(rest) =
                                    text_msg.text.strip_prefix("[WEB_FETCH_RESULT:")
                                {
                                    // Parse: [WEB_FETCH_RESULT:tool_use_id:url:json_content]
                                    let parts: Vec<&str> = rest.splitn(3, ':').collect();
                                    if parts.len() >= 3 {
                                        let tool_use_id = parts[0];
                                        let url = parts[1];
                                        let json_rest = parts[2].trim_end_matches(']');
                                        let content_preview = if json_rest.len() > 200 {
                                            format!("{}...", &json_rest[..200])
                                        } else {
                                            json_rest.to_string()
                                        };
                                        tracing::info!(
                                            "Parsed web fetch result for {}: {}",
                                            tool_use_id,
                                            url
                                        );
                                        emit_event(
                                            ctx,
                                            AiEvent::WebFetchResult {
                                                request_id: tool_use_id.to_string(),
                                                url: url.to_string(),
                                                content_preview,
                                            },
                                        );
                                    }
                                } else {
                                    // Regular text content
                                    text_content.push_str(&text_msg.text);
                                    accumulated_response.push_str(&text_msg.text);
                                    let _ = ctx.event_tx.send(AiEvent::TextDelta {
                                        delta: text_msg.text,
                                        accumulated: accumulated_response.clone(),
                                    });
                                }
                            }
                        }
                        StreamedAssistantContent::Reasoning(reasoning) => {
                            // Native reasoning/thinking content from extended thinking models
                            let reasoning_text = reasoning.reasoning.join("");
                            if supports_thinking {
                                tracing::trace!(
                                    "[Unified] Received native reasoning chunk #{}: {} chars, has_signature: {}",
                                    chunk_count,
                                    reasoning_text.len(),
                                    reasoning.signature.is_some()
                                );
                                thinking_content.push_str(&reasoning_text);
                                accumulated_thinking.push_str(&reasoning_text);
                                // Capture the signature (needed for Anthropic API when sending back history)
                                if reasoning.signature.is_some() {
                                    thinking_signature = reasoning.signature.clone();
                                }
                                // Capture the ID (needed for OpenAI Responses API - rs_... IDs that function calls reference)
                                if reasoning.id.is_some() {
                                    thinking_id = reasoning.id.clone();
                                }
                            }
                            // Always emit reasoning event (to frontend and sidecar)
                            emit_event(
                                ctx,
                                AiEvent::Reasoning {
                                    content: reasoning_text,
                                },
                            );
                        }
                        StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                            // Streaming reasoning delta (similar to Reasoning but delivered as deltas)
                            if supports_thinking {
                                tracing::trace!(
                                    "[Unified] Received reasoning delta chunk #{}: {} chars",
                                    chunk_count,
                                    reasoning.len()
                                );
                                thinking_content.push_str(&reasoning);
                                accumulated_thinking.push_str(&reasoning);
                                // Capture the ID if present (for OpenAI Responses API)
                                if id.is_some() && thinking_id.is_none() {
                                    thinking_id = id;
                                }
                            }
                            // Always emit reasoning event (to frontend and sidecar)
                            emit_event(ctx, AiEvent::Reasoning { content: reasoning });
                        }
                        StreamedAssistantContent::ToolCall(tool_call) => {
                            // Check if this is a server tool (executed by provider, not us)
                            let is_server_tool = tool_call
                                .call_id
                                .as_ref()
                                .map(|id| id.starts_with("server:"))
                                .unwrap_or(false);

                            if is_server_tool {
                                // Server tool (web_search/web_fetch) - already executed by provider
                                tracing::info!(
                                    "Server tool detected: {} ({})",
                                    tool_call.function.name,
                                    tool_call.id
                                );
                                emit_event(
                                    ctx,
                                    AiEvent::ServerToolStarted {
                                        request_id: tool_call.id.clone(),
                                        tool_name: tool_call.function.name.clone(),
                                        input: tool_call.function.arguments.clone(),
                                    },
                                );
                                // Don't add to tool_calls_to_execute - provider handles execution
                                continue;
                            }

                            has_tool_calls = true;

                            // Finalize any previous pending tool call first
                            if let (Some(prev_id), Some(prev_name)) =
                                (current_tool_id.take(), current_tool_name.take())
                            {
                                let args: serde_json::Value =
                                    serde_json::from_str(&current_tool_args)
                                        .unwrap_or(serde_json::Value::Null);
                                tool_calls_to_execute.push(ToolCall {
                                    id: prev_id.clone(),
                                    call_id: Some(prev_id),
                                    function: rig::message::ToolFunction {
                                        name: prev_name,
                                        arguments: args,
                                    },
                                    signature: None,
                                    additional_params: None,
                                });
                                current_tool_args.clear();
                            }

                            // Check if this tool call has complete args (non-streaming case)
                            // If args are empty object {}, we'll wait for deltas
                            let has_complete_args = !tool_call.function.arguments.is_null()
                                && tool_call.function.arguments != serde_json::json!({});

                            if has_complete_args {
                                // Tool call came complete, add directly
                                // Ensure call_id is set for OpenAI compatibility
                                let mut tool_call = tool_call;
                                if tool_call.call_id.is_none() {
                                    tool_call.call_id = Some(tool_call.id.clone());
                                }
                                tool_calls_to_execute.push(tool_call);
                            } else {
                                // Tool call has empty args, wait for deltas
                                current_tool_id = Some(tool_call.id.clone());
                                current_tool_name = Some(tool_call.function.name.clone());
                                // Start with any existing args (might be empty object serialized)
                                if !tool_call.function.arguments.is_null()
                                    && tool_call.function.arguments != serde_json::json!({})
                                {
                                    current_tool_args = tool_call.function.arguments.to_string();
                                }
                            }
                        }
                        StreamedAssistantContent::ToolCallDelta { id, content } => {
                            // If we don't have a current tool ID but the delta has one, use it
                            if current_tool_id.is_none() && !id.is_empty() {
                                current_tool_id = Some(id);
                            }
                            // Accumulate tool call argument deltas (extract string from enum)
                            if let rig::streaming::ToolCallDeltaContent::Delta(delta) = content {
                                current_tool_args.push_str(&delta);
                            }
                        }
                        StreamedAssistantContent::Final(ref resp) => {
                            // Extract and accumulate token usage
                            if let Some(usage) = resp.token_usage() {
                                total_usage.input_tokens += usage.input_tokens;
                                total_usage.output_tokens += usage.output_tokens;
                                // Record token usage as span attributes for Langfuse
                                // Using prompt_tokens/completion_tokens per GenAI semantic conventions
                                llm_span.record(
                                    "gen_ai.usage.prompt_tokens",
                                    usage.input_tokens as i64,
                                );
                                llm_span.record(
                                    "gen_ai.usage.completion_tokens",
                                    usage.output_tokens as i64,
                                );
                                tracing::info!(
                                    "[compaction] Token usage iter {}: input={}, output={}, cumulative={}",
                                    iteration,
                                    usage.input_tokens,
                                    usage.output_tokens,
                                    total_usage.total()
                                );

                                // Update compaction state with provider token count
                                {
                                    let mut compaction_state = ctx.compaction_state.write().await;
                                    compaction_state.update_tokens(usage.input_tokens);
                                    tracing::info!(
                                        "[compaction] State updated: {} input tokens from provider",
                                        usage.input_tokens
                                    );
                                }

                                // Emit context utilization event for frontend
                                let model_config = qbit_context::TokenBudgetConfig::for_model(ctx.model_name);
                                let max_tokens = model_config.max_context_tokens;
                                let utilization = usage.input_tokens as f64 / max_tokens as f64;
                                let _ = ctx.event_tx.send(AiEvent::ContextWarning {
                                    utilization,
                                    total_tokens: usage.input_tokens as usize,
                                    max_tokens,
                                });
                            } else {
                                // Fallback: estimate tokens from message content using heuristic
                                let total_chars: usize = chat_history
                                    .iter()
                                    .map(estimate_message_chars)
                                    .sum();
                                let estimated_tokens = total_chars / 4;

                                // Update total_usage with heuristic estimate so it's reported to frontend
                                // We split roughly 80/20 input/output as a reasonable approximation
                                let estimated_input = (estimated_tokens as f64 * 0.8) as u64;
                                let estimated_output = (estimated_tokens as f64 * 0.2) as u64;
                                total_usage.input_tokens += estimated_input;
                                total_usage.output_tokens += estimated_output;

                                {
                                    let mut compaction_state = ctx.compaction_state.write().await;
                                    compaction_state.update_tokens_heuristic(total_chars);
                                    tracing::info!(
                                        "[compaction] State updated (heuristic): ~{} estimated tokens from {} chars",
                                        estimated_tokens,
                                        total_chars
                                    );
                                }

                                // Emit context utilization event for frontend (heuristic)
                                let model_config = qbit_context::TokenBudgetConfig::for_model(ctx.model_name);
                                let max_tokens = model_config.max_context_tokens;
                                let utilization = estimated_tokens as f64 / max_tokens as f64;
                                let _ = ctx.event_tx.send(AiEvent::ContextWarning {
                                    utilization,
                                    total_tokens: estimated_tokens,
                                    max_tokens,
                                });
                            }

                            // Finalize any pending tool call from deltas
                            if let (Some(id), Some(name)) =
                                (current_tool_id.take(), current_tool_name.take())
                            {
                                let args: serde_json::Value =
                                    serde_json::from_str(&current_tool_args)
                                        .unwrap_or(serde_json::Value::Null);
                                tool_calls_to_execute.push(ToolCall {
                                    id: id.clone(),
                                    call_id: Some(id),
                                    function: rig::message::ToolFunction {
                                        name,
                                        arguments: args,
                                    },
                                    signature: None,
                                    additional_params: None,
                                });
                                current_tool_args.clear();
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Stream chunk error at #{}: {}", chunk_count, e);
                }
            }
        }

        tracing::info!(
            "[OpenAI Debug] Stream completed: iteration={}, chunks={}, text_chars={}, thinking_chars={}, tool_calls={}",
            iteration,
            chunk_count,
            text_content.len(),
            thinking_content.len(),
            tool_calls_to_execute.len()
        );
        tracing::debug!(
            "Stream completed (unified): {} chunks, {} chars text, {} chars thinking, {} tool calls",
            chunk_count,
            text_content.len(),
            thinking_content.len(),
            tool_calls_to_execute.len()
        );

        // Record the completion for Langfuse (truncated to avoid huge spans)
        // Only record text content - tool call details are in child tool spans
        if !text_content.is_empty() {
            let completion_for_span = if text_content.len() > 2000 {
                // Find a safe UTF-8 char boundary at or before position 2000
                let mut end = 2000;
                while end > 0 && !text_content.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}... [truncated]", &text_content[..end])
            } else {
                text_content.clone()
            };
            llm_span.record("gen_ai.completion", completion_for_span.as_str());
            llm_span.record("langfuse.observation.output", completion_for_span.as_str());
        }
        // When only tool calls: don't record output, let child tool spans provide details

        // Finalize any remaining tool call that wasn't closed by FinalResponse
        if let (Some(id), Some(name)) = (current_tool_id.take(), current_tool_name.take()) {
            let args: serde_json::Value =
                serde_json::from_str(&current_tool_args).unwrap_or(serde_json::Value::Null);
            tool_calls_to_execute.push(ToolCall {
                id: id.clone(),
                call_id: Some(id),
                function: rig::message::ToolFunction {
                    name,
                    arguments: args,
                },
                signature: None,
                additional_params: None,
            });
            has_tool_calls = true;
        }

        // Log thinking content if present (for debugging)
        if supports_thinking && !thinking_content.is_empty() {
            tracing::debug!("Model thinking: {} chars", thinking_content.len());
        }

        // Build assistant content for history
        // IMPORTANT: When thinking is enabled, thinking blocks MUST come first (required by Anthropic API)
        let mut assistant_content: Vec<AssistantContent> = vec![];

        // Conditionally add thinking content first (required by Anthropic API when thinking is enabled)
        // OpenAI Responses API reasoning handling:
        // - When there ARE tool calls: MUST include reasoning because tool calls reference it via rs_... IDs
        //   (otherwise: "function_call was provided without its required 'reasoning' item")
        // - When there are NO tool calls: MUST NOT include reasoning as standalone
        //   (otherwise: "reasoning was provided without its required following item")
        let is_openai_responses = ctx.provider_name == "openai_responses";
        let has_reasoning = !thinking_content.is_empty() || thinking_id.is_some();
        let should_include_reasoning = if is_openai_responses {
            // For OpenAI Responses API: only include reasoning when there are tool calls
            has_reasoning && has_tool_calls
        } else {
            // For other providers (Anthropic, etc.): include reasoning when present
            has_reasoning
        };
        if supports_thinking && should_include_reasoning {
            assistant_content.push(AssistantContent::Reasoning(
                Reasoning::multi(vec![thinking_content.clone()])
                    .optional_id(thinking_id.clone())
                    .with_signature(thinking_signature.clone()),
            ));
        }

        if !text_content.is_empty() {
            assistant_content.push(AssistantContent::Text(Text {
                text: text_content.clone(),
            }));
        }

        // Add tool calls to assistant content if present
        for tool_call in &tool_calls_to_execute {
            assistant_content.push(AssistantContent::ToolCall(tool_call.clone()));
        }

        // ALWAYS add assistant message to history (even when no tool calls)
        // This is critical for maintaining conversation context across turns
        if !assistant_content.is_empty() {
            chat_history.push(Message::Assistant {
                id: None,
                content: OneOrMany::many(assistant_content).unwrap_or_else(|_| {
                    OneOrMany::one(AssistantContent::Text(Text {
                        text: String::new(),
                    }))
                }),
            });
        }

        // If no tool calls, we're done
        if !has_tool_calls {
            break;
        }

        // Execute tool calls and collect results
        let mut tool_results: Vec<UserContent> = vec![];
        let mut system_hooks: Vec<String> = vec![];

        for tool_call in tool_calls_to_execute {
            let tool_name = &tool_call.function.name;
            // Normalize run_command/run_pty_cmd args to convert array commands to strings
            let tool_args = if tool_name == "run_pty_cmd" || tool_name == "run_command" {
                normalize_run_pty_cmd_args(tool_call.function.arguments.clone())
            } else {
                tool_call.function.arguments.clone()
            };
            let tool_id = tool_call.id.clone();
            // For OpenAI, call_id is different from id (call_* vs fc_*)
            // Use call_id for tool results if available, otherwise fall back to id
            let tool_call_id = tool_call.call_id.clone().unwrap_or_else(|| tool_id.clone());

            // Create span for tool call (child of llm_span since tools execute during LLM iteration)
            // Truncate args for span to avoid huge spans (use truncate_str for UTF-8 safety)
            let args_str = serde_json::to_string(&tool_args).unwrap_or_default();
            let args_for_span = if args_str.len() > 1000 {
                format!("{}... [truncated]", truncate_str(&args_str, 1000))
            } else {
                args_str
            };
            // Build span with tool name for better Langfuse display
            let tool_span = tracing::info_span!(
                parent: &llm_span,
                "tool_call",
                "otel.name" = %tool_name,  // Override span name in OpenTelemetry
                "langfuse.span.name" = %tool_name,  // Langfuse-specific name override
                "langfuse.observation.type" = "tool",  // Use tool type for proper categorization
                "langfuse.session.id" = ctx.session_id.unwrap_or(""),
                tool.name = %tool_name,
                tool.id = %tool_id,
                "langfuse.observation.input" = %args_for_span,
                "langfuse.observation.output" = tracing::field::Empty,
                success = tracing::field::Empty,
            );
            // Note: We use explicit parent instead of span.enter() for async compatibility

            // Check for loop detection
            let loop_result = {
                let mut detector = ctx.loop_detector.write().await;
                detector.record_tool_call(tool_name, &tool_args)
            };

            // Handle loop detection (may add a blocked result)
            if let Some(blocked_result) =
                handle_loop_detection(&loop_result, &tool_id, &tool_call_id, ctx.event_tx)
            {
                // Record loop blocked event in Langfuse
                let loop_info = match &loop_result {
                    qbit_loop_detection::LoopDetectionResult::Blocked {
                        repeat_count,
                        max_count,
                        ..
                    } => {
                        format!("repeat_count={}, max={}", repeat_count, max_count)
                    }
                    qbit_loop_detection::LoopDetectionResult::MaxIterationsReached {
                        iterations,
                        max_iterations,
                        ..
                    } => {
                        format!("iterations={}, max={}", iterations, max_iterations)
                    }
                    _ => String::new(),
                };
                let _loop_event = tracing::info_span!(
                    parent: &llm_span,
                    "loop_blocked",
                    "langfuse.observation.type" = "event",
                    "langfuse.session.id" = ctx.session_id.unwrap_or(""),
                    tool_name = %tool_name,
                    details = %loop_info,
                );

                tool_span.record("success", false);
                tool_span.record("langfuse.observation.output", "blocked by loop detection");
                tool_results.push(blocked_result);
                continue;
            }

            // Execute tool with HITL approval check (generic version)
            let result = execute_with_hitl_generic(
                tool_name,
                &tool_args,
                &tool_id,
                ctx,
                &mut capture_ctx,
                model,
                &sub_agent_context,
            )
            .await
            .unwrap_or_else(|e| ToolExecutionResult {
                value: json!({ "error": e.to_string() }),
                success: false,
            });

            // Record tool result in span (use truncate_str for UTF-8 safety)
            let result_str = serde_json::to_string(&result.value).unwrap_or_default();
            let result_for_span = if result_str.len() > 1000 {
                format!("{}... [truncated]", truncate_str(&result_str, 1000))
            } else {
                result_str
            };
            tool_span.record("langfuse.observation.output", result_for_span.as_str());
            tool_span.record("success", result.success);

            // Emit tool result event (to frontend and capture to sidecar with state)
            let result_event = AiEvent::ToolResult {
                tool_name: tool_name.clone(),
                result: result.value.clone(),
                success: result.success,
                request_id: tool_id.clone(),
                source: qbit_core::events::ToolSource::Main,
            };
            emit_to_frontend(ctx, result_event.clone());
            capture_ctx.process(&result_event);

            // Convert result to text and truncate if necessary
            let raw_result_text = serde_json::to_string(&result.value).unwrap_or_default();
            let truncation_result = ctx
                .context_manager
                .truncate_tool_response(&raw_result_text, tool_name)
                .await;

            // Emit truncation event if truncation occurred
            if truncation_result.truncated {
                let original_tokens =
                    qbit_context::TokenBudgetManager::estimate_tokens(&raw_result_text);
                let truncated_tokens =
                    qbit_context::TokenBudgetManager::estimate_tokens(&truncation_result.content);
                let _ = ctx.event_tx.send(AiEvent::ToolResponseTruncated {
                    tool_name: tool_name.clone(),
                    original_tokens,
                    truncated_tokens,
                });
            }

            // Add to tool results for LLM (using truncated content)
            // Use tool_call_id for call_id (OpenAI requires call_* format)
            tool_results.push(UserContent::ToolResult(ToolResult {
                id: tool_id.clone(),
                call_id: Some(tool_call_id),
                content: OneOrMany::one(ToolResultContent::Text(Text {
                    text: truncation_result.content,
                })),
            }));

            // Run post-tool hooks based on tool execution result
            let post_ctx = PostToolContext::new(
                tool_name,
                &tool_args,
                &result.value,
                result.success,
                0, // duration_ms not tracked here currently
                ctx.session_id.unwrap_or(""),
            );
            system_hooks.extend(hook_registry.run_post_tool_hooks(&post_ctx));
        }

        // Add tool results as user message
        chat_history.push(Message::User {
            content: OneOrMany::many(tool_results).unwrap_or_else(|_| {
                OneOrMany::one(UserContent::Text(Text {
                    text: "Tool executed".to_string(),
                }))
            }),
        });

        // Push queued system hooks as separate user message
        if !system_hooks.is_empty() {
            let formatted_hooks = format_system_hooks(&system_hooks);

            // Log injection at info level
            tracing::info!(
                count = system_hooks.len(),
                content_len = formatted_hooks.len(),
                "Injecting system hooks as user message"
            );

            // Emit to frontend so the timeline can display the injected hooks.
            let _ = ctx
                .event_tx
                .send(AiEvent::SystemHooksInjected { hooks: system_hooks.clone() });

            // Create OTel event for Langfuse visibility
            let _system_hook_event = tracing::info_span!(
                parent: &llm_span,
                "system_hooks_injected",
                "langfuse.observation.type" = "event",
                "langfuse.observation.level" = "DEFAULT",
                "langfuse.session.id" = ctx.session_id.unwrap_or(""),
                hook_count = system_hooks.len(),
                "langfuse.observation.input" = %formatted_hooks,
            );

            chat_history.push(Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: formatted_hooks,
                })),
            });
        }
    }

    // Log thinking stats at debug level
    if supports_thinking && !accumulated_thinking.is_empty() {
        tracing::debug!(
            "[Unified] Total thinking content: {} chars",
            accumulated_thinking.len()
        );
    }

    let agent_label = if config.is_sub_agent {
        format!("sub-agent (depth={})", sub_agent_context.depth)
    } else {
        "main-agent".to_string()
    };
    tracing::info!(
        "[{}] Turn complete: provider={}, model={}, tokens={{input={}, output={}, total={}}}",
        agent_label,
        ctx.provider_name,
        ctx.model_name,
        total_usage.input_tokens,
        total_usage.output_tokens,
        total_usage.total()
    );

        Ok::<_, anyhow::Error>((accumulated_response, accumulated_thinking, chat_history, total_usage))
    }
    .instrument(agent_span.clone())
    .instrument(chat_message_span.clone())
    .await?;

    // Record the final output on both trace and agent spans
    let output_for_span = if accumulated_response.len() > 2000 {
        format!("{}... [truncated]", &accumulated_response[..2000])
    } else {
        accumulated_response.clone()
    };
    chat_message_span.record("langfuse.observation.output", output_for_span.as_str());
    agent_span.record("langfuse.observation.output", output_for_span.as_str());

    // Convert accumulated_thinking to Option (None if empty)
    let reasoning = if accumulated_thinking.is_empty() {
        None
    } else {
        Some(accumulated_thinking)
    };

    Ok((
        accumulated_response,
        reasoning,
        chat_history,
        Some(total_usage),
    ))
}

// =============================================================================
// CONTEXT COMPACTION ORCHESTRATION
// =============================================================================

use std::path::PathBuf;

/// Result of a context compaction attempt.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Whether compaction was successful
    pub success: bool,
    /// The generated summary (if successful)
    pub summary: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Number of tokens before compaction
    pub tokens_before: u64,
    /// Number of messages before compaction
    pub messages_before: usize,
}

/// Get the transcript directory path.
///
/// Returns the path to `~/.qbit/transcripts/` by default.
pub fn get_transcript_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".qbit")
        .join("transcripts")
}

/// Get the artifacts directory path for compaction-related files.
///
/// Returns the path to `~/.qbit/artifacts/compaction/` by default.
pub fn get_artifacts_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".qbit")
        .join("artifacts")
        .join("compaction")
}

/// Get the summaries directory path.
///
/// Returns the path to `~/.qbit/artifacts/summaries/` by default.
pub fn get_summaries_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".qbit")
        .join("artifacts")
        .join("summaries")
}

/// Check if compaction should be triggered and perform it if needed.
///
/// This function:
/// 1. Checks if compaction threshold is exceeded
/// 2. If so, generates a summary and compacts the context
/// 3. Updates the message history with the compacted version
///
/// # Arguments
/// * `ctx` - The agentic loop context
/// * `session_id` - The session ID for transcript loading
/// * `chat_history` - The current message history (will be modified if compaction occurs)
///
/// # Returns
/// * `Ok(Some(CompactionResult))` - If compaction was attempted (success or failure)
/// * `Ok(None)` - If compaction was not needed
pub async fn maybe_compact(
    ctx: &AgenticLoopContext<'_>,
    session_id: &str,
    chat_history: &mut Vec<Message>,
) -> Result<Option<CompactionResult>> {
    // Check if compaction should be triggered
    let compaction_state = ctx.compaction_state.read().await;
    let check = ctx
        .context_manager
        .should_compact(&compaction_state, ctx.model_name);
    drop(compaction_state);

    // Log the compaction check result with full details
    let threshold_tokens = (check.max_tokens as f64 * check.threshold) as u64;
    tracing::info!(
        "[compaction] Check: model={}, current={}, threshold={} ({}% of {}), should_compact={}",
        ctx.model_name,
        check.current_tokens,
        threshold_tokens,
        (check.threshold * 100.0) as u32,
        check.max_tokens,
        check.should_compact
    );

    if !check.should_compact {
        tracing::info!(
            "[compaction] Not triggered: {} (need {} more tokens)",
            check.reason,
            threshold_tokens.saturating_sub(check.current_tokens)
        );
        return Ok(None);
    }

    tracing::info!(
        "[compaction] Triggered: tokens={}/{}, threshold={:.0}%, reason={}",
        check.current_tokens,
        check.max_tokens,
        check.threshold * 100.0,
        check.reason
    );

    // Emit CompactionStarted event
    let _ = ctx.event_tx.send(AiEvent::CompactionStarted {
        tokens_before: check.current_tokens,
        messages_before: chat_history.len(),
    });

    // Mark that we've attempted compaction this turn
    {
        let mut compaction_state = ctx.compaction_state.write().await;
        compaction_state.mark_attempted();
    }

    // Perform the compaction
    let result = perform_compaction(ctx, session_id, chat_history, check.current_tokens).await;

    // Update compaction state based on result
    if result.success {
        let mut compaction_state = ctx.compaction_state.write().await;
        compaction_state.increment_count();
    }

    Ok(Some(result))
}

/// Perform context compaction by summarizing the conversation and replacing history.
///
/// This function:
/// 1. Builds summarizer input from the transcript
/// 2. Saves the summarizer input for debugging
/// 3. Generates a summary using the LLM
/// 4. Saves the summary for debugging
/// 5. Replaces the message history with a compacted version
///
/// # Arguments
/// * `ctx` - The agentic loop context
/// * `session_id` - The session ID for transcript loading
/// * `chat_history` - The current message history (will be modified)
/// * `tokens_before` - Token count before compaction
///
/// # Returns
/// A CompactionResult indicating success/failure and details
async fn perform_compaction(
    ctx: &AgenticLoopContext<'_>,
    session_id: &str,
    chat_history: &mut Vec<Message>,
    tokens_before: u64,
) -> CompactionResult {
    let messages_before = chat_history.len();
    let transcript_dir = get_transcript_dir();
    let artifacts_dir = get_artifacts_dir();
    let summaries_dir = get_summaries_dir();

    // Step 1: Build summarizer input from transcript
    let summarizer_input =
        match crate::transcript::build_summarizer_input(&transcript_dir, session_id) {
            Ok(input) => input,
            Err(e) => {
                tracing::warn!("[compaction] Failed to build summarizer input: {}", e);
                return CompactionResult {
                    success: false,
                    summary: None,
                    error: Some(format!("Failed to build summarizer input: {}", e)),
                    tokens_before,
                    messages_before,
                };
            }
        };

    // Step 2: Save summarizer input for debugging
    if let Err(e) =
        crate::transcript::save_summarizer_input(&artifacts_dir, session_id, &summarizer_input)
    {
        tracing::warn!("[compaction] Failed to save summarizer input: {}", e);
        // Continue - this is not fatal
    }

    tracing::info!(
        "[compaction] Calling summarizer with {} chars of conversation",
        summarizer_input.len()
    );

    // Step 3: Generate summary using the LLM client
    let client = ctx.client.read().await;
    let summary_result = crate::summarizer::generate_summary(&client, &summarizer_input).await;
    drop(client); // Release read lock

    let summary = match summary_result {
        Ok(response) => response.summary,
        Err(e) => {
            tracing::error!("[compaction] Summarizer failed: {}", e);
            let _ = ctx.event_tx.send(AiEvent::Warning {
                message: format!("Context compaction failed: {}", e),
            });
            return CompactionResult {
                success: false,
                summary: None,
                error: Some(format!("Summarizer failed: {}", e)),
                tokens_before,
                messages_before,
            };
        }
    };

    tracing::info!("[compaction] Summary generated: {} chars", summary.len());

    // Step 4: Save summary for debugging
    if let Err(e) = crate::transcript::save_summary(&summaries_dir, session_id, &summary) {
        tracing::warn!("[compaction] Failed to save summary: {}", e);
        // Continue - this is not fatal
    }

    // Step 5: Apply compaction to chat history
    let messages_removed = apply_compaction(chat_history, &summary);

    tracing::info!(
        "[compaction] Compaction complete: {} messages removed, {} remaining",
        messages_removed,
        chat_history.len()
    );

    CompactionResult {
        success: true,
        summary: Some(summary),
        error: None,
        tokens_before,
        messages_before,
    }
}

/// Apply a summary to replace the message history with a compacted version.
///
/// This function takes a generated summary and creates a new message history
/// that contains just the summary as context, preserving the most recent
/// user message.
///
/// # Arguments
/// * `chat_history` - The current message history (will be modified)
/// * `summary` - The generated summary to use as context
///
/// # Returns
/// The number of messages removed
pub fn apply_compaction(chat_history: &mut Vec<Message>, summary: &str) -> usize {
    let original_len = chat_history.len();

    // Extract the last user message before clearing (so agent knows what to continue with)
    let last_user_message = chat_history.iter().rev().find_map(|msg| {
        if let Message::User { content } = msg {
            // Extract text content from the user message
            let text = content
                .iter()
                .filter_map(|c| {
                    if let UserContent::Text(t) = c {
                        Some(t.text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            if !text.is_empty() {
                Some(text)
            } else {
                None
            }
        } else {
            None
        }
    });

    // Clear the history
    chat_history.clear();

    // Build the combined message with summary and last user request
    let message_text = match last_user_message {
        Some(last_msg) => format!(
            "[Context Summary - Previous conversation has been compacted]\n\n{}\n\n[End of Summary]\n\nThe user's most recent request was:\n\n{}",
            summary,
            last_msg
        ),
        None => format!(
            "[Context Summary - Previous conversation has been compacted]\n\n{}\n\n[End of Summary]",
            summary
        ),
    };

    let summary_message = Message::User {
        content: OneOrMany::one(UserContent::Text(Text { text: message_text })),
    };
    chat_history.push(summary_message);

    original_len.saturating_sub(chat_history.len())
}

#[cfg(test)]
mod unified_loop_tests {
    use super::*;

    #[test]
    fn test_agentic_loop_config_main_agent_anthropic() {
        let config = AgenticLoopConfig::main_agent_anthropic();
        assert!(
            config.capabilities.supports_thinking_history,
            "Anthropic config should support thinking history"
        );
        assert!(
            config.capabilities.supports_temperature,
            "Anthropic config should support temperature"
        );
        assert!(config.require_hitl, "Main agent should require HITL");
        assert!(!config.is_sub_agent, "Main agent should not be sub-agent");
    }

    #[test]
    fn test_agentic_loop_config_main_agent_generic() {
        let config = AgenticLoopConfig::main_agent_generic();
        assert!(
            !config.capabilities.supports_thinking_history,
            "Generic config should not support thinking history"
        );
        assert!(
            config.capabilities.supports_temperature,
            "Generic config should support temperature"
        );
        assert!(config.require_hitl, "Main agent should require HITL");
        assert!(!config.is_sub_agent, "Main agent should not be sub-agent");
    }

    #[test]
    fn test_agentic_loop_config_sub_agent() {
        let config = AgenticLoopConfig::sub_agent(ModelCapabilities::conservative_defaults());
        assert!(
            !config.capabilities.supports_thinking_history,
            "Conservative defaults should not support thinking history"
        );
        assert!(!config.require_hitl, "Sub-agent should not require HITL");
        assert!(config.is_sub_agent, "Should be marked as sub-agent");
    }

    #[test]
    fn test_agentic_loop_config_sub_agent_with_anthropic_capabilities() {
        let config = AgenticLoopConfig::sub_agent(ModelCapabilities::anthropic_defaults());
        assert!(
            config.capabilities.supports_thinking_history,
            "Anthropic sub-agent should support thinking history"
        );
        assert!(!config.require_hitl, "Sub-agent should not require HITL");
        assert!(config.is_sub_agent, "Should be marked as sub-agent");
    }

    #[test]
    fn test_agentic_loop_config_with_detection_anthropic() {
        let config = AgenticLoopConfig::with_detection("anthropic", "claude-3-opus", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "Anthropic detection should enable thinking history"
        );
        assert!(
            config.capabilities.supports_temperature,
            "Anthropic detection should enable temperature"
        );
        assert!(config.require_hitl, "Non-sub-agent should require HITL");
        assert!(!config.is_sub_agent);
    }

    #[test]
    fn test_agentic_loop_config_with_detection_openai_reasoning() {
        let config = AgenticLoopConfig::with_detection("openai", "o3-mini", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "OpenAI reasoning model should support thinking history"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "OpenAI reasoning model should not support temperature"
        );
        assert!(config.require_hitl);
    }

    #[test]
    fn test_agentic_loop_config_with_detection_openai_regular() {
        let config = AgenticLoopConfig::with_detection("openai", "gpt-4o", false);
        assert!(
            !config.capabilities.supports_thinking_history,
            "Regular OpenAI model should not support thinking history"
        );
        assert!(
            config.capabilities.supports_temperature,
            "Regular OpenAI model should support temperature"
        );
    }

    #[test]
    fn test_agentic_loop_config_with_detection_sub_agent() {
        let config = AgenticLoopConfig::with_detection("openai", "gpt-4o", true);
        assert!(!config.require_hitl, "Sub-agent should not require HITL");
        assert!(config.is_sub_agent, "Should be marked as sub-agent");
    }

    #[test]
    fn test_agentic_loop_config_with_detection_openai_gpt5_series() {
        // GPT-5 base model
        let config = AgenticLoopConfig::with_detection("openai", "gpt-5", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "GPT-5 should support thinking history (reasoning model)"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "GPT-5 should not support temperature (reasoning model)"
        );

        // GPT-5.1
        let config = AgenticLoopConfig::with_detection("openai", "gpt-5.1", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "GPT-5.1 should support thinking history"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "GPT-5.1 should not support temperature"
        );

        // GPT-5.2
        let config = AgenticLoopConfig::with_detection("openai", "gpt-5.2", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "GPT-5.2 should support thinking history"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "GPT-5.2 should not support temperature"
        );

        // GPT-5-mini
        let config = AgenticLoopConfig::with_detection("openai", "gpt-5-mini", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "GPT-5-mini should support thinking history"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "GPT-5-mini should not support temperature"
        );
    }

    #[test]
    fn test_agentic_loop_config_with_detection_openai_responses_gpt5() {
        // OpenAI Responses API with GPT-5.2
        let config = AgenticLoopConfig::with_detection("openai_responses", "gpt-5.2", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "OpenAI Responses API should support thinking history"
        );
        assert!(
            !config.capabilities.supports_temperature,
            "GPT-5.2 via Responses API should not support temperature"
        );

        // Contrast with GPT-4.1 which DOES support temperature
        let config = AgenticLoopConfig::with_detection("openai_responses", "gpt-4.1", false);
        assert!(
            config.capabilities.supports_thinking_history,
            "OpenAI Responses API should support thinking history"
        );
        assert!(
            config.capabilities.supports_temperature,
            "GPT-4.1 via Responses API should support temperature"
        );
    }

    #[test]
    fn test_agentic_loop_config_with_detection_openai_codex() {
        // Codex models don't support temperature
        let config = AgenticLoopConfig::with_detection("openai", "gpt-5.1-codex-max", false);
        assert!(
            !config.capabilities.supports_temperature,
            "Codex models should not support temperature"
        );

        let config = AgenticLoopConfig::with_detection("openai_responses", "gpt-5.2-codex", false);
        assert!(
            !config.capabilities.supports_temperature,
            "Codex models via Responses API should not support temperature"
        );
    }
}

#[cfg(test)]
mod utf8_truncation_tests {
    #[test]
    fn test_utf8_safe_truncation_ascii() {
        let text = "Hello, World!";
        let mut end = 5;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(&text[..end], "Hello");
    }

    #[test]
    fn test_utf8_safe_truncation_multibyte() {
        // "─" is 3 bytes (E2 94 80), testing truncation at various positions
        let text = "abc─def"; // a=0, b=1, c=2, ─=3-5, d=6, e=7, f=8

        // Truncate at position 4 (middle of ─)
        let mut end = 4;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(end, 3); // Should back up to position 3 (start of ─)
        assert_eq!(&text[..end], "abc");

        // Truncate at position 5 (still in ─)
        let mut end = 5;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(end, 3);
        assert_eq!(&text[..end], "abc");

        // Truncate at position 6 (after ─)
        let mut end = 6;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(end, 6);
        assert_eq!(&text[..end], "abc─");
    }

    #[test]
    fn test_utf8_safe_truncation_emoji() {
        // Emoji like 🎉 is 4 bytes
        let text = "Hi🎉!"; // H=0, i=1, 🎉=2-5, !=6

        // Truncate at position 3 (middle of emoji)
        let mut end = 3;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(end, 2);
        assert_eq!(&text[..end], "Hi");

        // Truncate at position 6 (after emoji)
        let mut end = 6;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        assert_eq!(end, 6);
        assert_eq!(&text[..end], "Hi🎉");
    }

    #[test]
    fn test_utf8_safe_truncation_mixed_box_drawing() {
        // Box drawing characters like those that caused the original panic
        let text = "Summary:\n─────────";
        let target = 12; // Might land in middle of a box char

        let mut end = target.min(text.len());
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }

        // Should not panic and result should be valid UTF-8
        let truncated = &text[..end];
        assert!(truncated.len() <= target);
        // Verify it's valid UTF-8 by checking we can iterate chars
        assert!(truncated.chars().count() > 0);
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    #[test]
    fn test_get_transcript_dir() {
        let path = get_transcript_dir();
        assert!(path.to_string_lossy().contains(".qbit"));
        assert!(path.to_string_lossy().contains("transcripts"));
    }

    #[test]
    fn test_get_artifacts_dir() {
        let path = get_artifacts_dir();
        assert!(path.to_string_lossy().contains(".qbit"));
        assert!(path.to_string_lossy().contains("artifacts"));
        assert!(path.to_string_lossy().contains("compaction"));
    }

    #[test]
    fn test_get_summaries_dir() {
        let path = get_summaries_dir();
        assert!(path.to_string_lossy().contains(".qbit"));
        assert!(path.to_string_lossy().contains("artifacts"));
        assert!(path.to_string_lossy().contains("summaries"));
    }

    #[test]
    fn test_compaction_result_default_fields() {
        let result = CompactionResult {
            success: false,
            summary: None,
            error: Some("test error".to_string()),
            tokens_before: 100_000,
            messages_before: 50,
        };

        assert!(!result.success);
        assert!(result.summary.is_none());
        assert_eq!(result.error, Some("test error".to_string()));
        assert_eq!(result.tokens_before, 100_000);
        assert_eq!(result.messages_before, 50);
    }

    #[test]
    fn test_apply_compaction_empty_history() {
        let mut history: Vec<Message> = vec![];
        let removed = apply_compaction(&mut history, "Test summary");

        // Should have added the summary message
        assert_eq!(history.len(), 1);
        assert_eq!(removed, 0); // No messages were "removed" since we started with 0
    }

    #[test]
    fn test_apply_compaction_replaces_all_messages() {
        let mut history = vec![
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "First message".to_string(),
                })),
            },
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "Last message".to_string(),
                })),
            },
        ];

        let removed = apply_compaction(&mut history, "Test summary");

        // Should only have summary (all messages replaced)
        assert_eq!(history.len(), 1);
        assert_eq!(removed, 1); // 2 - 1 = 1

        // Verify it's the summary
        if let Message::User { ref content } = history[0] {
            let text = content.iter().next().unwrap();
            if let UserContent::Text(t) = text {
                assert!(t.text.contains("[Context Summary"));
                assert!(t.text.contains("Test summary"));
            } else {
                panic!("Expected text content");
            }
        } else {
            panic!("Expected user message");
        }
    }

    #[test]
    fn test_apply_compaction_removes_many_messages() {
        let mut history: Vec<Message> = (0..10)
            .map(|i| Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: format!("Message {}", i),
                })),
            })
            .collect();

        let removed = apply_compaction(&mut history, "Comprehensive summary");

        // Should only have summary
        assert_eq!(history.len(), 1);
        assert_eq!(removed, 9); // 10 - 1 = 9
    }

    #[test]
    fn test_apply_compaction_summary_format() {
        let mut history = vec![Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: "Original message".to_string(),
            })),
        }];

        apply_compaction(&mut history, "This is the summary content");

        // Verify summary format
        if let Message::User { ref content } = history[0] {
            let text = content.iter().next().unwrap();
            if let UserContent::Text(t) = text {
                assert!(t
                    .text
                    .contains("[Context Summary - Previous conversation has been compacted]"));
                assert!(t.text.contains("This is the summary content"));
                assert!(t.text.contains("[End of Summary]"));
                // Should also contain the last user message
                assert!(t.text.contains("The user's most recent request was:"));
                assert!(t.text.contains("Original message"));
            }
        }
    }

    #[test]
    fn test_apply_compaction_includes_last_user_message() {
        let mut history = vec![
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "First user message".to_string(),
                })),
            },
            Message::Assistant {
                id: None,
                content: OneOrMany::one(AssistantContent::Text(Text {
                    text: "Assistant response".to_string(),
                })),
            },
            Message::User {
                content: OneOrMany::one(UserContent::Text(Text {
                    text: "This is my latest request".to_string(),
                })),
            },
        ];

        apply_compaction(&mut history, "Summary of conversation");

        // Verify the compacted message includes both summary and last user message
        if let Message::User { ref content } = history[0] {
            let text = content.iter().next().unwrap();
            if let UserContent::Text(t) = text {
                assert!(t.text.contains("Summary of conversation"));
                assert!(t.text.contains("This is my latest request"));
                assert!(t.text.contains("The user's most recent request was:"));
            } else {
                panic!("Expected text content");
            }
        } else {
            panic!("Expected user message");
        }
    }
}
