//! Lightweight agent executor for evaluations.
//!
//! Provides a minimal agent execution loop without the heavyweight features
//! of the main agentic loop (HITL, loop detection, context management, etc.).
//!
//! Supports multiple LLM providers:
//! - Vertex AI Claude Sonnet (default)
//! - Z.AI GLM-4.7
//! - OpenAI GPT-4o

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rig::completion::{AssistantContent, CompletionModel as RigCompletionModel, Message};
use rig::message::{Text, ToolCall, ToolResult, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;
use serde_json::{json, Value};

use qbit_tools::{build_function_declarations, ToolRegistry};

use crate::config::{EvalConfig, EvalProvider};
use crate::runner::{AgentOutput, ToolCall as EvalToolCall, VerboseConfig};

/// Maximum iterations before stopping to prevent runaway loops
const MAX_ITERATIONS: usize = 50;

/// OpenAI models that don't support the temperature parameter.
/// These are reasoning models that use internal chain-of-thought.
const OPENAI_NO_TEMPERATURE_MODELS: &[&str] = &[
    // o-series reasoning models
    "o1",
    "o1-preview",
    "o3",
    "o3-mini",
    "o4-mini",
    // GPT-5 base models (reasoning-enabled)
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-nano",
    // Codex models
    "gpt-5.1-codex",
    "gpt-5.1-codex-max",
    "codex-mini-latest",
];

/// Check if a model supports the temperature parameter.
fn model_supports_temperature(model_name: &str, provider: EvalProvider) -> bool {
    match provider {
        EvalProvider::OpenAi => {
            // Check if model is in the no-temperature list
            !OPENAI_NO_TEMPERATURE_MODELS
                .iter()
                .any(|m| model_name.to_lowercase().contains(&m.to_lowercase()))
        }
        // Other providers support temperature
        _ => true,
    }
}

/// Writer that can output to either stdout or a file.
#[allow(dead_code)] // Prepared for file-based verbose output
enum VerboseWriter {
    Stdout,
    File(BufWriter<File>),
}

#[allow(dead_code)] // Prepared for file-based verbose output
impl VerboseWriter {
    /// Create a new writer based on verbose config.
    fn from_config(config: &VerboseConfig) -> Result<Option<Self>> {
        if !config.enabled {
            return Ok(None);
        }

        match &config.log_file {
            Some(path) => {
                let file = File::create(path)?;
                Ok(Some(Self::File(BufWriter::new(file))))
            }
            None => Ok(Some(Self::Stdout)),
        }
    }

    /// Write a line to the output.
    fn writeln(&mut self, s: &str) -> Result<()> {
        match self {
            Self::Stdout => {
                println!("{}", s);
            }
            Self::File(f) => {
                writeln!(f, "{}", s)?;
            }
        }
        Ok(())
    }

    /// Flush the output (important for file writes).
    fn flush(&mut self) -> Result<()> {
        if let Self::File(f) = self {
            f.flush()?;
        }
        Ok(())
    }

    /// Check if this is writing to a file (for stripping ANSI codes).
    fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }
}

/// Eval-specific system prompt - minimal and focused
const EVAL_SYSTEM_PROMPT: &str = r#"You are an AI coding assistant being evaluated on your ability to complete software engineering tasks.

You have access to the following tools:
- read_file: Read a file's contents
- write_file: Write or overwrite a file
- create_file: Create a new file (fails if exists)
- edit_file: Edit an existing file with search/replace
- delete_file: Delete a file
- list_files: List files matching a pattern
- list_directory: List directory contents
- grep_file: Search for patterns in files
- run_pty_cmd: Run a shell command

Complete the task efficiently. When done, provide a brief summary of what you accomplished.
Do not ask for clarification - make reasonable assumptions and proceed.
"#;

/// Execute a prompt against the agent in the given workspace using the default provider.
///
/// This is a lightweight executor that:
/// - Uses the configured LLM provider (default: Vertex Claude Sonnet)
/// - Has a minimal set of tools
/// - Runs an agentic loop until completion
/// - Auto-approves all tool calls (no HITL)
///
/// If `verbose_config.enabled` is true, outputs real-time conversation.
/// If `verbose_config.log_file` is set, writes to that file instead of stdout.
pub async fn execute_eval_prompt(
    workspace: &Path,
    prompt: &str,
    verbose_config: &VerboseConfig,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(
        workspace,
        prompt,
        None,
        verbose_config,
        EvalProvider::default(),
    )
    .await
}

/// Execute a prompt with a custom system prompt.
///
/// This variant allows testing how different system prompts affect agent behavior.
/// If `system_prompt` is `None`, uses the default eval system prompt.
pub async fn execute_eval_prompt_with_system(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        EvalProvider::default(),
    )
    .await
}

/// Execute a prompt against the agent using a specific provider.
pub async fn execute_eval_prompt_with_provider(
    workspace: &Path,
    prompt: &str,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(workspace, prompt, None, verbose_config, provider).await
}

/// Execute a prompt with all options: custom system prompt and provider.
pub async fn execute_eval_prompt_with_options(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
) -> Result<AgentOutput> {
    // Load configuration for the specified provider
    let config = EvalConfig::load_for_provider(provider).await?;

    match provider {
        EvalProvider::VertexClaude => {
            execute_with_vertex_claude(workspace, prompt, system_prompt, verbose_config, &config)
                .await
        }
        EvalProvider::Zai => {
            execute_with_zai(workspace, prompt, system_prompt, verbose_config, &config).await
        }
        EvalProvider::OpenAi => {
            execute_with_openai(workspace, prompt, system_prompt, verbose_config, &config).await
        }
    }
}

/// Execute with Vertex AI Claude.
async fn execute_with_vertex_claude(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig_anthropic_vertex::{models, Client};

    let vertex_config = config
        .vertex
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vertex AI configuration not available"))?;

    // Create client using service account credentials if available, otherwise fall back to ADC
    let client = if let Some(ref creds_path) = vertex_config.credentials_path {
        Client::from_service_account(
            creds_path,
            &vertex_config.project_id,
            &vertex_config.location,
        )
        .await?
    } else {
        Client::from_env(&vertex_config.project_id, &vertex_config.location).await?
    };
    // Enable native web search (web_search_20250305)
    // Note: web_fetch_20250910 requires a beta header not yet supported on Vertex AI
    let model = client
        .completion_model(models::CLAUDE_SONNET_4_5)
        .with_web_search();

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "Claude Sonnet 4.5",
        EvalProvider::VertexClaude,
    )
    .await
}

/// Execute with Z.AI GLM.
async fn execute_with_zai(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig::client::CompletionClient;

    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    let client = rig_zai::Client::new(&zai_config.api_key);
    let model = client.completion_model(rig_zai::GLM_4_7);

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "GLM-4.7",
        EvalProvider::Zai,
    )
    .await
}

/// Execute with OpenAI.
async fn execute_with_openai(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig::client::CompletionClient;
    use rig::providers::openai as rig_openai;

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    // Use completion_model which returns Responses API model (same as main app)
    let model = client.completion_model("gpt-5.1");

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "GPT-5.1",
        EvalProvider::OpenAi,
    )
    .await
}

/// Generic execution with any model implementing CompletionModel.
async fn execute_with_model<M>(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    model: M,
    model_name: &str,
    provider: EvalProvider,
) -> Result<AgentOutput>
where
    M: RigCompletionModel,
{
    let start = std::time::Instant::now();

    // Create verbose writer if enabled
    let mut writer = VerboseWriter::from_config(verbose_config)?;

    // Create tool registry for the workspace
    let mut registry = ToolRegistry::new(workspace.to_path_buf()).await;

    // Build tool definitions (applies schema sanitization for OpenAI)
    let tools = build_eval_tool_definitions(provider);

    // Initialize chat history with the prompt
    let mut chat_history: Vec<Message> = vec![Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: prompt.to_string(),
        })),
    }];

    // Print the user prompt
    if let Some(ref mut w) = writer {
        let header = if w.is_file() {
            format!("━━━ User ({}) ━━━", model_name)
        } else {
            format!("\x1b[36m━━━ User ({}) ━━━\x1b[0m", model_name)
        };
        w.writeln("")?;
        w.writeln(&header)?;
        w.writeln(prompt)?;
    }

    let mut accumulated_response = String::new();
    let mut all_tool_calls: Vec<EvalToolCall> = vec![];
    let mut files_modified: Vec<PathBuf> = vec![];
    let mut total_tokens: u32 = 0;

    for iteration in 1..=MAX_ITERATIONS {
        tracing::debug!("Eval executor iteration {} ({})", iteration, model_name);
        if let Some(ref mut w) = writer {
            let header = if w.is_file() {
                format!("\n━━━ Agent (turn {}) ━━━", iteration)
            } else {
                format!("\n\x1b[33m━━━ Agent (turn {}) ━━━\x1b[0m", iteration)
            };
            w.writeln(&header)?;
        }

        // Build completion request
        // Note: Some models (OpenAI reasoning models) don't support temperature
        let temperature = if model_supports_temperature(model_name, provider) {
            Some(0.3) // Low temperature for consistent but not rigid evals
        } else {
            None
        };

        let request = rig::completion::CompletionRequest {
            preamble: Some(system_prompt.unwrap_or(EVAL_SYSTEM_PROMPT).to_string()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: tools.clone(),
            temperature,
            max_tokens: Some(4096),
            tool_choice: None,
            additional_params: None,
        };

        // Make completion request
        let response = model.completion(request).await?;

        // Track token usage from this turn
        total_tokens += response.usage.total_tokens as u32;

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
                    // Log thinking content
                    let thinking_text = reasoning.reasoning.join("");
                    if !thinking_text.is_empty() {
                        tracing::debug!("[eval] Thinking: {} chars", thinking_text.len());
                        if let Some(ref mut w) = writer {
                            let msg = if w.is_file() {
                                format!("[thinking: {} chars]", thinking_text.len())
                            } else {
                                format!("\x1b[90m[thinking: {} chars]\x1b[0m", thinking_text.len())
                            };
                            w.writeln(&msg)?;
                        }
                    }
                }
                AssistantContent::Image(_) => {
                    // Images in responses not supported in evals, skip
                }
            }
        }

        if !text_content.is_empty() {
            accumulated_response.push_str(&text_content);
            accumulated_response.push('\n');
            if let Some(ref mut w) = writer {
                w.writeln(&text_content)?;
            }
        }

        // If no tool calls, we're done
        if !has_tool_calls {
            tracing::info!(
                "Eval completed after {} iterations, {} tool calls ({})",
                iteration,
                all_tool_calls.len(),
                model_name
            );
            break;
        }

        // Build assistant content for history (thinking first, then other content)
        let mut thinking_content: Vec<AssistantContent> = vec![];
        let mut other_content: Vec<AssistantContent> = vec![];
        for c in response.choice.iter() {
            match c {
                AssistantContent::Reasoning(_) => thinking_content.push(c.clone()),
                _ => other_content.push(c.clone()),
            }
        }
        thinking_content.append(&mut other_content);

        chat_history.push(Message::Assistant {
            id: None,
            content: OneOrMany::many(thinking_content).unwrap_or_else(|_| {
                OneOrMany::one(AssistantContent::Text(Text {
                    text: String::new(),
                }))
            }),
        });

        // Execute tool calls
        let mut tool_results: Vec<UserContent> = vec![];

        for tool_call in tool_calls_to_execute {
            let tool_name = &tool_call.function.name;
            let tool_args = normalize_args(tool_name, tool_call.function.arguments.clone());
            let tool_id = tool_call.id.clone();
            // For OpenAI Responses API, call_id is the ID needed for tool results
            let tool_call_id = tool_call.call_id.clone();

            tracing::debug!("Executing tool: {} with args: {}", tool_name, tool_args);

            if let Some(ref mut w) = writer {
                // Format args nicely for display
                let args_display = if let Some(obj) = tool_args.as_object() {
                    obj.iter()
                        .map(|(k, v)| {
                            let v_str = match v {
                                serde_json::Value::String(s) => {
                                    if s.len() > 100 {
                                        format!("\"{}...\"", &s[..100])
                                    } else {
                                        format!("\"{}\"", s)
                                    }
                                }
                                _ => v.to_string(),
                            };
                            format!("{}={}", k, v_str)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    tool_args.to_string()
                };
                let msg = if w.is_file() {
                    format!("→ {}({})", tool_name, args_display)
                } else {
                    format!("\x1b[35m→ {}({})\x1b[0m", tool_name, args_display)
                };
                w.writeln(&msg)?;
            }

            // Execute the tool
            let (result_value, success) =
                match registry.execute_tool(tool_name, tool_args.clone()).await {
                    Ok(v) => {
                        // Check for error field in result
                        let has_error = v.get("error").is_some();
                        (v, !has_error)
                    }
                    Err(e) => (serde_json::json!({ "error": e.to_string() }), false),
                };

            if let Some(ref mut w) = writer {
                let status = if w.is_file() {
                    if success {
                        "✓"
                    } else {
                        "✗"
                    }
                } else if success {
                    "\x1b[32m✓\x1b[0m"
                } else {
                    "\x1b[31m✗\x1b[0m"
                };
                let result_preview = serde_json::to_string(&result_value)
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect::<String>();
                let msg = if result_preview.len() >= 200 {
                    format!("  {} {}...", status, result_preview)
                } else {
                    format!("  {} {}", status, result_preview)
                };
                w.writeln(&msg)?;
            }

            // Track the tool call
            all_tool_calls.push(EvalToolCall {
                name: tool_name.to_string(),
                input: tool_args.clone(),
                output: Some(serde_json::to_string(&result_value).unwrap_or_default()),
                success,
            });

            // Track files modified
            if success && is_write_tool(tool_name) {
                if let Some(path) = extract_file_path(tool_name, &tool_args) {
                    let full_path = workspace.join(&path);
                    if !files_modified.contains(&full_path) {
                        files_modified.push(full_path);
                    }
                }
            }

            let result_text = serde_json::to_string(&result_value).unwrap_or_default();
            tool_results.push(UserContent::ToolResult(ToolResult {
                id: tool_id.clone(),
                // Use call_id for OpenAI Responses API compatibility
                call_id: tool_call_id.or(Some(tool_id)),
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

    // Flush writer to ensure all output is written
    if let Some(ref mut w) = writer {
        w.flush()?;
    }

    Ok(AgentOutput {
        response: accumulated_response.trim().to_string(),
        tool_calls: all_tool_calls,
        files_modified,
        duration_ms: start.elapsed().as_millis() as u64,
        tokens_used: Some(total_tokens),
    })
}

/// Build tool definitions for eval execution.
///
/// For OpenAI, applies schema sanitization to meet strict mode requirements.
fn build_eval_tool_definitions(provider: EvalProvider) -> Vec<rig::completion::ToolDefinition> {
    build_function_declarations()
        .into_iter()
        .map(|decl| {
            let parameters = if provider == EvalProvider::OpenAi {
                // OpenAI Responses API requires strict mode schema:
                // - additionalProperties: false
                // - all properties in required array
                // - optional properties must be nullable
                sanitize_schema_for_openai(decl.parameters)
            } else {
                decl.parameters
            };
            rig::completion::ToolDefinition {
                name: decl.name,
                description: decl.description,
                parameters,
            }
        })
        .collect()
}

/// Sanitize JSON schema for OpenAI strict mode compatibility.
///
/// This function recursively:
/// - Adds `additionalProperties: false` to all object types
/// - Makes optional properties nullable by adding "null" to the type
/// - Includes all properties in the `required` array
fn sanitize_schema_for_openai(schema: serde_json::Value) -> serde_json::Value {
    sanitize_schema_recursive(schema, &HashSet::new())
}

/// Internal recursive schema sanitization with context about which properties are required.
fn sanitize_schema_recursive(
    mut schema: serde_json::Value,
    _parent_required: &HashSet<String>,
) -> serde_json::Value {
    if let Some(obj) = schema.as_object_mut() {
        // Remove top-level anyOf/allOf/oneOf (not supported by some providers)
        obj.remove("anyOf");
        obj.remove("allOf");
        obj.remove("oneOf");

        // Check if this is an object type schema
        let is_object_type = obj
            .get("type")
            .map(|t| {
                t == "object" || (t.is_array() && t.as_array().unwrap().contains(&json!("object")))
            })
            .unwrap_or(false);

        // Add additionalProperties: false for object types (OpenAI strict mode)
        if is_object_type || obj.contains_key("properties") {
            obj.insert(
                "additionalProperties".to_string(),
                serde_json::Value::Bool(false),
            );
        }

        // Get the set of originally required properties at this level
        let originally_required: HashSet<String> = obj
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // Collect all property keys
        let mut all_property_keys: Vec<String> = Vec::new();

        // Recursively sanitize properties and make optional ones nullable
        if let Some(props) = obj.get_mut("properties") {
            if let Some(props_obj) = props.as_object_mut() {
                all_property_keys = props_obj.keys().cloned().collect();

                for (key, prop_value) in props_obj.iter_mut() {
                    // First, handle oneOf simplification
                    if let Some(prop_obj) = prop_value.as_object_mut() {
                        if prop_obj.contains_key("oneOf") {
                            if let Some(one_of) = prop_obj.remove("oneOf") {
                                if let Some(arr) = one_of.as_array() {
                                    if let Some(first) = arr.first() {
                                        if let Some(first_obj) = first.as_object() {
                                            for (k, v) in first_obj {
                                                prop_obj.insert(k.clone(), v.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        prop_obj.remove("anyOf");
                        prop_obj.remove("allOf");
                    }

                    // Recursively sanitize nested schemas
                    *prop_value =
                        sanitize_schema_recursive(prop_value.take(), &originally_required);

                    // For optional properties (not in original required array),
                    // make them nullable by adding "null" to the type
                    if !originally_required.contains(key) {
                        if let Some(prop_obj) = prop_value.as_object_mut() {
                            if let Some(type_val) = prop_obj.get_mut("type") {
                                if let Some(type_str) = type_val.as_str() {
                                    *type_val = json!([type_str, "null"]);
                                } else if let Some(type_arr) = type_val.as_array_mut() {
                                    if !type_arr.iter().any(|v| v == "null") {
                                        type_arr.push(json!("null"));
                                    }
                                }
                            } else if !prop_obj.contains_key("properties")
                                && !prop_obj.contains_key("items")
                            {
                                // Only add default type if not a complex nested schema
                                prop_obj.insert("type".to_string(), json!(["string", "null"]));
                            }
                        }
                    }
                }
            }
        }

        // Handle array items schema
        if let Some(items) = obj.get_mut("items") {
            *items = sanitize_schema_recursive(items.take(), &HashSet::new());
        }

        // Set all properties as required (OpenAI Responses API strict mode)
        if !all_property_keys.is_empty() {
            let required_array: Vec<serde_json::Value> = all_property_keys
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            obj.insert(
                "required".to_string(),
                serde_json::Value::Array(required_array),
            );
        }
    }
    schema
}

/// Normalize tool arguments (handle run_pty_cmd variants).
fn normalize_args(tool_name: &str, args: Value) -> Value {
    if tool_name == "run_pty_cmd" {
        // Handle both "command" and "cmd" parameter names
        if let Some(cmd) = args.get("cmd").and_then(|v| v.as_str()) {
            return serde_json::json!({ "command": cmd });
        }
    }
    args
}

/// Check if a tool modifies files.
fn is_write_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write_file" | "create_file" | "edit_file" | "delete_file"
    )
}

/// Extract file path from tool arguments.
fn extract_file_path(tool_name: &str, args: &Value) -> Option<String> {
    match tool_name {
        "write_file" | "create_file" | "edit_file" | "delete_file" => args
            .get("path")
            .or_else(|| args.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_args_run_pty_cmd() {
        let args = serde_json::json!({ "cmd": "echo hello" });
        let normalized = normalize_args("run_pty_cmd", args);
        assert_eq!(normalized["command"].as_str(), Some("echo hello"));
    }

    #[test]
    fn test_normalize_args_passthrough() {
        let args = serde_json::json!({ "path": "test.txt", "content": "hello" });
        let normalized = normalize_args("write_file", args.clone());
        assert_eq!(normalized, args);
    }

    #[test]
    fn test_is_write_tool() {
        assert!(is_write_tool("write_file"));
        assert!(is_write_tool("create_file"));
        assert!(is_write_tool("edit_file"));
        assert!(is_write_tool("delete_file"));
        assert!(!is_write_tool("read_file"));
        assert!(!is_write_tool("run_pty_cmd"));
    }

    #[test]
    fn test_extract_file_path() {
        let args = serde_json::json!({ "path": "src/main.rs", "content": "fn main() {}" });
        assert_eq!(
            extract_file_path("write_file", &args),
            Some("src/main.rs".to_string())
        );

        let args = serde_json::json!({ "command": "ls" });
        assert_eq!(extract_file_path("run_pty_cmd", &args), None);
    }
}
