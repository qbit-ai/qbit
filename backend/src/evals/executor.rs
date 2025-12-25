//! Lightweight agent executor for evaluations.
//!
//! Provides a minimal agent execution loop without the heavyweight features
//! of the main agentic loop (HITL, loop detection, context management, etc.).
//!
//! Uses Vertex Claude Haiku for fast, cost-effective eval runs.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rig::completion::{AssistantContent, CompletionModel as RigCompletionModel, Message};
use rig::message::{Text, ToolCall, ToolResult, ToolResultContent, UserContent};
use rig::one_or_many::OneOrMany;
use rig_anthropic_vertex::{models, Client};
use serde_json::Value;

use crate::tools::ToolRegistry;

use super::config::EvalConfig;
use super::runner::{AgentOutput, ToolCall as EvalToolCall, VerboseConfig};

/// Maximum iterations before stopping to prevent runaway loops
const MAX_ITERATIONS: usize = 50;

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

/// Execute a prompt against the agent in the given workspace.
///
/// This is a lightweight executor that:
/// - Uses Vertex Claude Haiku for speed
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
    let start = std::time::Instant::now();

    // Create verbose writer if enabled
    let mut writer = VerboseWriter::from_config(verbose_config)?;

    // Load configuration from settings.toml with env var fallback
    let config = EvalConfig::load().await?;

    // Create client using service account credentials if available, otherwise fall back to ADC
    let client = if let Some(ref creds_path) = config.credentials_path {
        Client::from_service_account(creds_path, &config.project_id, &config.location).await?
    } else {
        Client::from_env(&config.project_id, &config.location).await?
    };
    let model = client.completion_model(models::CLAUDE_HAIKU_4_5);

    // Create tool registry for the workspace
    let mut registry = ToolRegistry::new(workspace.to_path_buf()).await;

    // Build tool definitions
    let tools = build_eval_tool_definitions();

    // Initialize chat history with the prompt
    let mut chat_history: Vec<Message> = vec![Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: prompt.to_string(),
        })),
    }];

    // Print the user prompt
    if let Some(ref mut w) = writer {
        let header = if w.is_file() {
            "━━━ User ━━━".to_string()
        } else {
            "\x1b[36m━━━ User ━━━\x1b[0m".to_string()
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
        tracing::debug!("Eval executor iteration {}", iteration);
        if let Some(ref mut w) = writer {
            let header = if w.is_file() {
                format!("\n━━━ Agent (turn {}) ━━━", iteration)
            } else {
                format!("\n\x1b[33m━━━ Agent (turn {}) ━━━\x1b[0m", iteration)
            };
            w.writeln(&header)?;
        }

        // Build completion request
        let request = rig::completion::CompletionRequest {
            preamble: Some(EVAL_SYSTEM_PROMPT.to_string()),
            chat_history: OneOrMany::many(chat_history.clone())
                .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
            documents: vec![],
            tools: tools.clone(),
            temperature: Some(0.3), // Lower temperature for more deterministic evals
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
                "Eval completed after {} iterations, {} tool calls",
                iteration,
                all_tool_calls.len()
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
fn build_eval_tool_definitions() -> Vec<rig::completion::ToolDefinition> {
    use crate::tools::build_function_declarations;

    build_function_declarations()
        .into_iter()
        .map(|decl| rig::completion::ToolDefinition {
            name: decl.name,
            description: decl.description,
            parameters: decl.parameters,
        })
        .collect()
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
