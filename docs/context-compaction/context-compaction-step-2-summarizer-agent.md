# Step 2: Summarizer Agent

**Goal:** Create an isolated summarizer agent (modeled after `commit_writer.rs`) that takes conversation text and produces a structured summary. No tools, no HITL, hardcoded system prompt.

**Outcome:** After this step, we have a callable `generate_summary()` function that can be tested independently.

---

## Implementation Notes

> **Changes from original plan:**
>
> | Aspect | Original Plan | Actual |
> |--------|---------------|--------|
> | `generate_summary_with_config` | 4 params (client, model, conversation, factory) | 2 params (client, conversation) - model selection is TODO |
> | LlmClient variants | 10 | 13 (includes `RigOpenAiResponses`, `RigZaiAnthropicLogging`, `Mock`) |
> | Extra functions | None | `parse_summary_response()` with JSON/code-block/fallback handling |
> | Test count | 5 | 8 (added JSON parsing edge case tests) |
> | Tauri command | Uses separate `State<'_, AiState>` | Uses `state.ai_state` from `AppState` |
>
> The code samples below reflect the **original plan**. See `summarizer.rs` for actual implementation.

---

## Prerequisites

- Step 1 completed (transcript writer exists, though not required for this step)

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit/src/ai/commands/summarizer.rs` | **Create** | Summarizer agent module |
| `backend/crates/qbit/src/ai/commands/mod.rs` | Modify | Export summarizer module |
| `backend/crates/qbit-settings/src/schema.rs` | Modify | Add `summarizer_model` setting |

---

## Task Breakdown

### 2.1 Add summarizer_model to settings schema

**File:** `backend/crates/qbit-settings/src/schema.rs`

```rust
// Add to AiSettings struct:

/// Model to use for the summarizer agent.
/// If not specified, uses the session's current model.
/// Example: "claude-sonnet-4-20250514"
#[serde(default, skip_serializing_if = "Option::is_none")]
pub summarizer_model: Option<String>,
```

**Test:**
```rust
#[test]
fn test_summarizer_model_setting() {
    let toml = r#"
        [ai]
        summarizer_model = "claude-haiku-4-5@20251001"
    "#;
    
    let settings: QbitSettings = toml::from_str(toml).unwrap();
    assert_eq!(
        settings.ai.summarizer_model,
        Some("claude-haiku-4-5@20251001".to_string())
    );
}

#[test]
fn test_summarizer_model_defaults_to_none() {
    let toml = r#"
        [ai]
        default_provider = "anthropic"
    "#;
    
    let settings: QbitSettings = toml::from_str(toml).unwrap();
    assert!(settings.ai.summarizer_model.is_none());
}
```

### 2.2 Create failing tests for summarizer

**File:** `backend/crates/qbit/src/ai/commands/summarizer.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_summary_response_deserialization() {
        let json = r#"{"summary": "User asked to implement a feature. Agent created files and ran tests."}"#;
        let response: SummaryResponse = serde_json::from_str(json).unwrap();
        assert!(!response.summary.is_empty());
    }

    #[test]
    fn test_summary_response_handles_multiline() {
        let json = r#"{"summary": "## Task\nImplement feature X\n\n## Actions\n- Created file A\n- Modified file B"}"#;
        let response: SummaryResponse = serde_json::from_str(json).unwrap();
        assert!(response.summary.contains("## Task"));
        assert!(response.summary.contains("## Actions"));
    }

    #[test]
    fn test_summarizer_system_prompt_not_empty() {
        assert!(!SUMMARIZER_SYSTEM_PROMPT.is_empty());
        assert!(SUMMARIZER_SYSTEM_PROMPT.contains("summary"));
    }

    #[test]
    fn test_build_summarizer_prompt() {
        let conversation = "[turn 001] USER:\nHelp me fix a bug\n\n[turn 001] ASSISTANT:\nI'll help you.";
        let prompt = build_summarizer_user_prompt(conversation);
        
        assert!(prompt.contains(conversation));
        assert!(prompt.contains("<conversation>"));
        assert!(prompt.contains("</conversation>"));
    }

    // Integration test (requires LLM - mark as ignored for CI)
    #[tokio::test]
    #[ignore]
    async fn test_generate_summary_integration() {
        // This test requires actual LLM access
        // Run manually with: cargo test -p qbit test_generate_summary_integration -- --ignored
        
        let conversation = r#"
[turn 001] USER:
Please create a hello world function in Rust.

[turn 001] ASSISTANT (completed):
I'll create a simple hello world function for you.

```rust
fn hello_world() {
    println!("Hello, world!");
}
```
"#;

        // Would need actual LLM client setup here
        // let response = generate_summary(&client, conversation).await.unwrap();
        // assert!(!response.summary.is_empty());
    }
}
```

### 2.3 Implement summarizer module

**File:** `backend/crates/qbit/src/ai/commands/summarizer.rs`

```rust
//! Isolated summarizer agent for context compaction.
//!
//! This module provides a dedicated AI agent for generating conversation summaries.
//! It is completely isolated from the main agent and sub-agent system - it cannot
//! be called by any other agent and has no tools. It simply takes a conversation
//! transcript and generates a structured summary.

use anyhow::Result;
use rig::completion::{CompletionModel as _, CompletionRequest, Message};
use rig::message::{Text, UserContent};
use rig::one_or_many::OneOrMany;
use serde::{Deserialize, Serialize};

use crate::ai::llm_client::LlmClient;

/// Response from the summarizer agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResponse {
    /// The generated summary in markdown format
    pub summary: String,
}

/// System prompt for the summarizer agent.
pub const SUMMARIZER_SYSTEM_PROMPT: &str = r#"You are a conversation summarizer for an AI coding assistant. Your task is to create a comprehensive summary of the conversation that preserves all important context for continuing the work.

<purpose>
The summary will be used to continue the conversation after a context reset. The AI assistant reading this summary must be able to:
1. Understand what task the user originally requested
2. Know what has been accomplished so far
3. Be aware of any important decisions, constraints, or preferences expressed
4. Continue working without asking redundant questions
</purpose>

<format>
Structure your summary with these sections:

## Original Request
What the user asked for (1-2 sentences)

## Current State
- What has been done
- Key files created/modified
- Current status (working, blocked, in progress)

## Key Decisions
- Important choices made during the conversation
- User preferences or constraints mentioned
- Technical decisions and their rationale

## Pending Work
- What still needs to be done (if anything)
- Known issues or blockers

## Important Context
- Any other critical information for continuation
- Relevant paths, names, or technical details
</format>

<rules>
- Be comprehensive but concise
- Focus on WHAT was done, not HOW (skip implementation details)
- Include specific file paths, function names, etc. that are referenced
- Preserve any constraints or requirements the user specified
- Note any errors encountered and how they were resolved
- If the task is complete, say so clearly
</rules>

<output>
Return ONLY valid JSON in this exact format:
{"summary": "<your markdown summary here>"}

Do NOT include any text before or after the JSON. Do NOT use markdown code blocks around the JSON.
</output>"#;

/// Build the user prompt for the summarizer.
pub fn build_summarizer_user_prompt(conversation: &str) -> String {
    format!(
        r#"Please summarize the following conversation:

<conversation>
{}
</conversation>

Generate a comprehensive summary that will allow an AI assistant to continue this work without losing context."#,
        conversation
    )
}

/// Generate a summary of the conversation using the LLM.
///
/// # Arguments
/// * `client` - The LLM client to use for generation
/// * `conversation` - The formatted conversation transcript
///
/// # Returns
/// A SummaryResponse containing the generated summary
pub async fn generate_summary(client: &LlmClient, conversation: &str) -> Result<SummaryResponse> {
    let user_prompt = build_summarizer_user_prompt(conversation);

    let user_message = Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: user_prompt,
        })),
    };

    let response_text = call_summarizer_model(client, user_message).await?;

    // Parse the JSON response
    let response: SummaryResponse = serde_json::from_str(&response_text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse summarizer response as JSON: {}. Response was: {}",
            e,
            response_text
        )
    })?;

    Ok(response)
}

/// Call the summarizer model with the given message.
///
/// This function handles the actual LLM API call, dispatching to the correct
/// model type based on the LlmClient variant.
async fn call_summarizer_model(client: &LlmClient, user_message: Message) -> Result<String> {
    use crate::ai::llm_client::LlmClient;

    // Helper macro to reduce repetition
    macro_rules! call_model {
        ($model:expr) => {{
            let request = CompletionRequest::builder()
                .preamble(SUMMARIZER_SYSTEM_PROMPT.to_string())
                .messages(vec![user_message])
                .build();

            let response = $model.completion(request).await?;
            Ok(response.choice.first().to_string())
        }};
    }

    match client {
        LlmClient::VertexAnthropic(model) => call_model!(model),
        LlmClient::RigOpenRouter(model) => call_model!(model),
        LlmClient::RigOpenAi(model) => call_model!(model),
        LlmClient::RigAnthropic(model) => call_model!(model),
        LlmClient::RigOllama(model) => call_model!(model),
        LlmClient::RigGemini(model) => call_model!(model),
        LlmClient::RigGroq(model) => call_model!(model),
        LlmClient::RigXai(model) => call_model!(model),
        LlmClient::RigZai(model) => call_model!(model),
        LlmClient::RigZaiAnthropic(model) => call_model!(model),
    }
}

/// Generate a summary using the configured summarizer model.
///
/// This is the main entry point for context compaction. It:
/// 1. Determines which model to use (summarizer_model setting or fallback to session model)
/// 2. Creates an appropriate LLM client
/// 3. Calls generate_summary()
///
/// # Arguments
/// * `session_client` - The current session's LLM client (used as fallback)
/// * `summarizer_model` - Optional override model from settings
/// * `conversation` - The formatted conversation transcript
/// * `model_factory` - Factory for creating LLM clients
pub async fn generate_summary_with_config(
    session_client: &LlmClient,
    summarizer_model: Option<&str>,
    conversation: &str,
    model_factory: Option<&crate::ai::llm_client::LlmClientFactory>,
) -> Result<SummaryResponse> {
    // If a specific summarizer model is configured and we have a factory, use it
    if let (Some(model_name), Some(factory)) = (summarizer_model, model_factory) {
        // Try to create a client for the summarizer model
        // For now, we'll use the session client as the summarizer model config
        // is just a model name, not a full provider config
        tracing::info!("Using configured summarizer model: {}", model_name);
        // TODO: Create client for specific model when factory supports it
        // For now, fall through to use session client
    }

    // Use the session's client
    generate_summary(session_client, conversation).await
}
```

### 2.4 Export summarizer module

**File:** `backend/crates/qbit/src/ai/commands/mod.rs`

Add:
```rust
pub mod summarizer;

// In re-exports section:
pub use summarizer::*;
```

### 2.5 Add Tauri command for testing (optional)

**File:** `backend/crates/qbit/src/ai/commands/summarizer.rs`

Add a Tauri command for manual testing:

```rust
use tauri::State;
use crate::state::AppState;

/// Generate a summary of a conversation (for testing/debugging).
///
/// This command is primarily for development and testing of the summarizer.
#[tauri::command]
pub async fn generate_conversation_summary(
    session_id: String,
    conversation: String,
    state: State<'_, AppState>,
    ai_state: State<'_, super::AiState>,
) -> Result<SummaryResponse, String> {
    let bridge = ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| super::ai_session_not_initialized_error(&session_id))?;

    let client = bridge.client().read().await;
    let client = client.as_ref().ok_or("LLM client not initialized")?;

    let settings = state.settings_manager.get().await;
    let summarizer_model = settings.ai.summarizer_model.as_deref();

    generate_summary_with_config(client, summarizer_model, &conversation, None)
        .await
        .map_err(|e| e.to_string())
}
```

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit summarizer
cargo test -p qbit-settings summarizer_model
```

### Manual Testing
```bash
# Start the app in dev mode
cd frontend && pnpm tauri dev

# Use browser console or a test script to call:
# await invoke('generate_conversation_summary', { 
#   sessionId: 'test', 
#   conversation: '[turn 001] USER:\nHello\n\n[turn 001] ASSISTANT:\nHi there!' 
# })
```

### Integration Check
```bash
# Full test suite should still pass
cd backend
cargo test
```

---

## Definition of Done

- [ ] `SummaryResponse` struct defined and serializable
- [ ] `SUMMARIZER_SYSTEM_PROMPT` defined with clear instructions
- [ ] `generate_summary()` function implemented
- [ ] `summarizer_model` setting added to schema
- [ ] All tests pass (unit tests, not integration requiring LLM)
- [ ] Module exported from commands/mod.rs
- [ ] Existing tests still pass

---

## Notes

- The summarizer is completely isolated - no tools, no HITL
- Uses same LlmClient infrastructure as other agents
- The system prompt is carefully designed to produce JSON output
- Fallback to session model when no summarizer_model is configured
- This step doesn't integrate with the main flow yet - that comes in Step 5
