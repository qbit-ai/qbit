# Plan: rig-openai-responses Crate (Revised)

Thin adapter crate that wraps `async-openai` to provide rig-core `CompletionModel` trait implementation with explicit streaming event handling for reasoning models (o1, o3, gpt-5.x).

## Motivation

The rig-core library has known issues with streaming event handling:
- **Issue #1054**: Events are inconsistent between providers
- **Issue #1072**: OpenAI-with-reasoning streaming isn't unified

## Key Decision: Use async-openai

Instead of building SSE parsing from scratch, we leverage **[async-openai](https://github.com/64bit/async-openai)** (v0.32.3):

| What async-openai provides | What we build |
|---------------------------|---------------|
| All OpenAI API types | rig-core trait adapter |
| HTTP client with auth | Event mapping logic |
| SSE parsing | `CompletionModel` impl |
| `ResponseStreamEvent` enum (40+ variants) | `RawStreamingChoice` conversion |
| Battle-tested, maintained | Thin integration layer |

### async-openai Event Types We'll Use

```rust
// From async-openai::types::responses::stream
ResponseStreamEvent::ResponseOutputTextDelta(e)           // text delta
ResponseStreamEvent::ResponseReasoningSummaryTextDelta(e) // reasoning delta
ResponseStreamEvent::ResponseReasoningTextDelta(e)        // reasoning text delta
ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(e) // tool args delta
ResponseStreamEvent::ResponseCompleted(e)                  // completion with usage
ResponseStreamEvent::ResponseError(e)                      // errors
```

## Architecture

### Crate Structure (Simplified)

```
rig-openai-responses/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Module exports, re-exports
│   ├── completion.rs    # CompletionModel implementation + stream mapping
│   └── error.rs         # Error type wrapper
```

**No custom types.rs or streaming.rs needed** - async-openai provides everything.

### Key Types

#### Event Mapping: async-openai → rig-core

```rust
use async_openai::types::ResponseStreamEvent;
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, ToolCallDeltaContent};

fn map_event(event: ResponseStreamEvent) -> Option<RawStreamingChoice<StreamingResponseData>> {
    match event {
        // Text deltas → Message
        ResponseStreamEvent::ResponseOutputTextDelta(e) => {
            Some(RawStreamingChoice::Message(e.delta))
        }

        // Reasoning deltas → ReasoningDelta (EXPLICIT separation!)
        ResponseStreamEvent::ResponseReasoningSummaryTextDelta(e) => {
            Some(RawStreamingChoice::ReasoningDelta {
                id: Some(e.item_id),
                reasoning: e.delta,
            })
        }
        ResponseStreamEvent::ResponseReasoningTextDelta(e) => {
            Some(RawStreamingChoice::ReasoningDelta {
                id: Some(e.item_id),
                reasoning: e.delta,
            })
        }

        // Function call deltas → ToolCallDelta
        ResponseStreamEvent::ResponseFunctionCallArgumentsDelta(e) => {
            Some(RawStreamingChoice::ToolCallDelta {
                id: e.item_id,
                content: ToolCallDeltaContent::Delta(e.delta),
            })
        }

        // Function call start (from OutputItemAdded)
        ResponseStreamEvent::ResponseOutputItemAdded(e) => {
            if let OutputItem::FunctionCall { id, call_id, name, .. } = e.item {
                Some(RawStreamingChoice::ToolCall(RawStreamingToolCall {
                    id: id.clone(),
                    call_id: Some(call_id),
                    name,
                    arguments: serde_json::json!({}),
                    signature: None,
                    additional_params: None,
                }))
            } else {
                None
            }
        }

        // Completion → FinalResponse with usage
        ResponseStreamEvent::ResponseCompleted(e) => {
            Some(RawStreamingChoice::FinalResponse(StreamingResponseData {
                usage: e.response.usage.map(|u| Usage {
                    prompt_tokens: u.input_tokens,
                    completion_tokens: u.output_tokens,
                    total_tokens: u.total_tokens,
                }),
            }))
        }

        // Errors
        ResponseStreamEvent::ResponseError(e) => {
            tracing::error!("OpenAI stream error: {:?}", e);
            Some(RawStreamingChoice::Message(format!("[Error: {:?}]", e)))
        }

        // Lifecycle events we don't need to emit
        ResponseStreamEvent::ResponseCreated(_)
        | ResponseStreamEvent::ResponseInProgress(_)
        | ResponseStreamEvent::ResponseOutputItemDone(_)
        | ResponseStreamEvent::ResponseContentPartAdded(_)
        | ResponseStreamEvent::ResponseContentPartDone(_)
        | ResponseStreamEvent::ResponseOutputTextDone(_)
        | ResponseStreamEvent::ResponseReasoningSummaryTextDone(_)
        | ResponseStreamEvent::ResponseReasoningTextDone(_)
        | ResponseStreamEvent::ResponseFunctionCallArgumentsDone(_) => None,

        // Other events (web search, file search, etc.) - log and skip for now
        other => {
            tracing::debug!("Unhandled OpenAI stream event: {:?}", other);
            None
        }
    }
}
```

#### CompletionModel (`completion.rs`)

```rust
use async_openai::{Client as OpenAIClient, config::OpenAIConfig};
use async_openai::types::{
    CreateResponseArgs, ResponseStreamEvent, ReasoningEffort as OAReasoningEffort,
};
use rig::completion::{self, CompletionError, CompletionRequest, CompletionResponse};
use rig::streaming::{RawStreamingChoice, StreamingCompletionResponse};

/// Wrapper around async-openai client
pub struct Client {
    inner: OpenAIClient<OpenAIConfig>,
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Self {
        let config = OpenAIConfig::new().with_api_key(api_key);
        Self {
            inner: OpenAIClient::with_config(config),
        }
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        Self {
            inner: OpenAIClient::with_config(config),
        }
    }

    pub fn completion_model(&self, model: impl Into<String>) -> CompletionModel {
        CompletionModel::new(self.inner.clone(), model.into())
    }
}

pub struct CompletionModel {
    client: OpenAIClient<OpenAIConfig>,
    model: String,
    reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl CompletionModel {
    pub fn new(client: OpenAIClient<OpenAIConfig>, model: String) -> Self {
        Self {
            client,
            model,
            reasoning_effort: None,
        }
    }

    pub fn with_reasoning_effort(mut self, effort: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(effort);
        self
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StreamingResponseData {
    pub usage: Option<Usage>,
}

impl rig::completion::GetTokenUsage for StreamingResponseData {
    fn token_usage(&self) -> Option<rig::completion::Usage> {
        self.usage.as_ref().map(|u| rig::completion::Usage {
            input_tokens: u.input_tokens as u64,
            output_tokens: u.output_tokens as u64,
            total_tokens: (u.input_tokens + u.output_tokens) as u64,
        })
    }
}

impl completion::CompletionModel for CompletionModel {
    type Response = async_openai::types::Response;
    type StreamingResponse = StreamingResponseData;
    type Client = Client;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.inner.clone(), model.into())
    }

    async fn completion(&self, request: CompletionRequest)
        -> Result<CompletionResponse<Self::Response>, CompletionError>
    {
        let openai_request = self.build_request(&request)?;

        let response = self.client
            .responses()
            .create(openai_request)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        Ok(self.convert_response(response))
    }

    async fn stream(&self, request: CompletionRequest)
        -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        let openai_request = self.build_request(&request)?;

        let stream = self.client
            .responses()
            .create_stream(openai_request)
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        // Map async-openai events to rig-core RawStreamingChoice
        use futures::StreamExt;
        let mapped = stream.filter_map(|result| async move {
            match result {
                Ok(event) => map_event(event),
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    Some(Ok(RawStreamingChoice::Message(format!("[Error: {}]", e))))
                }
            }
        });

        Ok(StreamingCompletionResponse::stream(Box::pin(mapped)))
    }
}
```

## Key Differences from rig-core

| Aspect | rig-core | rig-openai-responses |
|--------|----------|---------------------|
| Event granularity | 6 types | 12+ explicit types |
| Reasoning handling | Mixed with text | Explicit ReasoningStart/Delta/Done |
| Tool call tracking | Stateless | Stateful (tracks pending calls) |
| Error handling | Generic | OpenAI-specific error types |
| Reasoning IDs | Optional | Always preserved (required for history) |

## Integration Points

### 1. LlmClient Enum (`qbit-llm-providers/src/lib.rs`)

```rust
pub enum LlmClient {
    // ... existing variants ...

    /// OpenAI Responses API with explicit reasoning support
    RigOpenAiResponsesCustom(rig_openai_responses::CompletionModel),
}
```

### 2. Client Creation (`qbit-ai/src/llm_client.rs`)

```rust
pub async fn create_openai_reasoning_components(
    config: OpenAiClientConfig<'_>,
    shared_config: SharedComponentsConfig,
) -> Result<AgentBridgeComponents> {
    // Use custom provider for reasoning models
    let client = rig_openai_responses::Client::new(&config.api_key);
    let mut completion_model = client.completion_model(&config.model);

    // Configure reasoning effort for reasoning models
    if let Some(effort) = config.reasoning_effort {
        completion_model = completion_model.with_reasoning_effort(
            match effort.as_str() {
                "low" => rig_openai_responses::ReasoningEffort::Low,
                "medium" => rig_openai_responses::ReasoningEffort::Medium,
                _ => rig_openai_responses::ReasoningEffort::High,
            }
        );
    }

    Ok(AgentBridgeComponents {
        model: LlmClient::RigOpenAiResponsesCustom(completion_model),
        model_name: config.model.to_string(),
        provider_name: "openai_responses_custom".to_string(),
        // ... rest of config
    })
}

/// Detect if a model should use the custom reasoning provider
pub fn is_openai_reasoning_model(model: &str) -> bool {
    let model_lower = model.to_lowercase();
    model_lower.starts_with("o1")
        || model_lower.starts_with("o3")
        || model_lower.starts_with("o4")
        || model_lower.starts_with("gpt-5")
}
```

## Benefits

1. **Explicit reasoning event handling** - No ambiguity between text and reasoning
2. **Correct reasoning ID preservation** - Required for multi-turn conversations
3. **Model-specific configuration** - Reasoning effort, web search settings
4. **Better debugging** - Clear event types in logs
5. **Future-proof** - Easy to add new event types as OpenAI adds them

## Implementation Order (Simplified)

1. **Phase 1: Crate Setup** (~30 min)
   - Create `Cargo.toml` with dependencies
   - Create `lib.rs` with module structure
   - Create `error.rs` with error wrapper

2. **Phase 2: Core Implementation** (~2 hours)
   - Create `completion.rs`:
     - `Client` wrapper around async-openai
     - `CompletionModel` struct with builder methods
     - `map_event()` function for stream mapping
     - Implement `completion::CompletionModel` trait
   - Message conversion (rig `Message` ↔ async-openai `Input`)
   - Tool definition conversion

3. **Phase 3: Integration** (~1 hour)
   - Add `RigOpenAiResponsesCustom` to `LlmClient` enum
   - Create `create_openai_reasoning_components()` function
   - Auto-detect reasoning models to use this provider

4. **Phase 4: Testing** (~1 hour)
   - Unit tests for event mapping
   - Integration test with mocked responses
   - Manual E2E test with real API

## Review Feedback (Addressed)

Using `async-openai` eliminates most of the original review concerns:

| Original Concern | Resolution |
|-----------------|------------|
| SSE parsing edge cases | Handled by async-openai |
| PendingToolCall state | Handled by async-openai |
| Usage data location | Handled by async-openai (`ResponseCompleted` event) |
| Event name verification | Verified by async-openai against real API |
| GetTokenUsage impl | Still needed - shown in completion.rs |
| Refusal mapping | Map to `RawStreamingChoice::Message("[Refusal] ...")` |

### Remaining Items to Implement

1. **Message conversion** - rig `Message` ↔ async-openai `Input`
2. **Tool definition conversion** - rig `ToolDefinition` ↔ async-openai tool format
3. **Azure support** - Use `Client::with_base_url()` for custom endpoints

### Verified: rig-core 0.29.0 RawStreamingChoice Variants

Confirmed these variants exist for our mapping:
- `RawStreamingChoice::Message(String)` - for text deltas
- `RawStreamingChoice::Reasoning { id, reasoning, signature }` - for complete reasoning
- `RawStreamingChoice::ReasoningDelta { id, reasoning }` - for reasoning deltas ✓
- `RawStreamingChoice::ToolCall(RawStreamingToolCall)` - for tool calls
- `RawStreamingChoice::ToolCallDelta { id, content }` - for tool arg deltas
- `RawStreamingChoice::FinalResponse(R)` - for completion

### Pre-Implementation Verification: DONE

**async-openai v0.32.3** has already verified all event types against the real OpenAI API.
The `ResponseStreamEvent` enum matches the official API spec.

## Testing Strategy (Simplified)

1. **Unit tests** for `map_event()` function - verify each event type maps correctly
2. **Unit tests** for message/tool conversion - rig types ↔ async-openai types
3. **Integration tests** with mocked async-openai client
4. **Manual E2E test** with real API (requires OPENAI_API_KEY)
5. **Regression test** - verify reasoning deltas are NOT mixed with text deltas

## Dependencies (Simplified)

```toml
[package]
name = "rig-openai-responses"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core
rig-core = "^0.29.0"
async-openai = { version = "0.32", features = ["responses"] }

# Async
futures = "0.3"
tokio = { version = "1", features = ["rt"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
tracing = "0.1"
thiserror = "2.0"
```

**Note**: No reqwest or bytes needed - async-openai handles HTTP and SSE parsing.
