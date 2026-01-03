# OpenAI Responses API Reasoning ID Fix

## Issue Summary

When using OpenAI models via the Responses API, multi-turn conversations and agentic loops with tool calls would fail with the error:

```
Item 'fc_...' of type 'function_call' was provided without its required 'reasoning' item: 'rs_...'
```

This error occurred on the second iteration of an agentic loop or on subsequent conversation turns.

## Root Cause

The OpenAI Responses API generates internal reasoning IDs (`rs_...`) that function calls (`fc_...`) reference. Unlike the Chat Completions API, the Responses API requires these reasoning items to be preserved in the conversation history for function calls to work correctly.

### Why It Failed

1. **Empty reasoning content**: The Responses API sometimes sends reasoning IDs with empty or minimal content. Our code only added reasoning to history when `thinking_content` was non-empty:
   ```rust
   // OLD (broken): Only added reasoning if content was non-empty
   if supports_thinking && !thinking_content.is_empty() {
       assistant_content.push(AssistantContent::Reasoning(...));
   }
   ```

2. **Wrong provider detection**: The OpenAI client was using `provider_name: "openai"` which only enabled thinking history for reasoning models (o1/o3/o4/codex). Regular models like `gpt-5.1` didn't get thinking history support, even though the Responses API requires it for ALL models.

3. **History not preserved**: The `agent_bridge.rs` was discarding the full conversation history returned by the agentic loop and only saving text responses.

## The Fix

### 1. Include reasoning when there's an ID (agentic_loop.rs)

```rust
// NEW (fixed): Include reasoning if there's content OR an ID
let has_reasoning = !thinking_content.is_empty() || thinking_id.is_some();
if supports_thinking && has_reasoning {
    assistant_content.push(AssistantContent::Reasoning(
        Reasoning::multi(vec![thinking_content.clone()])
            .optional_id(thinking_id.clone())
            .with_signature(thinking_signature.clone()),
    ));
}
```

### 2. Use distinct provider name for Responses API (llm_client.rs)

```rust
// Changed from "openai" to "openai_responses"
provider_name: "openai_responses".to_string(),
```

### 3. Always enable thinking history for Responses API (model_capabilities.rs)

```rust
fn detect_thinking_history_support(provider_name: &str, model_name: &str) -> bool {
    match provider_name {
        // OpenAI Responses API: ALWAYS preserve reasoning history
        "openai_responses" => true,

        // OpenAI Chat Completions API: Only for reasoning models
        "openai" => {
            model_lower.starts_with("o1")
                || model_lower.starts_with("o3")
                || model_lower.starts_with("o4")
                || model_lower.contains("codex")
        }
        // ...
    }
}
```

### 4. Preserve full history in agent_bridge.rs

Updated `finalize_execution()` to accept and store the full `final_history` from the agentic loop:

```rust
async fn finalize_execution(
    &self,
    accumulated_response: String,
    final_history: Vec<Message>,  // Now preserved
    token_usage: Option<TokenUsage>,
    start_time: std::time::Instant,
) -> String {
    // Replace conversation history with the full history from the agentic loop
    {
        let mut history_guard = self.conversation_history.write().await;
        *history_guard = final_history;
    }
    // ...
}
```

All `execute_with_*_model()` methods were updated to pass `final_history` to `finalize_execution()`.

### 5. Fix eval executor (executor.rs)

Updated the eval executor to use `"openai_responses"` for OpenAI evals since they use the Responses API.

## Files Modified

| File | Change |
|------|--------|
| `crates/qbit-ai/src/agentic_loop.rs` | Include reasoning when ID exists (even if content empty) |
| `crates/qbit-ai/src/llm_client.rs` | Use `"openai_responses"` provider name |
| `crates/qbit-ai/src/agent_bridge.rs` | Preserve full history, pass to `finalize_execution()` |
| `crates/qbit-llm-providers/src/model_capabilities.rs` | Always enable thinking history for `openai_responses` |
| `crates/qbit-evals/src/executor.rs` | Use `"openai_responses"` provider name |

## Key Insight

The Responses API is fundamentally different from the Chat Completions API in how it handles reasoning:

- **Chat Completions API**: Reasoning is optional and only present for reasoning models (o1/o3/o4)
- **Responses API**: Internal reasoning IDs are generated for ALL models and MUST be preserved in conversation history for function calls to work

This distinction requires different handling based on which API is being used, not just which model is being used.

## Testing

The fix was verified by:
1. Running multi-turn eval scenarios with OpenAI provider
2. Testing the main agent with multiple iterations in a single turn
3. Testing sub-agents that use OpenAI models

## Date

January 2026
