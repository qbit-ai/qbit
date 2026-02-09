//! Sub-agent execution logic.
//!
//! This module handles the execution of sub-agents, which are specialized
//! agents that can be invoked by the main agent to handle specific tasks.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::StreamExt;
use rig::completion::request::ToolDefinition;
use rig::completion::{AssistantContent, CompletionModel as RigCompletionModel, Message};
use rig::message::{
    Reasoning, Text, ToolCall, ToolFunction, ToolResult, ToolResultContent, UserContent,
};
use rig::one_or_many::OneOrMany;
use rig::streaming::StreamedAssistantContent;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tracing::Instrument;
use uuid::Uuid;

use qbit_tools::ToolRegistry;
use qbit_udiff::{ApplyResult, UdiffApplier, UdiffParser};

use crate::definition::{SubAgentContext, SubAgentDefinition, SubAgentResult};
use crate::transcript::SubAgentTranscriptWriter;
use qbit_core::events::AiEvent;
use qbit_core::utils::truncate_str;
use qbit_core::ApiRequestStats;
use qbit_llm_providers::ModelCapabilities;

/// Trait for providing tool definitions to the sub-agent executor.
/// This allows the executor to be decoupled from the tool definition source.
#[async_trait::async_trait]
pub trait ToolProvider: Send + Sync {
    /// Get all available tool definitions
    fn get_all_tool_definitions(&self) -> Vec<ToolDefinition>;

    /// Filter tools to only those allowed by the sub-agent
    fn filter_tools_by_allowed(
        &self,
        tools: Vec<ToolDefinition>,
        allowed: &[String],
    ) -> Vec<ToolDefinition>;

    /// Execute a web fetch tool
    async fn execute_web_fetch_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> (serde_json::Value, bool);

    /// Normalize run_pty_cmd arguments
    fn normalize_run_pty_cmd_args(&self, args: serde_json::Value) -> serde_json::Value;
}

/// Context needed for sub-agent execution.
pub struct SubAgentExecutorContext<'a> {
    pub event_tx: &'a mpsc::UnboundedSender<AiEvent>,
    pub tool_registry: &'a Arc<RwLock<ToolRegistry>>,
    pub workspace: &'a Arc<RwLock<std::path::PathBuf>>,
    /// Provider name (e.g., "openai", "anthropic_vertex") for model capability checks
    pub provider_name: &'a str,
    /// Model name for model capability checks
    pub model_name: &'a str,
    /// Session ID for Langfuse tracing (propagated from parent agent)
    pub session_id: Option<&'a str>,
    /// Base directory for transcript files (e.g., `~/.qbit/transcripts`)
    /// If set, sub-agent internal events will be written to separate transcript files.
    pub transcript_base_dir: Option<&'a std::path::Path>,
    /// API request stats collector (per session, optional)
    pub api_request_stats: Option<&'a Arc<ApiRequestStats>>,
}

/// Execute a sub-agent with the given task and context.
///
/// # Arguments
/// * `agent_def` - The sub-agent definition
/// * `args` - Arguments containing the task and optional context
/// * `parent_context` - The context from the parent agent
/// * `model` - The LLM model to use for completion (any model implementing CompletionModel)
/// * `ctx` - Execution context with shared resources
/// * `tool_provider` - Provider for tool definitions and execution
/// * `parent_request_id` - The ID of the parent request that spawned this sub-agent
///
/// # Returns
/// The result of the sub-agent execution
pub async fn execute_sub_agent<M, P>(
    agent_def: &SubAgentDefinition,
    args: &serde_json::Value,
    parent_context: &SubAgentContext,
    model: &M,
    ctx: SubAgentExecutorContext<'_>,
    tool_provider: &P,
    parent_request_id: &str,
) -> Result<SubAgentResult>
where
    M: RigCompletionModel + Sync,
    P: ToolProvider,
{
    let start_time = std::time::Instant::now();
    let agent_id = &agent_def.id;

    // Create span for sub-agent execution (Langfuse observability)
    //
    // IMPORTANT: Explicitly parent this span to the current span so sub-agent work
    // is attached to the main trace even when crossing async/task boundaries.
    let sub_agent_span = tracing::info_span!(
        parent: &tracing::Span::current(),
        "sub_agent",
        "langfuse.observation.type" = "agent",
        "langfuse.session.id" = ctx.session_id.unwrap_or(""),
        "langfuse.observation.input" = tracing::field::Empty,
        "langfuse.observation.output" = tracing::field::Empty,
        agent_type = %format!("sub-agent:{}", agent_id),
        agent_id = %agent_id,
        model = %ctx.model_name,
        provider = %ctx.provider_name,
        depth = parent_context.depth + 1,
    );

    // Determine overall timeout
    let timeout_duration = agent_def
        .timeout_secs
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600));

    // Determine idle timeout
    let idle_timeout_duration = agent_def.idle_timeout_secs.map(Duration::from_secs);

    // Clone event_tx for timeout error handling (ctx is borrowed, not moved)
    let event_tx_clone = ctx.event_tx.clone();

    // Execute the sub-agent within the span, with overall timeout
    match tokio::time::timeout(
        timeout_duration,
        execute_sub_agent_inner(
            agent_def,
            args,
            parent_context,
            model,
            ctx,
            tool_provider,
            parent_request_id,
            start_time,
            &sub_agent_span,
            idle_timeout_duration,
        )
        .instrument(sub_agent_span.clone()),
    )
    .await
    {
        Ok(result) => result,
        Err(_elapsed) => {
            let duration_ms = start_time.elapsed().as_millis() as u64;
            let error_msg = format!(
                "Sub-agent '{}' timed out after {}s",
                agent_def.id,
                timeout_duration.as_secs()
            );
            tracing::warn!("{}", error_msg);

            let _ = event_tx_clone.send(AiEvent::SubAgentError {
                agent_id: agent_def.id.clone(),
                error: error_msg.clone(),
                parent_request_id: parent_request_id.to_string(),
            });

            Ok(SubAgentResult {
                agent_id: agent_def.id.clone(),
                response: format!("Error: {}", error_msg),
                context: SubAgentContext {
                    original_request: parent_context.original_request.clone(),
                    conversation_summary: parent_context.conversation_summary.clone(),
                    variables: parent_context.variables.clone(),
                    depth: parent_context.depth + 1,
                },
                success: false,
                duration_ms,
                files_modified: vec![],
            })
        }
    }
}

/// Inner implementation of sub-agent execution (instrumented by caller).
#[allow(clippy::too_many_arguments)]
async fn execute_sub_agent_inner<M, P>(
    agent_def: &SubAgentDefinition,
    args: &serde_json::Value,
    parent_context: &SubAgentContext,
    model: &M,
    ctx: SubAgentExecutorContext<'_>,
    tool_provider: &P,
    parent_request_id: &str,
    start_time: std::time::Instant,
    sub_agent_span: &tracing::Span,
    idle_timeout: Option<Duration>,
) -> Result<SubAgentResult>
where
    M: RigCompletionModel + Sync,
    P: ToolProvider,
{
    let agent_id = &agent_def.id;

    // Create transcript writer for sub-agent internal events if transcript_base_dir is set
    let transcript_writer: Option<Arc<SubAgentTranscriptWriter>> = if let (
        Some(base_dir),
        Some(session_id),
    ) =
        (ctx.transcript_base_dir, ctx.session_id)
    {
        match SubAgentTranscriptWriter::new(base_dir, session_id, agent_id, parent_request_id).await
        {
            Ok(writer) => Some(Arc::new(writer)),
            Err(e) => {
                tracing::warn!(
                        "Failed to create sub-agent transcript writer: {}. Continuing without transcript.",
                        e
                    );
                None
            }
        }
    } else {
        None
    };

    // Idle timeout tracking: stores epoch seconds of last activity
    let last_activity = Arc::new(AtomicU64::new(epoch_secs()));

    // Track files modified by this sub-agent
    let mut files_modified: Vec<String> = vec![];

    // Extract task and additional context from args
    let task = args
        .get("task")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Sub-agent call missing 'task' parameter"))?;
    let additional_context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");

    // Build the sub-agent context with incremented depth
    let sub_context = SubAgentContext {
        original_request: parent_context.original_request.clone(),
        conversation_summary: parent_context.conversation_summary.clone(),
        variables: parent_context.variables.clone(),
        depth: parent_context.depth + 1,
    };

    // Build the prompt for the sub-agent
    let sub_prompt = if additional_context.is_empty() {
        task.to_string()
    } else {
        format!("{}\n\nAdditional context: {}", task, additional_context)
    };

    // Record input on the sub-agent span (truncated for Langfuse, use truncate_str for UTF-8 safety)
    let input_truncated = if sub_prompt.len() > 1000 {
        format!("{}...[truncated]", truncate_str(&sub_prompt, 1000))
    } else {
        sub_prompt.clone()
    };
    sub_agent_span.record("langfuse.observation.input", &input_truncated);

    // Emit sub-agent start event
    let _ = ctx.event_tx.send(AiEvent::SubAgentStarted {
        agent_id: agent_id.to_string(),
        agent_name: agent_def.name.clone(),
        task: task.to_string(),
        depth: sub_context.depth,
        parent_request_id: parent_request_id.to_string(),
    });

    // Build filtered tools based on agent's allowed tools
    let all_tools = tool_provider.get_all_tool_definitions();
    let tools = tool_provider.filter_tools_by_allowed(all_tools, &agent_def.allowed_tools);

    // Build chat history for sub-agent
    let mut chat_history: Vec<Message> = vec![Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: sub_prompt.clone(),
        })),
    }];

    let mut accumulated_response = String::new();
    let mut iteration = 0;

    loop {
        iteration += 1;
        if iteration > agent_def.max_iterations {
            tracing::info!(
                "[sub-agent] Max iterations ({}) reached, making final toolless call for summary",
                agent_def.max_iterations
            );

            // Make one final LLM call with no tools to force a text summary response
            let caps = ModelCapabilities::detect(ctx.provider_name, ctx.model_name);
            let temperature = if caps.supports_temperature {
                Some(0.3)
            } else {
                None
            };

            let final_request = rig::completion::CompletionRequest {
                preamble: Some(agent_def.system_prompt.clone()),
                chat_history: OneOrMany::many(chat_history.clone())
                    .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
                documents: vec![],
                tools: vec![],
                temperature,
                max_tokens: Some(8192),
                tool_choice: None,
                additional_params: None,
            };

            if let Some(stats) = ctx.api_request_stats {
                stats.record_sent(ctx.provider_name).await;
            }

            match model.stream(final_request).await {
                Ok(mut final_stream) => {
                    if let Some(stats) = ctx.api_request_stats {
                        stats.record_received(ctx.provider_name).await;
                    }
                    while let Some(chunk_result) = final_stream.next().await {
                        if let Ok(StreamedAssistantContent::Text(text_msg)) = chunk_result {
                            accumulated_response.push_str(&text_msg.text);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "[sub-agent] Final summary call failed: {}, returning accumulated response",
                        e
                    );
                }
            }

            break;
        }

        // Build request with sub-agent's system prompt
        // Conditionally set temperature based on model support (e.g., OpenAI o1/o3 models don't support it)
        let caps = ModelCapabilities::detect(ctx.provider_name, ctx.model_name);
        let temperature = if caps.supports_temperature {
            Some(0.3)
        } else {
            tracing::debug!(
                "Model {} does not support temperature parameter in sub-agent, omitting",
                ctx.model_name
            );
            None
        };

        let request = rig::completion::CompletionRequest {
            preamble: Some(agent_def.system_prompt.clone()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: tools.clone(),
            temperature,
            max_tokens: Some(8192),
            tool_choice: None,
            additional_params: None,
        };

        // Create LLM completion span for this iteration (Langfuse observability)
        // Explicit parent ensures this appears nested under sub_agent_span in Langfuse
        let llm_span = tracing::info_span!(
            parent: sub_agent_span,
            "llm_completion",
            "gen_ai.operation.name" = "chat_completion",
            "gen_ai.request.model" = %ctx.model_name,
            "gen_ai.system" = %ctx.provider_name,
            "gen_ai.usage.prompt_tokens" = tracing::field::Empty,
            "gen_ai.usage.completion_tokens" = tracing::field::Empty,
            "langfuse.observation.type" = "generation",
            "langfuse.session.id" = ctx.session_id.unwrap_or(""),
            iteration = iteration,
        );
        let _llm_guard = llm_span.enter();

        // Make streaming completion request (streaming works better with Z.AI for tool calls)
        if let Some(stats) = ctx.api_request_stats {
            stats.record_sent(ctx.provider_name).await;
        }

        let mut stream = match model.stream(request).await {
            Ok(s) => {
                if let Some(stats) = ctx.api_request_stats {
                    stats.record_received(ctx.provider_name).await;
                }
                s
            }
            Err(e) => {
                let _ = ctx.event_tx.send(AiEvent::SubAgentError {
                    agent_id: agent_id.to_string(),
                    error: e.to_string(),
                    parent_request_id: parent_request_id.to_string(),
                });
                return Ok(SubAgentResult {
                    agent_id: agent_id.to_string(),
                    response: format!("Error: {}", e),
                    context: sub_context,
                    success: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    files_modified: files_modified.clone(),
                });
            }
        };

        // Check if model supports thinking history (for proper conversation history)
        let caps = ModelCapabilities::detect(ctx.provider_name, ctx.model_name);
        let supports_thinking_history = caps.supports_thinking_history;

        // Process streaming response
        let mut has_tool_calls = false;
        let mut tool_calls_to_execute: Vec<ToolCall> = vec![];
        let mut text_content = String::new();
        let mut thinking_text = String::new();
        let mut thinking_signature: Option<String> = None;
        let mut thinking_id: Option<String> = None;

        // Track tool call state for streaming (tool args come as deltas)
        let mut current_tool_id: Option<String> = None;
        let mut current_tool_name: Option<String> = None;
        let mut current_tool_args = String::new();

        // Update activity before stream processing
        last_activity.store(epoch_secs(), Ordering::Relaxed);

        // Stream processing loop with idle timeout check
        let mut idle_timeout_hit = false;
        loop {
            let chunk_opt = if let Some(idle_dur) = idle_timeout {
                let last = last_activity.load(Ordering::Relaxed);
                let now = epoch_secs();
                let remaining = idle_dur.as_secs().saturating_sub(now.saturating_sub(last));

                if remaining == 0 {
                    idle_timeout_hit = true;
                    break;
                }

                match tokio::time::timeout(Duration::from_secs(remaining), stream.next()).await {
                    Ok(v) => v,
                    Err(_) => {
                        idle_timeout_hit = true;
                        break;
                    }
                }
            } else {
                stream.next().await
            };

            let Some(chunk_result) = chunk_opt else {
                break; // Stream ended normally
            };

            // Update activity on chunk received
            last_activity.store(epoch_secs(), Ordering::Relaxed);

            match chunk_result {
                Ok(chunk) => match chunk {
                    StreamedAssistantContent::Text(text_msg) => {
                        text_content.push_str(&text_msg.text);
                    }
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        let reasoning_str = reasoning.reasoning.join("");
                        if !reasoning_str.is_empty() {
                            tracing::debug!("[sub-agent] Thinking: {} chars", reasoning_str.len());
                            thinking_text.push_str(&reasoning_str);
                        }
                        // Capture signature and id for history (required by Anthropic API)
                        if reasoning.signature.is_some() && thinking_signature.is_none() {
                            thinking_signature = reasoning.signature.clone();
                        }
                        if reasoning.id.is_some() && thinking_id.is_none() {
                            thinking_id = reasoning.id.clone();
                        }
                    }
                    StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                        if !reasoning.is_empty() {
                            thinking_text.push_str(&reasoning);
                        }
                        // Capture id from delta (OpenAI Responses API sends id in deltas)
                        if id.is_some() && thinking_id.is_none() {
                            thinking_id = id;
                        }
                    }
                    StreamedAssistantContent::ToolCall(tool_call) => {
                        tracing::debug!(
                            "[sub-agent] Received tool call: {} (id: {})",
                            tool_call.function.name,
                            tool_call.id
                        );

                        // Finalize any previous pending tool call first
                        if let (Some(prev_id), Some(prev_name)) =
                            (current_tool_id.take(), current_tool_name.take())
                        {
                            let args = qbit_json_repair::parse_tool_args(&current_tool_args);
                            tracing::debug!(
                                "[sub-agent] Finalizing previous tool call: {} with args: {}",
                                prev_name,
                                current_tool_args
                            );
                            has_tool_calls = true;
                            tool_calls_to_execute.push(ToolCall {
                                id: prev_id.clone(),
                                call_id: Some(prev_id),
                                function: ToolFunction {
                                    name: prev_name,
                                    arguments: args,
                                },
                                signature: None,
                                additional_params: None,
                            });
                            current_tool_args.clear();
                        }

                        // Check if this tool call has complete args (non-streaming case)
                        let has_complete_args = !tool_call.function.arguments.is_null()
                            && tool_call.function.arguments != serde_json::json!({});

                        if has_complete_args {
                            // Tool call came complete, add directly
                            tracing::debug!(
                                "[sub-agent] Tool call has complete args: {:?}",
                                tool_call.function.arguments
                            );
                            has_tool_calls = true;
                            let mut tc = tool_call;
                            if tc.call_id.is_none() {
                                tc.call_id = Some(tc.id.clone());
                            }
                            tool_calls_to_execute.push(tc);
                        } else {
                            // Tool call has empty args, wait for deltas
                            tracing::debug!(
                                "[sub-agent] Tool call has empty args, tracking for delta accumulation"
                            );
                            current_tool_id = Some(tool_call.id.clone());
                            current_tool_name = Some(tool_call.function.name.clone());
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
                    _ => {
                        // Ignore other stream content types
                    }
                },
                Err(e) => {
                    tracing::warn!("[sub-agent] Stream error: {}", e);
                }
            }
        }

        // Check if we exited the streaming loop due to idle timeout
        if idle_timeout_hit {
            if let Some(idle_dur) = idle_timeout {
                let error_msg = format!(
                    "Sub-agent idle timeout: no activity for {}s",
                    idle_dur.as_secs()
                );
                tracing::warn!("[sub-agent] {}", error_msg);

                let _ = ctx.event_tx.send(AiEvent::SubAgentError {
                    agent_id: agent_id.to_string(),
                    error: error_msg.clone(),
                    parent_request_id: parent_request_id.to_string(),
                });

                return Ok(SubAgentResult {
                    agent_id: agent_id.to_string(),
                    response: format!("Error: {}", error_msg),
                    context: sub_context.clone(),
                    success: false,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    files_modified: files_modified.clone(),
                });
            }
        }

        // Finalize any remaining pending tool call after stream ends
        if let (Some(prev_id), Some(prev_name)) = (current_tool_id.take(), current_tool_name.take())
        {
            let args = qbit_json_repair::parse_tool_args(&current_tool_args);
            tracing::debug!(
                "[sub-agent] Finalizing final tool call: {} with args: {}",
                prev_name,
                current_tool_args
            );
            has_tool_calls = true;
            tool_calls_to_execute.push(ToolCall {
                id: prev_id.clone(),
                call_id: Some(prev_id),
                function: ToolFunction {
                    name: prev_name,
                    arguments: args,
                },
                signature: None,
                additional_params: None,
            });
        }

        // Note: Token usage tracking requires stream metadata which may not be available
        // in all streaming implementations. Skip recording for now.

        if !text_content.is_empty() {
            accumulated_response.push_str(&text_content);
        }

        if !has_tool_calls {
            break;
        }

        // Build assistant content for chat history using helper function
        // (ensures correct ordering: Reasoning -> Text -> ToolCalls)
        let assistant_content = build_assistant_content(
            supports_thinking_history,
            &thinking_text,
            thinking_id.clone(),
            thinking_signature.clone(),
            &text_content,
            &tool_calls_to_execute,
        );

        chat_history.push(Message::Assistant {
            id: None,
            content: OneOrMany::many(assistant_content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
        });

        // Execute tool calls
        let mut tool_results: Vec<UserContent> = vec![];

        for tool_call in tool_calls_to_execute {
            let tool_name = &tool_call.function.name;
            let tool_args = if tool_name == "run_pty_cmd" {
                tool_provider.normalize_run_pty_cmd_args(tool_call.function.arguments.clone())
            } else {
                tool_call.function.arguments.clone()
            };
            // For OpenAI Responses API, the actual call ID is in call_id field.
            // For Chat Completions API, call_id is None and we use id.
            let tool_id = tool_call.id.clone();
            let tool_call_id = tool_call
                .call_id
                .clone()
                .unwrap_or_else(|| tool_call.id.clone());

            // Emit tool request event
            let request_id = Uuid::new_v4().to_string();
            let tool_request_event = AiEvent::SubAgentToolRequest {
                agent_id: agent_id.to_string(),
                tool_name: tool_name.to_string(),
                args: tool_args.clone(),
                request_id: request_id.clone(),
                parent_request_id: parent_request_id.to_string(),
            };
            let _ = ctx.event_tx.send(tool_request_event.clone());

            // Write to sub-agent transcript (internal events go to separate file)
            if let Some(ref writer) = transcript_writer {
                let writer = Arc::clone(writer);
                let event = tool_request_event;
                tokio::spawn(async move {
                    if let Err(e) = writer.append(&event).await {
                        tracing::warn!("Failed to write to sub-agent transcript: {}", e);
                    }
                });
            }

            // Create tool call span (Langfuse observability)
            let args_for_span =
                serde_json::to_string(&tool_args).unwrap_or_else(|_| "{}".to_string());
            let args_truncated = if args_for_span.len() > 500 {
                format!("{}...[truncated]", &args_for_span[..500])
            } else {
                args_for_span
            };
            // Explicit parent ensures this appears nested under llm_span in Langfuse
            let tool_span = tracing::info_span!(
                parent: &llm_span,
                "tool_call",
                "otel.name" = %tool_name,
                "langfuse.span.name" = %tool_name,
                "langfuse.observation.type" = "tool",
                "langfuse.session.id" = ctx.session_id.unwrap_or(""),
                tool.name = %tool_name,
                tool.id = %tool_id,
                "langfuse.observation.input" = %args_truncated,
                "langfuse.observation.output" = tracing::field::Empty,
                success = tracing::field::Empty,
            );
            let _tool_guard = tool_span.enter();

            // Execute the tool
            let (result_value, success) = if tool_name == "web_fetch" {
                tool_provider
                    .execute_web_fetch_tool(tool_name, &tool_args)
                    .await
            } else {
                // All other tools (including web_* tools) use the registry
                let mut registry = ctx.tool_registry.write().await;
                let result = registry.execute_tool(tool_name, tool_args.clone()).await;

                match &result {
                    Ok(v) => (v.clone(), true),
                    Err(e) => (serde_json::json!({ "error": e.to_string() }), false),
                }
            };

            // Record tool result on span
            let result_str = serde_json::to_string(&result_value).unwrap_or_default();
            let result_truncated = if result_str.len() > 500 {
                format!("{}...[truncated]", &result_str[..500])
            } else {
                result_str
            };
            tool_span.record("langfuse.observation.output", &result_truncated);
            tool_span.record("success", success);

            // Emit tool result event
            let tool_result_event = AiEvent::SubAgentToolResult {
                agent_id: agent_id.to_string(),
                tool_name: tool_name.to_string(),
                success,
                result: result_value.clone(),
                request_id: request_id.clone(),
                parent_request_id: parent_request_id.to_string(),
            };
            let _ = ctx.event_tx.send(tool_result_event.clone());

            // Update idle timeout activity tracker after tool execution
            last_activity.store(epoch_secs(), Ordering::Relaxed);

            // Write to sub-agent transcript (internal events go to separate file)
            if let Some(ref writer) = transcript_writer {
                let writer = Arc::clone(writer);
                let event = tool_result_event;
                tokio::spawn(async move {
                    if let Err(e) = writer.append(&event).await {
                        tracing::warn!("Failed to write to sub-agent transcript: {}", e);
                    }
                });
            }

            // Track files modified by write tools
            if success && is_write_tool(tool_name) {
                if let Some(file_path) = extract_file_path(tool_name, &tool_args) {
                    if !files_modified.contains(&file_path) {
                        tracing::debug!(
                            "[sub-agent] Tracking modified file: {} (tool: {})",
                            file_path,
                            tool_name
                        );
                        files_modified.push(file_path);
                    }
                }
            }

            let result_text = serde_json::to_string(&result_value).unwrap_or_default();
            tool_results.push(UserContent::ToolResult(ToolResult {
                id: tool_id,
                call_id: Some(tool_call_id),
                content: OneOrMany::one(ToolResultContent::Text(Text { text: result_text })),
            }));
        }

        chat_history.push(Message::User {
            content: OneOrMany::many(tool_results).unwrap_or_else(|_| {
                OneOrMany::one(UserContent::Text(Text {
                    text: "Tool executed".to_string(),
                }))
            }),
        });
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Process udiff output if this is the coder sub-agent
    let mut final_response = accumulated_response.clone();
    if agent_def.id == "coder" {
        let workspace = ctx.workspace.read().await;
        let diffs = UdiffParser::parse(&accumulated_response);

        if !diffs.is_empty() {
            let mut applied_files = Vec::new();
            let mut errors = Vec::new();

            for diff in diffs {
                let file_path = workspace.join(&diff.file_path);

                // Handle new file creation
                if diff.is_new_file {
                    // Create parent directories if needed
                    if let Some(parent) = file_path.parent() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            errors.push(format!(
                                "Failed to create directories for {}: {}",
                                diff.file_path.display(),
                                e
                            ));
                            continue;
                        }
                    }

                    // Collect all new_lines from hunks to form the file content
                    let new_content: String = diff
                        .hunks
                        .iter()
                        .flat_map(|h| h.new_lines.iter())
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");

                    if let Err(e) = std::fs::write(&file_path, &new_content) {
                        errors.push(format!(
                            "Failed to create {}: {}",
                            diff.file_path.display(),
                            e
                        ));
                    } else {
                        let path_str = diff.file_path.display().to_string();
                        applied_files.push(path_str.clone());
                        if !files_modified.contains(&path_str) {
                            files_modified.push(path_str);
                        }
                        tracing::info!("[coder] Created new file: {}", diff.file_path.display());
                    }
                    continue;
                }

                // Handle existing file modification
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => {
                        match UdiffApplier::apply_hunks(&content, &diff.hunks) {
                            ApplyResult::Success { new_content } => {
                                if let Err(e) = std::fs::write(&file_path, new_content) {
                                    errors.push(format!(
                                        "Failed to write {}: {}",
                                        diff.file_path.display(),
                                        e
                                    ));
                                } else {
                                    let path_str = diff.file_path.display().to_string();
                                    applied_files.push(path_str.clone());
                                    if !files_modified.contains(&path_str) {
                                        files_modified.push(path_str);
                                    }
                                }
                            }
                            ApplyResult::PartialSuccess {
                                new_content,
                                applied,
                                failed,
                            } => {
                                // Clone failed before it's moved
                                let failed_hunks = failed.clone();
                                if let Err(e) = std::fs::write(&file_path, new_content) {
                                    errors.push(format!(
                                        "Failed to write {}: {}",
                                        diff.file_path.display(),
                                        e
                                    ));
                                } else {
                                    let path_str = diff.file_path.display().to_string();
                                    applied_files.push(path_str.clone());
                                    if !files_modified.contains(&path_str) {
                                        files_modified.push(path_str);
                                    }
                                    for (idx, reason) in failed {
                                        errors.push(format!(
                                            "Hunk {} in {}: {}",
                                            idx,
                                            diff.file_path.display(),
                                            reason
                                        ));
                                    }
                                }
                                tracing::info!(
                                    "[coder] Partial success: applied hunks {:?}, failed: {:?}",
                                    applied,
                                    failed_hunks
                                );
                            }
                            ApplyResult::NoMatch {
                                hunk_idx,
                                suggestion,
                            } => {
                                errors.push(format!(
                                    "{} (hunk {}): {}",
                                    diff.file_path.display(),
                                    hunk_idx,
                                    suggestion
                                ));
                            }
                            ApplyResult::MultipleMatches { hunk_idx, count } => {
                                errors.push(format!(
                                    "{} (hunk {}): Found {} matches, add more context",
                                    diff.file_path.display(),
                                    hunk_idx,
                                    count
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Cannot read {}: {}", diff.file_path.display(), e));
                    }
                }
            }

            // Append result summary to response
            if !applied_files.is_empty() || !errors.is_empty() {
                final_response.push_str("\n\n---\n**Applied Changes:**\n");

                if !applied_files.is_empty() {
                    final_response.push_str(&format!(
                        "\nSuccessfully modified {} file(s):\n",
                        applied_files.len()
                    ));
                    for file in &applied_files {
                        final_response.push_str(&format!("- {}\n", file));
                    }
                }

                if !errors.is_empty() {
                    final_response.push_str(&format!("\n{} error(s) occurred:\n", errors.len()));
                    for error in &errors {
                        final_response.push_str(&format!("- {}\n", error));
                    }
                }
            }
        }
    }

    let _ = ctx.event_tx.send(AiEvent::SubAgentCompleted {
        agent_id: agent_id.to_string(),
        response: final_response.clone(),
        duration_ms,
        parent_request_id: parent_request_id.to_string(),
    });

    if !files_modified.is_empty() {
        tracing::info!(
            "[sub-agent] {} modified {} files: {:?}",
            agent_id,
            files_modified.len(),
            files_modified
        );
    }

    // Record output on the sub-agent span (truncated for Langfuse, use truncate_str for UTF-8 safety)
    let output_truncated = if final_response.len() > 1000 {
        format!("{}...[truncated]", truncate_str(&final_response, 1000))
    } else {
        final_response.clone()
    };
    sub_agent_span.record("langfuse.observation.output", &output_truncated);

    Ok(SubAgentResult {
        agent_id: agent_id.to_string(),
        response: final_response,
        context: sub_context,
        success: true,
        duration_ms,
        files_modified,
    })
}

/// Get current time as epoch seconds (for idle timeout tracking).
fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Check if a tool modifies files
fn is_write_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write_file"
            | "create_file"
            | "edit_file"
            | "delete_file"
            | "delete_path"
            | "rename_file"
            | "move_file"
            | "move_path"
            | "copy_path"
            | "create_directory"
            | "apply_patch"
            | "ast_grep_replace"
    )
}

/// Extract file path from tool arguments
fn extract_file_path(tool_name: &str, args: &serde_json::Value) -> Option<String> {
    match tool_name {
        "write_file" | "create_file" | "edit_file" | "read_file" | "delete_file" => args
            .get("path")
            .or_else(|| args.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "apply_patch" => {
            // Extract file paths from patch content
            args.get("patch")
                .and_then(|v| v.as_str())
                .and_then(|patch| {
                    // Look for "*** Update File:" or "*** Add File:" lines
                    for line in patch.lines() {
                        if let Some(path) = line.strip_prefix("*** Update File:") {
                            return Some(path.trim().to_string());
                        }
                        if let Some(path) = line.strip_prefix("*** Add File:") {
                            return Some(path.trim().to_string());
                        }
                    }
                    None
                })
        }
        "rename_file" | "move_file" | "move_path" | "copy_path" => args
            .get("destination")
            .or_else(|| args.get("to"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "delete_path" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "create_directory" => args
            .get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// Build assistant content for chat history with proper ordering.
///
/// When thinking is enabled, thinking blocks MUST come first (required by Anthropic API).
/// This function ensures the correct order: Reasoning -> Text -> ToolCalls
///
/// # Arguments
/// * `supports_thinking_history` - Whether the model supports thinking history
/// * `thinking_text` - The accumulated thinking/reasoning text
/// * `thinking_id` - Optional reasoning ID (used by OpenAI Responses API)
/// * `thinking_signature` - Optional thinking signature (used by Anthropic)
/// * `text_content` - The text response content
/// * `tool_calls` - List of tool calls to include
///
/// # Returns
/// A vector of AssistantContent in the correct order for the API
pub fn build_assistant_content(
    supports_thinking_history: bool,
    thinking_text: &str,
    thinking_id: Option<String>,
    thinking_signature: Option<String>,
    text_content: &str,
    tool_calls: &[ToolCall],
) -> Vec<AssistantContent> {
    let mut content: Vec<AssistantContent> = vec![];

    // Add thinking content FIRST (required by Anthropic API when thinking is enabled)
    let has_reasoning = !thinking_text.is_empty() || thinking_id.is_some();
    if supports_thinking_history && has_reasoning {
        content.push(AssistantContent::Reasoning(
            Reasoning::multi(vec![thinking_text.to_string()])
                .optional_id(thinking_id)
                .with_signature(thinking_signature),
        ));
    }

    // Add text content
    if !text_content.is_empty() {
        content.push(AssistantContent::Text(Text {
            text: text_content.to_string(),
        }));
    }

    // Add tool calls
    for tc in tool_calls {
        content.push(AssistantContent::ToolCall(tc.clone()));
    }

    content
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool_call(id: &str, name: &str) -> ToolCall {
        ToolCall {
            id: id.to_string(),
            call_id: Some(id.to_string()),
            function: ToolFunction {
                name: name.to_string(),
                arguments: serde_json::json!({}),
            },
            signature: None,
            additional_params: None,
        }
    }

    #[test]
    fn test_build_assistant_content_with_thinking_and_tools() {
        // When thinking is supported and present, it should come FIRST
        let tool_calls = vec![make_tool_call("tc_1", "read_file")];
        let content = build_assistant_content(
            true,                        // supports_thinking_history
            "Let me analyze this...",    // thinking_text
            Some("thinking_123".into()), // thinking_id
            Some("sig_abc".into()),      // thinking_signature
            "I'll read the file.",       // text_content
            &tool_calls,
        );

        assert_eq!(content.len(), 3);

        // First element should be Reasoning
        assert!(
            matches!(&content[0], AssistantContent::Reasoning(_)),
            "First content should be Reasoning, got {:?}",
            content[0]
        );

        // Second element should be Text
        assert!(
            matches!(&content[1], AssistantContent::Text(_)),
            "Second content should be Text"
        );

        // Third element should be ToolCall
        assert!(
            matches!(&content[2], AssistantContent::ToolCall(_)),
            "Third content should be ToolCall"
        );
    }

    #[test]
    fn test_build_assistant_content_thinking_id_only() {
        // OpenAI Responses API may have reasoning ID but empty content
        let tool_calls = vec![make_tool_call("tc_1", "read_file")];
        let content = build_assistant_content(
            true,                  // supports_thinking_history
            "",                    // thinking_text (empty)
            Some("rs_123".into()), // thinking_id (present)
            None,                  // thinking_signature
            "",                    // text_content
            &tool_calls,
        );

        assert_eq!(content.len(), 2);

        // First element should be Reasoning (even with empty content, ID triggers inclusion)
        assert!(
            matches!(&content[0], AssistantContent::Reasoning(_)),
            "First content should be Reasoning when thinking_id is present"
        );

        // Second element should be ToolCall
        assert!(
            matches!(&content[1], AssistantContent::ToolCall(_)),
            "Second content should be ToolCall"
        );
    }

    #[test]
    fn test_build_assistant_content_no_thinking_support() {
        // When model doesn't support thinking history, no Reasoning should be added
        let tool_calls = vec![make_tool_call("tc_1", "read_file")];
        let content = build_assistant_content(
            false,                       // supports_thinking_history = false
            "Some thinking content",     // thinking_text (ignored)
            Some("thinking_123".into()), // thinking_id (ignored)
            Some("sig_abc".into()),      // thinking_signature (ignored)
            "I'll read the file.",       // text_content
            &tool_calls,
        );

        assert_eq!(content.len(), 2);

        // First element should be Text (no Reasoning)
        assert!(
            matches!(&content[0], AssistantContent::Text(_)),
            "First content should be Text when thinking not supported"
        );

        // Second element should be ToolCall
        assert!(
            matches!(&content[1], AssistantContent::ToolCall(_)),
            "Second content should be ToolCall"
        );
    }

    #[test]
    fn test_build_assistant_content_no_thinking_content() {
        // When there's no thinking content and no ID, no Reasoning should be added
        let tool_calls = vec![make_tool_call("tc_1", "read_file")];
        let content = build_assistant_content(
            true, // supports_thinking_history
            "",   // thinking_text (empty)
            None, // thinking_id (none)
            None, // thinking_signature
            "Response text",
            &tool_calls,
        );

        assert_eq!(content.len(), 2);

        // First element should be Text (no Reasoning since both text and id are empty)
        assert!(
            matches!(&content[0], AssistantContent::Text(_)),
            "First content should be Text when no thinking content"
        );
    }

    #[test]
    fn test_build_assistant_content_tools_only() {
        // Tool calls only, no text or thinking
        let tool_calls = vec![
            make_tool_call("tc_1", "read_file"),
            make_tool_call("tc_2", "write_file"),
        ];
        let content = build_assistant_content(true, "", None, None, "", &tool_calls);

        assert_eq!(content.len(), 2);
        assert!(matches!(&content[0], AssistantContent::ToolCall(_)));
        assert!(matches!(&content[1], AssistantContent::ToolCall(_)));
    }

    #[test]
    fn test_build_assistant_content_empty() {
        // Edge case: no content at all
        let content = build_assistant_content(true, "", None, None, "", &[]);

        assert!(content.is_empty());
    }

    #[test]
    fn test_build_assistant_content_thinking_with_signature() {
        // Verify signature is included when provided
        let content = build_assistant_content(
            true,
            "Thinking...",
            None,
            Some("signature_xyz".into()),
            "",
            &[],
        );

        assert_eq!(content.len(), 1);
        if let AssistantContent::Reasoning(reasoning) = &content[0] {
            assert_eq!(reasoning.signature, Some("signature_xyz".to_string()));
        } else {
            panic!("Expected Reasoning content");
        }
    }

    #[test]
    fn test_anthropic_vertex_model_capabilities() {
        // Verify Anthropic/Vertex models support thinking history
        // Multiple provider name aliases are supported for compatibility
        let caps = ModelCapabilities::detect("anthropic_vertex", "claude-sonnet-4-20250514");
        assert!(
            caps.supports_thinking_history,
            "anthropic_vertex should support thinking history"
        );

        let caps = ModelCapabilities::detect("vertex_ai", "claude-sonnet-4-20250514");
        assert!(
            caps.supports_thinking_history,
            "vertex_ai should support thinking history"
        );

        let caps = ModelCapabilities::detect("vertex_ai_anthropic", "claude-3-5-sonnet");
        assert!(
            caps.supports_thinking_history,
            "vertex_ai_anthropic should support thinking history"
        );

        let caps = ModelCapabilities::detect("anthropic", "claude-3-opus");
        assert!(
            caps.supports_thinking_history,
            "anthropic should support thinking history"
        );
    }

    #[test]
    fn test_non_thinking_model_capabilities() {
        // Verify models that don't support thinking are detected correctly
        let caps = ModelCapabilities::detect("groq", "llama-3.3-70b");
        assert!(
            !caps.supports_thinking_history,
            "Groq should not support thinking history"
        );

        let caps = ModelCapabilities::detect("ollama", "llama3.2");
        assert!(
            !caps.supports_thinking_history,
            "Ollama should not support thinking history"
        );
    }

    #[test]
    fn test_build_assistant_content_text_only() {
        // Text only, no thinking, no tools
        let content = build_assistant_content(true, "", None, None, "Just a text response", &[]);

        assert_eq!(content.len(), 1);
        if let AssistantContent::Text(text) = &content[0] {
            assert_eq!(text.text, "Just a text response");
        } else {
            panic!("Expected Text content");
        }
    }

    #[test]
    fn test_build_assistant_content_verifies_values() {
        // Verify actual content values, not just types
        let tool_calls = vec![make_tool_call("tc_123", "read_file")];
        let content = build_assistant_content(
            true,
            "My thinking process",
            Some("id_456".into()),
            Some("sig_789".into()),
            "My response text",
            &tool_calls,
        );

        assert_eq!(content.len(), 3);

        // Verify Reasoning content
        if let AssistantContent::Reasoning(reasoning) = &content[0] {
            assert_eq!(reasoning.reasoning, vec!["My thinking process"]);
            assert_eq!(reasoning.id, Some("id_456".to_string()));
            assert_eq!(reasoning.signature, Some("sig_789".to_string()));
        } else {
            panic!("Expected Reasoning content at index 0");
        }

        // Verify Text content
        if let AssistantContent::Text(text) = &content[1] {
            assert_eq!(text.text, "My response text");
        } else {
            panic!("Expected Text content at index 1");
        }

        // Verify ToolCall content
        if let AssistantContent::ToolCall(tc) = &content[2] {
            assert_eq!(tc.id, "tc_123");
            assert_eq!(tc.function.name, "read_file");
        } else {
            panic!("Expected ToolCall content at index 2");
        }
    }

    #[test]
    fn test_build_assistant_content_multiple_tools_preserve_order() {
        // Multiple tool calls should preserve their order
        let tool_calls = vec![
            make_tool_call("tc_1", "read_file"),
            make_tool_call("tc_2", "write_file"),
            make_tool_call("tc_3", "list_dir"),
        ];
        let content = build_assistant_content(
            false, // no thinking
            "",
            None,
            None,
            "",
            &tool_calls,
        );

        assert_eq!(content.len(), 3);

        // Verify order is preserved
        let names: Vec<&str> = content
            .iter()
            .filter_map(|c| {
                if let AssistantContent::ToolCall(tc) = c {
                    Some(tc.function.name.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(names, vec!["read_file", "write_file", "list_dir"]);
    }

    #[test]
    fn test_build_assistant_content_thinking_only() {
        // Thinking only, no text, no tools
        let content = build_assistant_content(true, "Just thinking aloud...", None, None, "", &[]);

        assert_eq!(content.len(), 1);
        assert!(matches!(&content[0], AssistantContent::Reasoning(_)));
    }

    #[test]
    fn test_openai_responses_api_model_capabilities() {
        // OpenAI Responses API always needs reasoning history preserved
        let caps = ModelCapabilities::detect("openai_responses", "gpt-4o");
        assert!(
            caps.supports_thinking_history,
            "OpenAI Responses API should support thinking history"
        );

        let caps = ModelCapabilities::detect("openai_responses", "o3-mini");
        assert!(
            caps.supports_thinking_history,
            "OpenAI Responses API with o3 should support thinking history"
        );
    }

    #[test]
    fn test_zai_model_capabilities() {
        // Z.AI GLM-4.7 supports thinking, GLM-4.5 does not
        let caps = ModelCapabilities::detect("zai", "GLM-4.7");
        assert!(
            caps.supports_thinking_history,
            "Z.AI GLM-4.7 should support thinking history"
        );

        let caps = ModelCapabilities::detect("zai", "glm-4.7-flash");
        assert!(
            caps.supports_thinking_history,
            "Z.AI GLM-4.7-flash should support thinking history"
        );

        let caps = ModelCapabilities::detect("zai", "GLM-4.5-air");
        assert!(
            !caps.supports_thinking_history,
            "Z.AI GLM-4.5 should not support thinking history"
        );
    }

    #[test]
    fn test_build_assistant_content_with_id_and_signature() {
        // Both ID and signature present (Anthropic case with streaming)
        let content = build_assistant_content(
            true,
            "Extended thinking...",
            Some("thinking_id_abc".into()),
            Some("signature_xyz".into()),
            "",
            &[make_tool_call("tc_1", "bash")],
        );

        assert_eq!(content.len(), 2);

        if let AssistantContent::Reasoning(reasoning) = &content[0] {
            assert_eq!(reasoning.id, Some("thinking_id_abc".to_string()));
            assert_eq!(reasoning.signature, Some("signature_xyz".to_string()));
            assert!(!reasoning.reasoning.is_empty());
        } else {
            panic!("Expected Reasoning content");
        }
    }

    #[test]
    fn test_openrouter_does_not_support_thinking() {
        // OpenRouter proxies requests but doesn't have native thinking support
        let caps = ModelCapabilities::detect("openrouter", "anthropic/claude-3-opus");
        assert!(
            !caps.supports_thinking_history,
            "OpenRouter should not support thinking history (proxy)"
        );
    }
}
