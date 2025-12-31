//! Sub-agent execution logic.
//!
//! This module handles the execution of sub-agents, which are specialized
//! agents that can be invoked by the main agent to handle specific tasks.

use std::sync::Arc;

use anyhow::Result;
use rig::completion::request::ToolDefinition;
use rig::completion::{AssistantContent, CompletionModel as RigCompletionModel, Message};
use rig::message::{Text, ToolCall, ToolResult, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use uuid::Uuid;

use qbit_tools::ToolRegistry;
use qbit_udiff::{ApplyResult, UdiffApplier, UdiffParser};

use crate::definition::{SubAgentContext, SubAgentDefinition, SubAgentResult};
use qbit_core::events::AiEvent;
use qbit_web::tavily::TavilyState;

/// Trait for providing tool definitions to the sub-agent executor.
/// This allows the executor to be decoupled from the tool definition source.
#[async_trait::async_trait]
pub trait ToolProvider: Send + Sync {
    /// Get all available tool definitions
    fn get_all_tool_definitions(&self) -> Vec<ToolDefinition>;

    /// Get tool definitions for tavily web search
    fn get_tavily_tool_definitions(
        &self,
        tavily_state: Option<&Arc<TavilyState>>,
    ) -> Vec<ToolDefinition>;

    /// Filter tools to only those allowed by the sub-agent
    fn filter_tools_by_allowed(
        &self,
        tools: Vec<ToolDefinition>,
        allowed: &[String],
    ) -> Vec<ToolDefinition>;

    /// Execute a tavily tool
    async fn execute_tavily_tool(
        &self,
        tavily_state: Option<&Arc<TavilyState>>,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> (serde_json::Value, bool);

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
    pub tavily_state: Option<&'a Arc<TavilyState>>,
    pub tool_registry: &'a Arc<RwLock<ToolRegistry>>,
    pub workspace: &'a Arc<RwLock<std::path::PathBuf>>,
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
) -> Result<SubAgentResult>
where
    M: RigCompletionModel + Sync,
    P: ToolProvider,
{
    let start_time = std::time::Instant::now();
    let agent_id = &agent_def.id;

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

    // Emit sub-agent start event
    let _ = ctx.event_tx.send(AiEvent::SubAgentStarted {
        agent_id: agent_id.to_string(),
        agent_name: agent_def.name.clone(),
        task: task.to_string(),
        depth: sub_context.depth,
    });

    // Build filtered tools based on agent's allowed tools
    let mut all_tools = tool_provider.get_all_tool_definitions();
    all_tools.extend(tool_provider.get_tavily_tool_definitions(ctx.tavily_state));
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
            let _ = ctx.event_tx.send(AiEvent::SubAgentError {
                agent_id: agent_id.to_string(),
                error: "Maximum iterations reached".to_string(),
            });
            break;
        }

        // Build request with sub-agent's system prompt
        let request = rig::completion::CompletionRequest {
            preamble: Some(agent_def.system_prompt.clone()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: tools.clone(),
            temperature: Some(0.3),
            max_tokens: Some(8192),
            tool_choice: None,
            additional_params: None,
        };

        // Make completion request
        let response = match model.completion(request).await {
            Ok(r) => r,
            Err(e) => {
                let _ = ctx.event_tx.send(AiEvent::SubAgentError {
                    agent_id: agent_id.to_string(),
                    error: e.to_string(),
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

        // Process response
        let mut has_tool_calls = false;
        let mut tool_calls_to_execute: Vec<ToolCall> = vec![];
        let mut text_content = String::new();

        for content in response.choice.iter() {
            match content {
                AssistantContent::Text(text) => {
                    text_content.push_str(&text.text);
                }
                AssistantContent::ToolCall(tool_call) => {
                    has_tool_calls = true;
                    tool_calls_to_execute.push(tool_call.clone());
                }
                AssistantContent::Reasoning(reasoning) => {
                    // Log thinking content from sub-agent (not exposed to parent)
                    let thinking_text = reasoning.reasoning.join("");
                    if !thinking_text.is_empty() {
                        tracing::debug!("[sub-agent] Thinking: {} chars", thinking_text.len());
                    }
                }
            }
        }

        if !text_content.is_empty() {
            accumulated_response.push_str(&text_content);
        }

        if !has_tool_calls {
            break;
        }

        // Add assistant response to history
        // IMPORTANT: Thinking blocks MUST come first when extended thinking is enabled
        let mut thinking_content: Vec<AssistantContent> = vec![];
        let mut other_content: Vec<AssistantContent> = vec![];
        for c in response.choice.iter() {
            match c {
                AssistantContent::Reasoning(_) => thinking_content.push(c.clone()),
                _ => other_content.push(c.clone()),
            }
        }
        // Combine: thinking first, then other content
        thinking_content.append(&mut other_content);
        let assistant_content = thinking_content;

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
            let tool_id = tool_call.id.clone();

            // Emit tool request event
            let request_id = Uuid::new_v4().to_string();
            let _ = ctx.event_tx.send(AiEvent::SubAgentToolRequest {
                agent_id: agent_id.to_string(),
                tool_name: tool_name.to_string(),
                args: tool_args.clone(),
                request_id: request_id.clone(),
            });

            // Execute the tool
            let (result_value, success) = if tool_name == "web_fetch" {
                tool_provider
                    .execute_web_fetch_tool(tool_name, &tool_args)
                    .await
            } else if tool_name.starts_with("web_search") || tool_name == "web_extract" {
                tool_provider
                    .execute_tavily_tool(ctx.tavily_state, tool_name, &tool_args)
                    .await
            } else {
                let mut registry = ctx.tool_registry.write().await;
                let result = registry.execute_tool(tool_name, tool_args.clone()).await;

                match &result {
                    Ok(v) => (v.clone(), true),
                    Err(e) => (serde_json::json!({ "error": e.to_string() }), false),
                }
            };

            // Emit tool result event
            let _ = ctx.event_tx.send(AiEvent::SubAgentToolResult {
                agent_id: agent_id.to_string(),
                tool_name: tool_name.to_string(),
                success,
                result: result_value.clone(),
                request_id: request_id.clone(),
            });

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
                id: tool_id.clone(),
                call_id: Some(tool_id),
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
    });

    if !files_modified.is_empty() {
        tracing::info!(
            "[sub-agent] {} modified {} files: {:?}",
            agent_id,
            files_modified.len(),
            files_modified
        );
    }

    Ok(SubAgentResult {
        agent_id: agent_id.to_string(),
        response: final_response,
        context: sub_context,
        success: true,
        duration_ms,
        files_modified,
    })
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
