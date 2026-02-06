//! Conversation summarizer for context compaction.
//!
//! This module provides a dedicated AI agent for generating conversation summaries
//! for context compaction. It is completely isolated from the main agent and sub-agent
//! system - it cannot be called by any other agent and has no tools. It takes a
//! conversation transcript and generates a structured summary.

use anyhow::Result;
use qbit_llm_providers::LlmClient;
use rig::completion::{CompletionModel as _, CompletionRequest, Message};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

/// Response from the conversation summarizer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResponse {
    /// The generated summary containing all structured sections
    pub summary: String,
}

/// System prompt for the conversation summarizer agent.
pub const SUMMARIZER_SYSTEM_PROMPT: &str = r#"Your task is to create a detailed summary of the conversation so far, paying close attention to the user's explicit requests and your previous actions.
This summary should be thorough in capturing technical details, code patterns, and architectural decisions that would be essential for continuing development work without losing context.

Before providing your final summary, wrap your analysis in <analysis> tags to organize your thoughts and ensure you've covered all necessary points. In your analysis process:

1. Chronologically analyze each message and section of the conversation. For each section thoroughly identify:
   - The user's explicit requests and intents
   - Your approach to addressing the user's requests
   - Key decisions, technical concepts and code patterns
   - Specific details like:
     - file names
     - full code snippets
     - function signatures
     - file edits
  - Errors that you ran into and how you fixed them
  - Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
2. Double-check for technical accuracy and completeness, addressing each required element thoroughly.

Your summary should include the following sections:

1. Primary Request and Intent: Capture all of the user's explicit requests and intents in detail
2. Key Technical Concepts: List all important technical concepts, technologies, and frameworks discussed.
3. Files and Code Sections: Enumerate specific files and code sections examined, modified, or created. Pay special attention to the most recent messages and include full code snippets where applicable and include a summary of why this file read or edit is important.
4. Errors and fixes: List all errors that you ran into, and how you fixed them. Pay special attention to specific user feedback that you received, especially if the user told you to do something differently.
5. Problem Solving: Document problems solved and any ongoing troubleshooting efforts.
6. All user messages: List ALL user messages that are not tool results. These are critical for understanding the users' feedback and changing intent.
6. Pending Tasks: Outline any pending tasks that you have explicitly been asked to work on.
7. Current Work: Describe in detail precisely what was being worked on immediately before this summary request, paying special attention to the most recent messages from both user and assistant. Include file names and code snippets where applicable.
8. Optional Next Step: List the next step that you will take that is related to the most recent work you were doing. IMPORTANT: ensure that this step is DIRECTLY in line with the user's most recent explicit requests, and the task you were working on immediately before this summary request. If your last task was concluded, then only list next steps if they are explicitly in line with the users request. Do not start on tangential requests or really old requests that were already completed without confirming with the user first.
                       If there is a next step, include direct quotes from the most recent conversation showing exactly what task you were working on and where you left off. This should be verbatim to ensure there's no drift in task interpretation.

Here's an example of how your output should be structured:

<example>
<analysis>
[Your thought process, ensuring all points are covered thoroughly and accurately]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]
   - [...]

3. Files and Code Sections:
   - [File Name 1]
      - [Summary of why this file is important]
      - [Summary of the changes made to this file, if any]
      - [Important Code Snippet]
   - [File Name 2]
      - [Important Code Snippet]
   - [...]

4. Errors and fixes:
    - [Detailed description of error 1]:
      - [How you fixed the error]
      - [User feedback on the error if any]
    - [...]

5. Problem Solving:
   [Description of solved problems and ongoing troubleshooting]

6. All user messages:
    - [Detailed non tool use user message]
    - [...]

7. Pending Tasks:
   - [Task 1]
   - [Task 2]
   - [...]

8. Current Work:
   [Precise description of current work]

9. Optional Next Step:
   [Optional Next step to take]

</summary>
</example>

Please provide your summary based on the conversation so far, following this structure and ensuring precision and thoroughness in your response.

There may be additional summarization instructions provided in the included context. If so, remember to follow these instructions when creating the above summary. Examples of instructions include:
<example>
## Compact Instructions
When summarizing the conversation focus on typescript code changes and also remember the mistakes you made and how you fixed them.
</example>

<example>
# Summary instructions
When you are using compact - please focus on test output and code changes. Include file reads verbatim.
</example>"#;

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

    // Log the full system prompt
    tracing::info!(
        "[summarizer] System prompt ({} chars):\n{}",
        SUMMARIZER_SYSTEM_PROMPT.len(),
        SUMMARIZER_SYSTEM_PROMPT
    );

    // Log the full user message
    tracing::info!(
        "[summarizer] User message ({} chars):\n{}",
        user_prompt.len(),
        user_prompt
    );

    // Build the user message
    let user_message = Message::User {
        content: OneOrMany::one(UserContent::Text(Text { text: user_prompt })),
    };

    // Call the model
    let response_text = call_summarizer_model(client, user_message).await?;

    // Log the full response
    tracing::info!(
        "[summarizer] Raw LLM response ({} chars):\n{}",
        response_text.len(),
        response_text
    );

    // Extract summary, stripping <analysis> tags
    Ok(extract_summary_text(&response_text))
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

    // TODO: max tokens (output tokens) is model dependent

    // Build the completion request
    let chat_history = vec![user_message.clone()];
    let request = CompletionRequest {
        preamble: Some(SUMMARIZER_SYSTEM_PROMPT.to_string()),
        chat_history: OneOrMany::many(chat_history.clone())
            .unwrap_or_else(|_| OneOrMany::one(chat_history[0].clone())),
        documents: vec![],
        tools: vec![],            // No tools - this is a simple completion
        temperature: Some(0.3),   // Low temperature for consistent output
        max_tokens: Some(64_000), // Summaries can be longer than commit messages
        tool_choice: None,
        additional_params: None,
    };

    // Macro to reduce repetition across provider variants
    macro_rules! complete_with_model {
        ($model:expr) => {{
            let response = $model.completion(request).await?;
            // Log token usage
            tracing::info!(
                "[summarizer] Token usage: input={}, output={}",
                response.usage.input_tokens,
                response.usage.output_tokens
            );
            Ok(extract_text(&response.choice))
        }};
    }

    match client {
        LlmClient::VertexAnthropic(model) => complete_with_model!(model),
        LlmClient::RigOpenRouter(model) => complete_with_model!(model),
        LlmClient::RigOpenAi(model) => complete_with_model!(model),
        LlmClient::RigOpenAiResponses(model) => complete_with_model!(model),
        LlmClient::OpenAiReasoning(model) => complete_with_model!(model),
        LlmClient::RigAnthropic(model) => complete_with_model!(model),
        LlmClient::RigOllama(model) => complete_with_model!(model),
        LlmClient::RigGemini(model) => complete_with_model!(model),
        LlmClient::RigGroq(model) => complete_with_model!(model),
        LlmClient::RigXai(model) => complete_with_model!(model),
        LlmClient::RigZaiSdk(model) => complete_with_model!(model),
        LlmClient::VertexGemini(model) => complete_with_model!(model),
        LlmClient::Mock => {
            // Return a mock response matching the expected <analysis>/<summary> format
            Ok("<analysis>\nMock analysis.\n</analysis>\n\n<summary>\n## Original Request\nMock summary for testing.\n\n## Current State\nN/A\n\n## Key Decisions\nN/A\n\n## Pending Work\nN/A\n\n## Important Context\nN/A\n</summary>".to_string())
        }
    }
}

/// Extract the summary text from the summarizer's raw LLM response.
///
/// The summarizer prompt asks the model to produce `<analysis>...</analysis>` thinking
/// followed by `<summary>...</summary>` content. This function:
/// 1. Strips the `<analysis>` block (internal reasoning, not useful in compacted context)
/// 2. Extracts content from `<summary>` tags if present
/// 3. Falls back to the full response (minus analysis) if no `<summary>` tags found
fn extract_summary_text(response: &str) -> SummaryResponse {
    let mut text = response.to_string();

    // Strip <analysis>...</analysis> blocks (greedy: handles nested content)
    while let Some(start) = text.find("<analysis>") {
        if let Some(end) = text.find("</analysis>") {
            let end = end + "</analysis>".len();
            text.replace_range(start..end, "");
        } else {
            // Unclosed <analysis> tag — strip from tag to end
            text.truncate(start);
            break;
        }
    }

    // Try to extract content from <summary>...</summary> tags
    let summary = if let Some(start) = text.find("<summary>") {
        let content_start = start + "<summary>".len();
        if let Some(end) = text.find("</summary>") {
            text[content_start..end].trim().to_string()
        } else {
            // Unclosed <summary> — take everything after the tag
            text[content_start..].trim().to_string()
        }
    } else {
        text.trim().to_string()
    };

    SummaryResponse { summary }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_strips_analysis_tags() {
        let response = "<analysis>\nLet me think about this...\n</analysis>\n\n<summary>\nThe user asked for help.\n</summary>";
        let result = extract_summary_text(response);
        assert_eq!(result.summary, "The user asked for help.");
        assert!(!result.summary.contains("analysis"));
        assert!(!result.summary.contains("Let me think"));
    }

    #[test]
    fn test_extract_handles_no_tags() {
        let response = "Just a plain summary with no XML tags.";
        let result = extract_summary_text(response);
        assert_eq!(result.summary, "Just a plain summary with no XML tags.");
    }

    #[test]
    fn test_extract_handles_summary_without_analysis() {
        let response = "<summary>\n1. Primary Request\nUser wants to fix a bug.\n</summary>";
        let result = extract_summary_text(response);
        assert_eq!(
            result.summary,
            "1. Primary Request\nUser wants to fix a bug."
        );
    }

    #[test]
    fn test_extract_handles_unclosed_analysis() {
        let response = "Some text before\n<analysis>\nThinking that never ends...";
        let result = extract_summary_text(response);
        assert_eq!(result.summary, "Some text before");
        assert!(!result.summary.contains("Thinking"));
    }

    #[test]
    fn test_extract_handles_unclosed_summary() {
        let response = "<analysis>thinking</analysis>\n<summary>\nThe actual summary content";
        let result = extract_summary_text(response);
        assert_eq!(result.summary, "The actual summary content");
    }

    #[test]
    fn test_extract_preserves_multiline_summary() {
        let response = "<analysis>\nLong analysis here.\n</analysis>\n\n<summary>\n1. Primary Request\n   User wants feature X.\n\n2. Key Technical Concepts\n   - Rust, Tokio\n\n3. Current Work\n   Implementing the feature.\n</summary>";
        let result = extract_summary_text(response);
        assert!(result.summary.contains("Primary Request"));
        assert!(result.summary.contains("Key Technical Concepts"));
        assert!(result.summary.contains("Current Work"));
        assert!(!result.summary.contains("Long analysis here"));
    }

    #[test]
    fn test_extract_handles_empty_response() {
        let result = extract_summary_text("");
        assert_eq!(result.summary, "");
    }

    #[test]
    fn test_extract_handles_whitespace_only() {
        let result = extract_summary_text("   \n\n  ");
        assert_eq!(result.summary, "");
    }
}
