//! Isolated conversation summarizer agent.
//!
//! This module provides a dedicated AI agent for generating conversation summaries
//! for context compaction. It is completely isolated from the main agent and sub-agent
//! system - it cannot be called by any other agent and has no tools. It takes a
//! conversation transcript and generates a structured summary.

use anyhow::Result;
use rig::completion::{CompletionModel as _, CompletionRequest, Message};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::llm_client::LlmClient;
use crate::state::AppState;

use super::ai_session_not_initialized_error;

/// Response from the conversation summarizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResponse {
    /// The generated summary containing all structured sections
    pub summary: String,
}

/// System prompt for the conversation summarizer agent.
pub const SUMMARIZER_SYSTEM_PROMPT: &str = r#"You are a conversation summarizer for an AI coding assistant. Your sole purpose is to analyze conversation transcripts and generate comprehensive summaries that preserve critical context for continued work.

<purpose>
Generate summaries that allow the AI assistant to seamlessly continue working on tasks after context compaction. The summary must capture everything needed to maintain continuity.
</purpose>

<format>
Your summary MUST include these sections:

## Original Request
- What the user originally asked for
- Key requirements and constraints mentioned
- Any clarifications or refinements to the request

## Current State
- What has been accomplished so far
- Files that have been created, modified, or deleted
- Tests that pass or fail
- Any running processes or state

## Key Decisions
- Important technical decisions made and their rationale
- Trade-offs that were considered
- Patterns or approaches chosen

## Pending Work
- What still needs to be done
- Known issues or blockers
- Next steps that were planned

## Important Context
- Error messages or issues encountered
- Specific file paths and line numbers relevant to ongoing work
- Dependencies or relationships between components
- Any warnings or caveats mentioned
</format>

<output>
Return ONLY valid JSON in this exact format:
{"summary": "<the full summary with all sections as markdown>"}

Do NOT include any text before or after the JSON. Do NOT use markdown code blocks around the JSON.
The summary field should contain markdown-formatted text with the sections above.
</output>

<rules>
- Be comprehensive but concise
- Include specific file paths, function names, and code snippets when relevant
- Preserve exact error messages if they were discussed
- Focus on actionable information needed to continue work
- Do not include meta-commentary about the summarization process
- If certain sections have no content, include them with "None" or "N/A"
</rules>"#;

/// Build the user prompt for the summarizer by wrapping the conversation in XML tags.
pub fn build_summarizer_user_prompt(conversation: &str) -> String {
    format!(
        r#"Summarize the following conversation for context compaction:

<conversation>
{}
</conversation>

Generate a comprehensive summary following the required format."#,
        conversation
    )
}

/// Generate a conversation summary using the LLM.
///
/// This function takes a conversation transcript and produces a structured summary
/// suitable for context compaction.
///
/// # Arguments
/// * `client` - The LLM client to use for generation
/// * `conversation` - The conversation transcript to summarize
///
/// # Returns
/// A SummaryResponse containing the structured summary
pub async fn generate_summary(client: &LlmClient, conversation: &str) -> Result<SummaryResponse> {
    let user_prompt = build_summarizer_user_prompt(conversation);

    // Build the user message
    let user_message = Message::User {
        content: OneOrMany::one(UserContent::Text(Text { text: user_prompt })),
    };

    // Call the model
    let response_text = call_summarizer_model(client, user_message).await?;

    // Parse the JSON response
    parse_summary_response(&response_text)
}

/// Internal helper that handles different LlmClient variants.
///
/// Uses a macro to reduce repetition across the many provider variants.
async fn call_summarizer_model(client: &LlmClient, user_message: Message) -> Result<String> {
    // Helper to extract text from completion response
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

    // Build the completion request
    let chat_history = vec![user_message.clone()];
    let request = CompletionRequest {
        preamble: Some(SUMMARIZER_SYSTEM_PROMPT.to_string()),
        chat_history: OneOrMany::many(chat_history.clone())
            .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
        documents: vec![],
        tools: vec![],          // No tools - this is a simple completion
        temperature: Some(0.3), // Low temperature for consistent output
        max_tokens: Some(4096), // Summaries can be longer than commit messages
        tool_choice: None,
        additional_params: None,
    };

    // Macro to reduce repetition across provider variants
    macro_rules! complete_with_model {
        ($model:expr) => {{
            let response = $model.completion(request).await?;
            Ok(extract_text(&response.choice))
        }};
    }

    match client {
        LlmClient::VertexAnthropic(model) => complete_with_model!(model),
        LlmClient::RigOpenRouter(model) => complete_with_model!(model),
        LlmClient::RigOpenAi(model) => complete_with_model!(model),
        LlmClient::RigOpenAiResponses(model) => complete_with_model!(model),
        LlmClient::RigAnthropic(model) => complete_with_model!(model),
        LlmClient::RigOllama(model) => complete_with_model!(model),
        LlmClient::RigGemini(model) => complete_with_model!(model),
        LlmClient::RigGroq(model) => complete_with_model!(model),
        LlmClient::RigXai(model) => complete_with_model!(model),
        LlmClient::RigZai(model) => complete_with_model!(model),
        LlmClient::RigZaiAnthropic(model) => complete_with_model!(model),
        LlmClient::RigZaiAnthropicLogging(model) => complete_with_model!(model),
        LlmClient::Mock => {
            // Return a mock response for testing
            // Note: Using escaped string instead of raw string for proper \n handling in JSON
            Ok("{\"summary\": \"## Original Request\\nMock summary for testing.\\n\\n## Current State\\nN/A\\n\\n## Key Decisions\\nN/A\\n\\n## Pending Work\\nN/A\\n\\n## Important Context\\nN/A\"}".to_string())
        }
    }
}

/// Entry point for generating summaries with optional model configuration.
///
/// This function handles the case where a specific summarizer model may be configured
/// in settings, falling back to the session's default client if not.
///
/// # Arguments
/// * `client` - The LLM client to use (either session default or configured summarizer model)
/// * `conversation` - The conversation transcript to summarize
///
/// # Returns
/// A SummaryResponse containing the structured summary
pub async fn generate_summary_with_config(
    client: &LlmClient,
    conversation: &str,
) -> Result<SummaryResponse> {
    // For now, this just delegates to generate_summary.
    // In the future, this could handle loading a specific summarizer model
    // from configuration if one is specified.
    generate_summary(client, conversation).await
}

/// Parse the LLM response into a SummaryResponse.
fn parse_summary_response(response: &str) -> Result<SummaryResponse> {
    let trimmed = response.trim();

    // Handle markdown code blocks if present
    let json_str = if trimmed.starts_with("```") {
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
    match serde_json::from_str::<SummaryResponse>(json_str) {
        Ok(resp) => Ok(resp),
        Err(json_err) => {
            // Fallback: treat the entire response as the summary
            tracing::warn!(
                "Failed to parse summary as JSON: {}. Using raw response as summary.",
                json_err
            );

            // Use the full response as the summary
            Ok(SummaryResponse {
                summary: trimmed.to_string(),
            })
        }
    }
}

/// Tauri command to generate a conversation summary.
///
/// This command is primarily for testing the summarizer from the frontend.
/// In production, the summarizer is called internally by the context compaction system.
///
/// # Arguments
/// * `state` - The application state
/// * `session_id` - The session ID to use for the LLM client
/// * `conversation` - The conversation transcript to summarize
///
/// # Returns
/// A SummaryResponse containing the structured summary
#[tauri::command]
pub async fn generate_conversation_summary(
    state: State<'_, AppState>,
    session_id: String,
    conversation: String,
) -> Result<SummaryResponse, String> {
    // Get Arc clone and release map lock immediately
    let bridge = state
        .ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    // Access the LLM client
    let client = bridge.client().clone();
    let client_guard = client.read().await;

    // Generate the summary
    generate_summary_with_config(&client_guard, &conversation)
        .await
        .map_err(|e| format!("Failed to generate summary: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_response_deserialization() {
        // JSON with escaped newlines - the \\n becomes \n in the JSON, which becomes actual newline in the parsed string
        let json = "{\"summary\": \"## Original Request\\nTest summary\"}";
        let result: SummaryResponse = serde_json::from_str(json).unwrap();
        assert_eq!(result.summary, "## Original Request\nTest summary");
    }

    #[test]
    fn test_summary_response_handles_multiline() {
        // Test with actual newlines in the JSON string value
        let json = "{\"summary\": \"## Original Request\\nLine 1\\n\\n## Current State\\nLine 2\\nLine 3\"}";
        let result: SummaryResponse = serde_json::from_str(json).unwrap();
        assert!(result.summary.contains("## Original Request"));
        assert!(result.summary.contains("## Current State"));
        assert!(result.summary.contains("Line 1"));
        assert!(result.summary.contains("Line 3"));
    }

    #[test]
    fn test_summarizer_system_prompt_not_empty() {
        assert!(!SUMMARIZER_SYSTEM_PROMPT.is_empty());
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Original Request"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Current State"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Key Decisions"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Pending Work"));
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("## Important Context"));
    }

    #[test]
    fn test_build_summarizer_prompt() {
        let conversation = "User: Hello\nAssistant: Hi there!";
        let prompt = build_summarizer_user_prompt(conversation);

        assert!(prompt.contains("<conversation>"));
        assert!(prompt.contains("</conversation>"));
        assert!(prompt.contains("User: Hello"));
        assert!(prompt.contains("Assistant: Hi there!"));
        assert!(prompt.contains("Summarize the following conversation"));
    }

    #[test]
    fn test_parse_summary_response_json() {
        let response = "{\"summary\": \"## Original Request\\nBuild a feature\\n\\n## Current State\\nIn progress\"}";
        let result = parse_summary_response(response).unwrap();
        assert!(result.summary.contains("## Original Request"));
        assert!(result.summary.contains("Build a feature"));
    }

    #[test]
    fn test_parse_summary_response_json_in_code_block() {
        let response = "```json\n{\"summary\": \"## Original Request\\nTest request\"}\n```";
        let result = parse_summary_response(response).unwrap();
        assert!(result.summary.contains("## Original Request"));
        assert!(result.summary.contains("Test request"));
    }

    #[test]
    fn test_parse_summary_response_fallback() {
        let response = "## Original Request\nThis is a raw summary without JSON";
        let result = parse_summary_response(response).unwrap();
        assert!(result.summary.contains("## Original Request"));
        assert!(result.summary.contains("raw summary"));
    }

    #[tokio::test]
    #[ignore = "requires LLM API access"]
    async fn test_generate_summary_integration() {
        // This test is ignored by default as it requires a real LLM client.
        // To run it, use: cargo test --package qbit -- --ignored
        //
        // Example conversation for testing:
        // let conversation = "User: Help me create a new Rust function\nAssistant: I'll help...";
        // let client = LlmClient::Mock;
        // let result = generate_summary(&client, conversation).await;
        // assert!(result.is_ok());
    }
}
