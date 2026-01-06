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

use qbit_tools::ToolRegistry;

use super::tool_definitions::{
    get_all_tool_definitions_with_config, get_run_command_tool_definition,
    get_sub_agent_tool_definitions, get_tavily_tool_definitions, ToolConfig,
};
use super::tool_executors::{
    execute_indexer_tool, execute_plan_tool, execute_tavily_tool, execute_web_fetch_tool,
    normalize_run_pty_cmd_args,
};
use super::tool_provider_impl::DefaultToolProvider;
use qbit_context::token_budget::TokenUsage;
use qbit_context::ContextManager;
use qbit_core::events::AiEvent;
use qbit_core::hitl::{ApprovalDecision, RiskLevel};
use qbit_core::runtime::QbitRuntime;
use qbit_hitl::ApprovalRecorder;
use qbit_indexer::IndexerState;
use qbit_llm_providers::ModelCapabilities;
use qbit_loop_detection::{LoopDetectionResult, LoopDetector};
use qbit_sidecar::{CaptureContext, SidecarState};
use qbit_sub_agents::{
    execute_sub_agent, SubAgentContext, SubAgentExecutorContext, SubAgentRegistry, MAX_AGENT_DEPTH,
};
use qbit_tool_policy::{PolicyConstraintResult, ToolPolicy, ToolPolicyManager};
use qbit_web::tavily::TavilyState;

/// Maximum number of tool call iterations before stopping
pub const MAX_TOOL_ITERATIONS: usize = 100;

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
    pub tavily_state: Option<&'a Arc<TavilyState>>,
    #[cfg_attr(not(feature = "tauri"), allow(dead_code))]
    pub workspace: &'a Arc<RwLock<std::path::PathBuf>>,
    #[cfg_attr(not(feature = "tauri"), allow(dead_code))]
    pub client: &'a Arc<RwLock<qbit_llm_providers::LlmClient>>,
    pub approval_recorder: &'a Arc<ApprovalRecorder>,
    pub pending_approvals: &'a Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>>,
    pub tool_policy_manager: &'a Arc<ToolPolicyManager>,
    pub context_manager: &'a Arc<ContextManager>,
    pub loop_detector: &'a Arc<RwLock<LoopDetector>>,
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

/// Helper to emit an event to frontend
fn emit_to_frontend(ctx: &AgenticLoopContext<'_>, event: AiEvent) {
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

    // Send to frontend
    let _ = ctx.event_tx.send(event.clone());

    // Capture in sidecar if available (stateless - creates fresh context each time)
    if let Some(sidecar) = ctx.sidecar_state {
        let mut capture = CaptureContext::new(sidecar.clone());
        capture.process(&event);
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
pub async fn run_agentic_loop(
    model: &rig_anthropic_vertex::CompletionModel,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
) -> Result<(String, Vec<Message>, Option<TokenUsage>)> {
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

    // Check if this is a web search (Tavily) tool call
    if tool_name.starts_with("web_search") || tool_name == "web_extract" {
        let (value, success) = execute_tavily_tool(ctx.tavily_state, tool_name, tool_args).await;
        return Ok(ToolExecutionResult { value, success });
    }

    // Check if this is an update_plan tool call
    if tool_name == "update_plan" {
        let (value, success) = execute_plan_tool(ctx.plan_manager, ctx.event_tx, tool_args).await;
        return Ok(ToolExecutionResult { value, success });
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

        let sub_ctx = SubAgentExecutorContext {
            event_tx: ctx.event_tx,
            tavily_state: ctx.tavily_state,
            tool_registry: ctx.tool_registry,
            workspace: ctx.workspace,
            provider_name: ctx.provider_name,
            model_name: ctx.model_name,
        };

        let tool_provider = DefaultToolProvider::new();
        match execute_sub_agent(
            &agent_def,
            tool_args,
            context,
            model,
            sub_ctx,
            &tool_provider,
        )
        .await
        {
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

    // Step 0: Check agent mode for planning mode restrictions
    let agent_mode = *ctx.agent_mode.read().await;
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
    if ctx.tool_policy_manager.is_denied(tool_name).await {
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

        return execute_tool_direct_generic(tool_name, &effective_args, ctx, model, context).await;
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

        return execute_tool_direct_generic(tool_name, &effective_args, ctx, model, context).await;
    }

    // Step 4.4: Check if agent mode is auto-approve
    if agent_mode.is_auto_approve() {
        emit_event(
            ctx,
            AiEvent::ToolAutoApproved {
                request_id: tool_id.to_string(),
                tool_name: tool_name.to_string(),
                args: effective_args.clone(),
                reason: "Auto-approved via agent mode".to_string(),
                source: qbit_core::events::ToolSource::Main,
            },
        );

        return execute_tool_direct_generic(tool_name, &effective_args, ctx, model, context).await;
    }

    // Step 4.5: Check if runtime has auto-approve enabled (CLI --auto-approve flag)
    if let Some(runtime) = ctx.runtime {
        if runtime.auto_approve() {
            emit_event(
                ctx,
                AiEvent::ToolAutoApproved {
                    request_id: tool_id.to_string(),
                    tool_name: tool_name.to_string(),
                    args: effective_args.clone(),
                    reason: "Auto-approved via --auto-approve flag".to_string(),
                    source: qbit_core::events::ToolSource::Main,
                },
            );

            return execute_tool_direct_generic(tool_name, &effective_args, ctx, model, context)
                .await;
        }
    }

    // Step 5: Need approval - create request with stats
    let stats = ctx.approval_recorder.get_pattern(tool_name).await;
    let risk_level = RiskLevel::for_tool(tool_name);
    let config = ctx.approval_recorder.get_config().await;
    let can_learn = !config
        .always_require_approval
        .contains(&tool_name.to_string());
    let suggestion = ctx.approval_recorder.get_suggestion(tool_name).await;

    // Create oneshot channel for response
    let (tx, rx) = oneshot::channel::<ApprovalDecision>();

    // Store the sender
    {
        let mut pending = ctx.pending_approvals.write().await;
        pending.insert(tool_id.to_string(), tx);
    }

    // Emit approval request event with HITL metadata
    let _ = ctx.event_tx.send(AiEvent::ToolApprovalRequest {
        request_id: tool_id.to_string(),
        tool_name: tool_name.to_string(),
        args: effective_args.clone(),
        stats,
        risk_level,
        can_learn,
        suggestion,
        source: qbit_core::events::ToolSource::Main,
    });

    // Wait for approval response (with timeout)
    match tokio::time::timeout(std::time::Duration::from_secs(APPROVAL_TIMEOUT_SECS), rx).await {
        Ok(Ok(decision)) => {
            if decision.approved {
                let _ = ctx
                    .approval_recorder
                    .record_approval(tool_name, true, decision.reason, decision.always_allow)
                    .await;

                execute_tool_direct_generic(tool_name, &effective_args, ctx, model, context).await
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
            let mut pending = ctx.pending_approvals.write().await;
            pending.remove(tool_id);

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
/// Returns a tuple of (response_text, message_history, token_usage)
///
/// Note: This is the generic entry point that delegates to the unified loop.
/// Model capabilities are detected from the provider/model name in the context.
pub async fn run_agentic_loop_generic<M>(
    model: &M,
    system_prompt: &str,
    initial_history: Vec<Message>,
    context: SubAgentContext,
    ctx: &AgenticLoopContext<'_>,
) -> Result<(String, Vec<Message>, Option<TokenUsage>)>
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
) -> Result<(String, Vec<Message>, Option<TokenUsage>)>
where
    M: rig::completion::CompletionModel + Sync,
{
    let supports_thinking = config.capabilities.supports_thinking_history;

    tracing::info!(
        "run_agentic_loop_unified: capabilities={:?}, require_hitl={}, is_sub_agent={}, supports_thinking={}",
        config.capabilities,
        config.require_hitl,
        config.is_sub_agent,
        supports_thinking
    );

    // Reset loop detector for new turn
    {
        let mut detector = ctx.loop_detector.write().await;
        detector.reset();
    }

    // Create persistent capture context for file event correlation
    let mut capture_ctx = LoopCaptureContext::new(ctx.sidecar_state);

    // Get all available tools (filtered by config + web search)
    let mut tools = get_all_tool_definitions_with_config(ctx.tool_config);

    // Add run_command (wrapper for run_pty_cmd with better naming)
    tools.push(get_run_command_tool_definition());

    tracing::debug!(
        "Available tools (unified loop): {:?}",
        tools.iter().map(|t| t.name.clone()).collect::<Vec<_>>()
    );

    // Check if native web tools are available (Vertex AI Anthropic or OpenAI web search)
    let use_native_web_tools = {
        let client = ctx.client.read().await;
        client.supports_native_web_tools()
    };
    let use_openai_web_search = ctx.openai_web_search_config.is_some();

    if use_native_web_tools {
        tracing::info!("Using Claude's native web tools (web_search, web_fetch) - skipping Tavily");
    } else if use_openai_web_search {
        tracing::info!("Using OpenAI's native web search (web_search_preview) - skipping Tavily");
    } else {
        // Add Tavily web search tools if available and not disabled by config
        tools.extend(
            get_tavily_tool_definitions(ctx.tavily_state)
                .into_iter()
                .filter(|t| ctx.tool_config.is_tool_enabled(&t.name)),
        );
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

    // Enforce context window limits if needed (also checks for warnings)
    let enforcement_result = ctx
        .context_manager
        .enforce_context_window(&chat_history)
        .await;

    // Update chat history with potentially pruned messages
    chat_history = enforcement_result.messages;

    // Emit warning event if utilization exceeded warning threshold
    if let Some(warning_info) = enforcement_result.warning_info {
        tracing::info!(
            "Context warning (unified loop): {:.1}% utilization ({} / {} tokens)",
            warning_info.utilization * 100.0,
            warning_info.total_tokens,
            warning_info.max_tokens
        );
        let _ = ctx.event_tx.send(AiEvent::ContextWarning {
            utilization: warning_info.utilization,
            total_tokens: warning_info.total_tokens,
            max_tokens: warning_info.max_tokens,
        });
    }

    // Emit pruned event if messages were removed
    if let Some(pruned_info) = enforcement_result.pruned_info {
        tracing::info!(
            "Context pruned (unified loop): {} messages removed, utilization {:.1}% -> {:.1}%",
            pruned_info.messages_removed,
            pruned_info.utilization_before * 100.0,
            pruned_info.utilization_after * 100.0
        );
        // Update stats after pruning
        ctx.context_manager
            .update_from_messages(&chat_history)
            .await;
        let _ = ctx.event_tx.send(AiEvent::ContextPruned {
            messages_removed: pruned_info.messages_removed,
            utilization_before: pruned_info.utilization_before,
            utilization_after: pruned_info.utilization_after,
        });
    }

    let mut accumulated_response = String::new();
    // Thinking history tracking - only used when supports_thinking is true
    let mut accumulated_thinking = String::new();
    let mut total_usage = TokenUsage::default();
    let mut iteration = 0;

    loop {
        iteration += 1;
        if iteration > MAX_TOOL_ITERATIONS {
            let _ = ctx.event_tx.send(AiEvent::Error {
                message: "Maximum tool iterations reached".to_string(),
                error_type: "max_iterations".to_string(),
            });
            break;
        }

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

        // Build additional_params for OpenAI web search if enabled
        let additional_params = if let Some(web_config) = ctx.openai_web_search_config {
            tracing::info!(
                "Adding OpenAI web_search_preview tool with context_size={}",
                web_config.search_context_size
            );
            Some(json!({
                "tools": [web_config.to_tool_json()]
            }))
        } else {
            None
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

        // Make streaming completion request
        tracing::debug!(
            "[Unified] Starting streaming completion request (iteration {}, thinking={})",
            iteration,
            supports_thinking
        );
        let mut stream = model.stream(request).await.map_err(|e| {
            tracing::error!("Failed to start stream: {}", e);
            anyhow::anyhow!("{}", e)
        })?;
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
                            tracing::info!(
                                "Received tool call chunk #{}: {}",
                                chunk_count,
                                tool_call.function.name
                            );

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
                                tracing::info!(
                                    "Finalizing previous tool call: {} with args: {}",
                                    prev_name,
                                    current_tool_args
                                );
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
                                tracing::info!("Tool call has complete args, adding directly");
                                let mut tool_call = tool_call;
                                if tool_call.call_id.is_none() {
                                    tool_call.call_id = Some(tool_call.id.clone());
                                }
                                tool_calls_to_execute.push(tool_call);
                            } else {
                                // Tool call has empty args, wait for deltas
                                tracing::info!(
                                    "Tool call has empty args, tracking for delta accumulation"
                                );
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
                        StreamedAssistantContent::ToolCallDelta { id, delta } => {
                            // If we don't have a current tool ID but the delta has one, use it
                            if current_tool_id.is_none() && !id.is_empty() {
                                current_tool_id = Some(id);
                            }
                            // Accumulate tool call argument deltas
                            current_tool_args.push_str(&delta);
                        }
                        StreamedAssistantContent::Final(ref resp) => {
                            tracing::info!("Received final response chunk #{}", chunk_count);

                            // Extract and accumulate token usage
                            if let Some(usage) = resp.token_usage() {
                                total_usage.input_tokens += usage.input_tokens;
                                total_usage.output_tokens += usage.output_tokens;
                                tracing::debug!(
                                    "Token usage for iteration {}: input={}, output={}, total={}",
                                    iteration,
                                    usage.input_tokens,
                                    usage.output_tokens,
                                    total_usage.total()
                                );
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

        tracing::debug!(
            "Stream completed (unified): {} chunks, {} chars text, {} chars thinking, {} tool calls",
            chunk_count,
            text_content.len(),
            thinking_content.len(),
            tool_calls_to_execute.len()
        );

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

        // If no tool calls, we're done
        if !has_tool_calls {
            break;
        }

        // Build assistant content for history
        // IMPORTANT: When thinking is enabled, thinking blocks MUST come first (required by Anthropic API)
        let mut assistant_content: Vec<AssistantContent> = vec![];

        // Conditionally add thinking content first (required by Anthropic API when thinking is enabled)
        // For OpenAI Responses API, we must also include the reasoning ID (rs_...) that function calls reference.
        // CRITICAL: We must include reasoning when:
        // 1. There's thinking content (Anthropic extended thinking)
        // 2. OR there's a reasoning ID (OpenAI Responses API - even if content is empty)
        let has_reasoning = !thinking_content.is_empty() || thinking_id.is_some();
        if supports_thinking && has_reasoning {
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
        for tool_call in &tool_calls_to_execute {
            assistant_content.push(AssistantContent::ToolCall(tool_call.clone()));
        }

        chat_history.push(Message::Assistant {
            id: None,
            content: OneOrMany::many(assistant_content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
        });

        // Execute tool calls and collect results
        let mut tool_results: Vec<UserContent> = vec![];

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

            // Check for loop detection
            let loop_result = {
                let mut detector = ctx.loop_detector.write().await;
                detector.record_tool_call(tool_name, &tool_args)
            };

            // Handle loop detection (may add a blocked result)
            if let Some(blocked_result) =
                handle_loop_detection(&loop_result, &tool_id, &tool_call_id, ctx.event_tx)
            {
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
        }

        // Add tool results as user message
        chat_history.push(Message::User {
            content: OneOrMany::many(tool_results).unwrap_or_else(|_| {
                OneOrMany::one(UserContent::Text(Text {
                    text: "Tool executed".to_string(),
                }))
            }),
        });
    }

    // Log thinking stats at debug level
    if supports_thinking && !accumulated_thinking.is_empty() {
        tracing::debug!(
            "[Unified] Total thinking content: {} chars",
            accumulated_thinking.len()
        );
    }

    tracing::info!(
        "Turn complete - tokens: input={}, output={}, total={}",
        total_usage.input_tokens,
        total_usage.output_tokens,
        total_usage.total()
    );

    Ok((accumulated_response, chat_history, Some(total_usage)))
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
}
