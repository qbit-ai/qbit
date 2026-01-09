//! Isolated commit message writer agent.
//!
//! This module provides a dedicated AI agent for generating git commit messages.
//! It is completely isolated from the main agent and sub-agent system - it cannot
//! be called by any other agent and has no tools. It simply takes a diff and
//! generates a commit message.

use rig::completion::{CompletionModel as _, CompletionRequest, Message};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::llm_client::LlmClient;
use crate::state::AppState;

use super::ai_session_not_initialized_error;

/// Response from the commit message generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMessageResponse {
    /// The generated commit summary (first line, max 72 chars)
    pub summary: String,
    /// The generated commit description (optional, can be empty)
    pub description: String,
}

/// System prompt for the commit message writer agent.
const COMMIT_WRITER_SYSTEM_PROMPT: &str = r#"You are a git commit message generator. Your sole purpose is to analyze git diffs and generate clear, concise commit messages following conventional commit format.

<format>
Generate a commit message with:
1. A summary line (max 72 characters) in the format: <type>(<scope>): <description>
2. Optionally, a longer description if the changes are complex

Types:
- feat: A new feature
- fix: A bug fix
- docs: Documentation changes
- style: Code style changes (formatting, whitespace)
- refactor: Code refactoring without behavior changes
- perf: Performance improvements
- test: Adding or modifying tests
- build: Build system or dependency changes
- ci: CI/CD changes
- chore: Maintenance tasks

Scope: The area of the codebase affected (e.g., auth, api, ui, git-panel)
</format>

<output>
Return ONLY valid JSON in this exact format:
{"summary": "<type>(<scope>): <short description>", "description": "<optional longer description or empty string>"}

Do NOT include any text before or after the JSON. Do NOT use markdown code blocks.
</output>

<rules>
- Keep the summary under 72 characters
- Use imperative mood ("Add feature" not "Added feature")
- Be specific but concise
- Focus on WHAT changed and WHY, not HOW
- If there are multiple logical changes, focus on the primary one
- The description should explain motivation/context if the summary isn't sufficient
</rules>"#;

/// Generate a commit message from a git diff.
///
/// This is a completely isolated agent that cannot be called by the main agent
/// or any sub-agents. It only generates commit messages based on the provided diff.
///
/// # Arguments
/// * `session_id` - The session ID to use for the LLM client
/// * `diff` - The git diff to analyze
/// * `file_summary` - Optional summary of files changed (e.g., "3 files: src/foo.rs, src/bar.rs, ...")
///
/// # Returns
/// A CommitMessageResponse with the generated summary and description
///
/// IMPORTANT: Uses get_session_bridge() to clone the Arc and release the map
/// lock immediately. This allows other sessions to initialize/shutdown while
/// this session is making LLM calls.
#[tauri::command]
pub async fn generate_commit_message(
    state: State<'_, AppState>,
    session_id: String,
    diff: String,
    file_summary: Option<String>,
) -> Result<CommitMessageResponse, String> {
    // Get Arc clone and release map lock immediately
    let bridge = state
        .ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    // Access the LLM client (bridge is now an Arc, not a reference from the map)
    let client = bridge.client().clone();

    // Build the user prompt with the diff
    let user_prompt = if let Some(summary) = file_summary {
        format!(
            "Generate a commit message for the following changes:\n\nFiles changed: {}\n\nDiff:\n```\n{}\n```",
            summary, diff
        )
    } else {
        format!(
            "Generate a commit message for the following changes:\n\nDiff:\n```\n{}\n```",
            diff
        )
    };

    // Build the completion request
    let chat_history = vec![Message::User {
        content: OneOrMany::one(UserContent::Text(Text { text: user_prompt })),
    }];

    let request = CompletionRequest {
        preamble: Some(COMMIT_WRITER_SYSTEM_PROMPT.to_string()),
        chat_history: OneOrMany::many(chat_history.clone())
            .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
        documents: vec![],
        tools: vec![],          // No tools - this is a simple completion
        temperature: Some(0.3), // Low temperature for consistent output
        max_tokens: Some(1024), // Commit messages should be short
        tool_choice: None,
        additional_params: None,
    };

    // Make the completion call
    let client_guard = client.read().await;
    let response_text = complete_with_client(&client_guard, request)
        .await
        .map_err(|e| format!("LLM completion failed: {}", e))?;

    // Parse the JSON response
    parse_commit_response(&response_text)
}

/// Execute a completion request using the LLM client.
async fn complete_with_client(
    client: &LlmClient,
    request: CompletionRequest,
) -> anyhow::Result<String> {
    // Extract text from the completion response
    fn extract_text(
        choice: &rig::one_or_many::OneOrMany<rig::completion::AssistantContent>,
    ) -> String {
        let mut text = String::new();
        for content in choice.iter() {
            if let rig::completion::AssistantContent::Text(t) = content {
                text.push_str(&t.text);
            }
        }
        text
    }

    match client {
        LlmClient::VertexAnthropic(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigOpenRouter(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigOpenAi(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigOpenAiResponses(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigAnthropic(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigOllama(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigGemini(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigGroq(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigXai(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::RigZai(model) => {
            let response = model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }
        LlmClient::Mock => {
            // Return a mock response for testing
            Ok(r#"{"summary": "chore: mock commit message", "description": ""}"#.to_string())
        }
    }
}

/// Parse the LLM response into a CommitMessageResponse.
fn parse_commit_response(response: &str) -> Result<CommitMessageResponse, String> {
    // Try to parse as JSON first
    let trimmed = response.trim();

    // Handle markdown code blocks if present
    let json_str = if trimmed.starts_with("```") {
        // Extract content between code blocks
        let without_start = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .unwrap_or(trimmed);
        without_start
            .strip_suffix("```")
            .unwrap_or(without_start)
            .trim()
    } else {
        trimmed
    };

    // Try to parse as JSON
    match serde_json::from_str::<CommitMessageResponse>(json_str) {
        Ok(resp) => Ok(resp),
        Err(json_err) => {
            // Fallback: treat the entire response as the summary
            tracing::warn!(
                "Failed to parse commit message as JSON: {}. Response: {}",
                json_err,
                response
            );

            // Try to extract something useful
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.is_empty() {
                return Err("Empty response from LLM".to_string());
            }

            // Use first non-empty line as summary, rest as description
            let summary = lines[0].trim().to_string();
            let description = if lines.len() > 1 {
                lines[1..].join("\n").trim().to_string()
            } else {
                String::new()
            };

            Ok(CommitMessageResponse {
                summary,
                description,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commit_response_json() {
        let response = r#"{"summary": "feat(git): add commit message generator", "description": "Adds an isolated AI agent for generating commit messages"}"#;
        let result = parse_commit_response(response).unwrap();
        assert_eq!(result.summary, "feat(git): add commit message generator");
        assert_eq!(
            result.description,
            "Adds an isolated AI agent for generating commit messages"
        );
    }

    #[test]
    fn test_parse_commit_response_json_in_code_block() {
        let response = r#"```json
{"summary": "fix(ui): correct button styling", "description": ""}
```"#;
        let result = parse_commit_response(response).unwrap();
        assert_eq!(result.summary, "fix(ui): correct button styling");
        assert_eq!(result.description, "");
    }

    #[test]
    fn test_parse_commit_response_fallback() {
        let response = "feat(git): add commit writer\n\nThis adds a new feature";
        let result = parse_commit_response(response).unwrap();
        assert_eq!(result.summary, "feat(git): add commit writer");
        assert!(result.description.contains("This adds"));
    }
}
