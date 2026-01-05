# Plan: Full LLM Request/Response Tracing for LangSmith

## Goal

Capture complete LLM request/response data in LangSmith traces, including:
- Input prompt (currently working)
- Output completion text
- Token usage (input_tokens, output_tokens)
- Response model
- Full request/response timing
- Tool execution spans

## References

- [OpenTelemetry GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/gen-ai-spans/)
- [LangSmith OpenTelemetry Integration](https://docs.langchain.com/langsmith/trace-with-opentelemetry)
- [LangChain Blog: OpenTelemetry Support](https://blog.langchain.com/opentelemetry-langsmith/)
- [tracing-opentelemetry crate](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/)
- [tracing #[instrument] macro](https://docs.rs/tracing/latest/tracing/attr.instrument.html)
- [OpenTelemetrySpanExt trait](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/trait.OpenTelemetrySpanExt.html)

## Rust tracing-opentelemetry Best Practices

### Special `otel.*` Fields

The `tracing-opentelemetry` crate reserves special field prefixes:

| Field | Purpose | Example |
|-------|---------|---------|
| `otel.name` | Override span name (for dynamic names) | `otel.name = format!("chat {}", model)` |
| `otel.kind` | Set span kind | `"client"`, `"server"`, `"internal"` |
| `otel.status_code` | Set span status | `"OK"`, `"ERROR"` |
| `otel.status_description` | Status context (with status_code) | Error message |

### `#[instrument]` Macro Options

```rust
#[instrument(
    target = "qbit::llm",           // Log target for filtering
    name = "chat",                   // Custom span name (default: fn name)
    level = "info",                  // Span level
    skip_all,                        // Skip all fn args from fields
    fields(                          // Custom fields
        "key" = %value,              // Formatted value
        "empty" = tracing::field::Empty,  // Record later
    ),
    err,                             // Auto-capture Result::Err
    ret,                             // Auto-capture return value
)]
async fn my_function(...) -> Result<...> { }
```

### Recording Fields Later

```rust
#[instrument(fields(result = tracing::field::Empty))]
async fn my_function() -> Result<String> {
    let result = do_work().await?;

    // Record field value after computation
    tracing::Span::current().record("result", &result);

    Ok(result)
}
```

### Dynamic Attributes with `OpenTelemetrySpanExt`

For attributes beyond the 32-field limit or dynamic keys:

```rust
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::trace::Status;

// Set dynamic attributes
tracing::Span::current().set_attribute("http.response.status_code", 200);

// Set span status
tracing::Span::current().set_status(Status::Ok);

// Add events with attributes
tracing::Span::current().add_event(
    "cache_hit",
    vec![KeyValue::new("cache.key", "user:123")],
);
```

### Async Span Propagation

**DO:** Use `#[instrument]` for async functions - it handles context propagation automatically.

**DON'T:** Use `span.enter()` in async code - the guard doesn't propagate across `.await` points.

```rust
// ❌ WRONG - guard lost at await
let span = info_span!("my_span");
let _guard = span.enter();
some_async_work().await;  // Span context lost!

// ✅ CORRECT - use #[instrument]
#[instrument]
async fn my_function() {
    some_async_work().await;  // Span context preserved
}

// ✅ ALSO CORRECT - use .instrument()
async {
    some_async_work().await;
}
.instrument(info_span!("my_span"))
.await;
```

### Error Handling

Use the `err` argument to automatically capture errors:

```rust
#[instrument(err)]
async fn fallible_work() -> Result<(), Error> {
    // If this returns Err, an error event is automatically emitted
    do_something()?;
    Ok(())
}
```

### Semantic Convention Attributes

OpenTelemetry semantic conventions can be used directly as field names:

```rust
#[instrument(fields(
    "gen_ai.system" = "anthropic",
    "gen_ai.request.model" = %model,
    "server.port" = 443,
))]
```

## Current Problem

The current implementation uses `span.enter()` which doesn't work across `.await` points in async Rust:

```rust
let llm_span = tracing::info_span!("llm.completion", ...);
let _guard = llm_span.enter();  // ❌ Guard loses context at await points

let stream = model.stream(request).await;  // Span context lost here
while let Some(chunk) = stream.next().await {
    // Not in span context
}
// Recording here doesn't work reliably
```

## Solution: Extract to Instrumented Async Function

Use `#[tracing::instrument]` which properly handles async span propagation:

```rust
#[tracing::instrument(
    target = "qbit::llm",
    name = "chat",  // Span name will be "chat {model}"
    skip_all,
    fields(
        "gen_ai.operation.name" = "chat",
        "gen_ai.request.model" = %model_name,
        // ... other fields
    )
)]
async fn stream_llm_completion(...) -> Result<...> {
    // All code here is properly instrumented
    // Span::current() works correctly
}
```

## GenAI Semantic Conventions (OTel Standard)

### LLM Chat Span Attributes

| Attribute | Type | Requirement | Description |
|-----------|------|-------------|-------------|
| `gen_ai.operation.name` | string | Required | `chat` for chat completions |
| `gen_ai.request.model` | string | Required | Model name requested |
| `gen_ai.provider.name` | string | Required | `anthropic`, `openai`, etc. |
| `gen_ai.response.model` | string | Recommended | Actual model used |
| `gen_ai.usage.input_tokens` | int | Recommended | Tokens in prompt |
| `gen_ai.usage.output_tokens` | int | Recommended | Tokens in response |
| `gen_ai.request.temperature` | double | Recommended | Temperature setting |
| `gen_ai.request.max_tokens` | int | Recommended | Max output tokens |
| `gen_ai.response.finish_reasons` | string[] | Recommended | Why generation stopped |

### LangSmith-Specific Attributes

| Attribute | Values | Description |
|-----------|--------|-------------|
| `langsmith.span.kind` | `llm`, `tool`, `chain` | Run type in LangSmith UI |
| `langsmith.metadata.*` | any | Custom metadata |
| `langsmith.trace.session_id` | string | Groups traces into a conversation/thread |
| `langsmith.trace.session_name` | string | Human-readable session name |
| `langsmith.trace.name` | string | Override trace name |
| `langsmith.span.tags` | string (comma-sep) | Custom tags |

### Thread/Session Tracing

LangSmith supports grouping related traces together as "threads" or "sessions". This is essential for:
- Multi-turn conversations
- Tracking agent runs across multiple LLM calls
- Debugging conversation flows

**In LangGraph/LangChain:** Uses `thread_id` in config for checkpointer persistence.

**In LangSmith traces:** Use `langsmith.trace.session_id` attribute to group related spans.

```rust
// Set session ID on the root span to group all agent activity
#[instrument(fields(
    "langsmith.trace.session_id" = %session_id,
    "langsmith.trace.session_name" = %session_name,
))]
async fn run_agent_session(session_id: &str, session_name: &str) {
    // All child spans inherit this session grouping
}
```

### Tool Execution Span Attributes

| Attribute | Type | Requirement | Description |
|-----------|------|-------------|-------------|
| `gen_ai.operation.name` | string | Required | `execute_tool` |
| `gen_ai.tool.name` | string | Recommended | Tool identifier |
| `gen_ai.tool.call.id` | string | Recommended | Unique call ID |
| `gen_ai.tool.type` | string | Recommended | `function` |

## Implementation Plan

### Phase 1: Extract LLM Streaming to Instrumented Function

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

#### Step 1.1: Create StreamResult struct

```rust
/// Result of streaming LLM completion
struct StreamCompletionResult {
    /// Accumulated response text
    pub text: String,
    /// Accumulated thinking/reasoning content
    pub thinking: String,
    /// Thinking signature (for Anthropic)
    pub thinking_signature: Option<String>,
    /// Thinking ID (for OpenAI)
    pub thinking_id: Option<String>,
    /// Tool calls requested by model
    pub tool_calls: Vec<ToolCall>,
    /// Token usage
    pub usage: TokenUsage,
    /// Whether model requested tool calls
    pub has_tool_calls: bool,
}
```

#### Step 1.2: Extract streaming function

```rust
/// Stream LLM completion with full OpenTelemetry instrumentation.
///
/// This function is instrumented with `#[tracing::instrument]` to properly
/// handle async span propagation for LangSmith/OpenTelemetry tracing.
#[tracing::instrument(
    target = "qbit::llm",
    name = "chat",
    skip_all,
    fields(
        // LangSmith span kind
        "langsmith.span.kind" = "llm",
        // GenAI semantic conventions (OTel standard)
        "gen_ai.operation.name" = "chat",
        "gen_ai.provider.name" = %provider_name,
        "gen_ai.request.model" = %model_name,
        "gen_ai.request.max_tokens" = tracing::field::Empty,
        "gen_ai.request.temperature" = tracing::field::Empty,
        // Response fields (filled after streaming)
        "gen_ai.response.model" = tracing::field::Empty,
        "gen_ai.response.finish_reasons" = tracing::field::Empty,
        "gen_ai.usage.input_tokens" = tracing::field::Empty,
        "gen_ai.usage.output_tokens" = tracing::field::Empty,
    )
)]
async fn stream_llm_completion<M>(
    model: &M,
    request: CompletionRequest,
    provider_name: &str,
    model_name: &str,
    supports_thinking: bool,
    ctx: &AgenticLoopContext<'_>,
    iteration: usize,
) -> Result<StreamCompletionResult>
where
    M: rig::completion::CompletionModel + Sync,
{
    // Record request parameters
    let span = tracing::Span::current();
    if let Some(max_tokens) = request.max_tokens {
        span.record("gen_ai.request.max_tokens", max_tokens as i64);
    }
    if let Some(temp) = request.temperature {
        span.record("gen_ai.request.temperature", temp);
    }

    tracing::debug!(
        "[LLM] Starting streaming completion (iteration {}, thinking={})",
        iteration,
        supports_thinking
    );

    // Create stream
    let mut stream = model.stream(request).await.map_err(|e| {
        tracing::error!("Failed to start stream: {}", e);
        anyhow::anyhow!("{}", e)
    })?;

    // Initialize accumulators
    let mut result = StreamCompletionResult::default();
    let mut chunk_count = 0;
    let mut current_tool_id: Option<String> = None;
    let mut current_tool_name: Option<String> = None;
    let mut current_tool_args = String::new();

    // Consume stream
    while let Some(chunk_result) = stream.next().await {
        chunk_count += 1;
        match chunk_result {
            Ok(chunk) => {
                // ... existing chunk processing logic ...
                // (moved from run_agentic_loop_unified)
            }
            Err(e) => {
                tracing::warn!("Stream chunk error at #{}: {}", chunk_count, e);
            }
        }
    }

    tracing::info!(
        "[LLM] Stream completed: {} chunks, {} chars text, {} tool calls",
        chunk_count,
        result.text.len(),
        result.tool_calls.len()
    );

    // Record response data on span
    span.record("gen_ai.usage.input_tokens", result.usage.input_tokens as i64);
    span.record("gen_ai.usage.output_tokens", result.usage.output_tokens as i64);
    span.record("gen_ai.response.finish_reasons", "end_turn");

    Ok(result)
}
```

#### Step 1.3: Update run_agentic_loop_unified to call extracted function

```rust
// In the main loop, replace inline streaming with:
let stream_result = stream_llm_completion(
    model,
    request,
    ctx.provider_name,
    ctx.model_name,
    supports_thinking,
    ctx,
    iteration,
).await?;

// Use results
text_content = stream_result.text;
thinking_content = stream_result.thinking;
// ... etc
```

### Phase 2: Add Tool Execution Spans

**File:** `backend/crates/qbit-ai/src/tool_executors.rs` (or inline in agentic_loop.rs)

#### Step 2.1: Create instrumented tool execution wrapper

```rust
/// Execute a tool with OpenTelemetry instrumentation.
#[tracing::instrument(
    target = "qbit::tools",
    name = "execute_tool",
    skip_all,
    fields(
        // LangSmith span kind
        "langsmith.span.kind" = "tool",
        // GenAI semantic conventions
        "gen_ai.operation.name" = "execute_tool",
        "gen_ai.tool.name" = %tool_name,
        "gen_ai.tool.call.id" = %tool_call_id,
        "gen_ai.tool.type" = "function",
        // Result fields
        "tool.success" = tracing::field::Empty,
        "tool.error" = tracing::field::Empty,
    )
)]
async fn execute_tool_instrumented(
    tool_name: &str,
    tool_call_id: &str,
    args: &serde_json::Value,
    ctx: &AgenticLoopContext<'_>,
    // ... other params
) -> Result<ToolExecutionResult> {
    let span = tracing::Span::current();

    // Log input (truncated for safety)
    tracing::debug!(
        tool.input = %truncate_json(args, 1000),
        "Executing tool"
    );

    // Execute tool
    let result = match tool_name {
        "read_file" => execute_read_file(args, ctx).await,
        "write_file" => execute_write_file(args, ctx).await,
        // ... other tools
    };

    // Record result
    match &result {
        Ok(r) => {
            span.record("tool.success", true);
            tracing::debug!(
                tool.output = %truncate_json(&r.value, 1000),
                "Tool completed successfully"
            );
        }
        Err(e) => {
            span.record("tool.success", false);
            span.record("tool.error", e.to_string().as_str());
        }
    }

    result
}
```

#### Step 2.2: Update tool execution loop

In `run_agentic_loop_unified`, wrap each tool call:

```rust
for tool_call in tool_calls_to_execute {
    let result = execute_tool_instrumented(
        &tool_call.function.name,
        &tool_call.id,
        &tool_call.function.arguments,
        ctx,
        // ...
    ).await;

    // ... handle result
}
```

### Phase 3: Add Session/Thread Tracing

**File:** `backend/crates/qbit-ai/src/agent_bridge.rs` (or wherever sessions are initiated)

Wrap the entire agent session with a root span that includes the session ID:

```rust
/// Run an agent session with full tracing.
///
/// This creates a root span with the session ID so all nested traces
/// are grouped together in LangSmith.
#[tracing::instrument(
    target = "qbit::agent",
    name = "agent_session",
    skip_all,
    fields(
        "langsmith.span.kind" = "chain",
        "langsmith.trace.session_id" = %session_id,
        "langsmith.trace.session_name" = %session_name,
        "agent.workspace" = %workspace_path,
    )
)]
pub async fn run_agent_session(
    session_id: &str,
    session_name: &str,
    workspace_path: &str,
    // ... other params
) -> Result<AgentResponse> {
    // All nested spans (agent_turn, chat, execute_tool) will be
    // grouped under this session in LangSmith
    run_agentic_loop_unified(...).await
}
```

The `session_id` should come from:
- Qbit's existing session system (`session-qbit-{timestamp}`)
- Or a conversation ID if implementing multi-turn within a single process

### Phase 4: Add Agent Turn Span (Optional)

Wrap each iteration of the agentic loop:

```rust
#[tracing::instrument(
    target = "qbit::agent",
    name = "agent_turn",
    skip_all,
    fields(
        "langsmith.span.kind" = "chain",
        "agent.iteration" = %iteration,
        "agent.has_tool_calls" = tracing::field::Empty,
        "agent.tool_count" = tracing::field::Empty,
    )
)]
async fn execute_agent_turn(...) -> Result<TurnResult> {
    // LLM call
    let llm_result = stream_llm_completion(...).await?;

    // Tool execution
    for tool in llm_result.tool_calls {
        execute_tool_instrumented(...).await?;
    }

    // Record summary
    let span = tracing::Span::current();
    span.record("agent.has_tool_calls", !llm_result.tool_calls.is_empty());
    span.record("agent.tool_count", llm_result.tool_calls.len() as i64);

    Ok(result)
}
```

## Expected Trace Hierarchy

```
agent_session (chain) - session_id: "abc123"
│   langsmith.trace.session_id = "abc123"
│   langsmith.trace.session_name = "User conversation"
│
├── agent_turn (chain) - iteration 1
│   ├── chat claude-opus-4 (llm)
│   │   ├── Input: user message
│   │   ├── Output: response + tool calls
│   │   └── Tokens: 1000 in, 500 out
│   ├── execute_tool read_file (tool)
│   │   ├── Input: {"path": "/foo/bar.rs"}
│   │   └── Output: {file contents}
│   └── execute_tool write_file (tool)
│       ├── Input: {"path": "/foo/bar.rs", "content": "..."}
│       └── Output: {"success": true}
│
└── agent_turn (chain) - iteration 2
    ├── chat claude-opus-4 (llm)
    │   └── ...
    └── (no tool calls - final response)
```

In LangSmith UI, all traces with the same `session_id` are grouped together, allowing you to view the entire conversation history.

## Migration Steps

### Phase 1: LLM Streaming (Priority: High)
1. **Create `StreamCompletionResult` struct** - holds text, thinking, tool_calls, usage
2. **Extract `stream_llm_completion` function** with `#[tracing::instrument]`
3. **Move chunk processing logic** (~300 lines) into extracted function
4. **Update `run_agentic_loop_unified`** to call extracted function
5. **Remove old span code** (manual `info_span!` + `enter()` + `llm_span.record()`)

### Phase 2: Tool Execution (Priority: High)
6. **Create `execute_tool_instrumented` wrapper** with proper GenAI attributes
7. **Update tool execution loop** to use instrumented wrapper

### Phase 3: Session Tracing (Priority: Medium)
8. **Add session span to `agent_bridge.rs`** - wrap agent runs with session_id
9. **Pass session_id through** from Qbit's session system

### Phase 4: Agent Turns (Priority: Low)
10. **Optionally add agent_turn spans** - wrap each loop iteration

### Verification
11. **Update telemetry filter directives**
12. **Test with LangSmith** to verify:
    - Traces grouped by session_id
    - LLM spans show tokens and timing
    - Tool spans show input/output
    - Proper parent-child relationships

## Files to Modify

| File | Changes |
|------|---------|
| `backend/crates/qbit-tracing/` | **NEW CRATE** - GenAI constants, helpers, types |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Extract `stream_llm_completion`, use `qbit_tracing::prelude::*` |
| `backend/crates/qbit-ai/src/tool_executors.rs` | Add `execute_tool_instrumented` wrapper |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Add session root span with `langsmith.trace.session_id` |
| `backend/crates/qbit/src/telemetry.rs` | Add filter directives for new targets |
| `backend/crates/qbit-ai/Cargo.toml` | Add `qbit-tracing` dependency |

## New Crate: qbit-tracing

A dedicated crate for OpenTelemetry/LangSmith tracing infrastructure:

```
backend/crates/qbit-tracing/
├── Cargo.toml
└── src/
    ├── lib.rs          # Main exports, prelude
    ├── attributes.rs   # GenAI, LangSmith, OTel constants
    ├── helpers.rs      # Truncation, recording utilities
    └── types.rs        # StreamCompletionResult, ToolExecutionResult, SessionConfig
```

### Usage

```rust
use qbit_tracing::prelude::*;

#[tracing::instrument(
    target = gen_ai::TARGET_LLM,
    name = "chat",
    skip_all,
    fields(
        langsmith::SPAN_KIND = langsmith::KIND_LLM,
        gen_ai::OPERATION_NAME = gen_ai::OP_CHAT,
        gen_ai::PROVIDER_NAME = %provider,
        gen_ai::REQUEST_MODEL = %model,
        gen_ai::USAGE_INPUT_TOKENS = Empty,
        gen_ai::USAGE_OUTPUT_TOKENS = Empty,
    ),
    err,
)]
async fn stream_llm_completion(...) -> Result<StreamCompletionResult<ToolCall>> {
    // ... streaming logic ...

    // Record using helpers
    record_token_usage(result.usage.input_tokens, result.usage.output_tokens);
    record_truncated(gen_ai::OUTPUT_MESSAGES, &result.text, DEFAULT_MAX_STRING_SIZE);

    Ok(result)
}
```

## Telemetry Filter Update

```rust
// In telemetry.rs
if langsmith_config.is_some() {
    for directive in &[
        "rig=info",           // Provider-level spans
        "qbit::llm=info",     // LLM completion spans
        "qbit::tools=info",   // Tool execution spans
        "qbit::agent=info",   // Agent session/turn spans
    ] {
        if let Ok(d) = directive.parse() {
            filter = filter.add_directive(d);
        }
    }
}
```

## Testing Checklist

- [ ] **Session grouping**: Multiple agent runs with same session_id appear together in LangSmith
- [ ] **LLM spans**: `chat {model}` spans show input_tokens, output_tokens, timing
- [ ] **Tool spans**: `execute_tool {name}` spans show input, output, success/error
- [ ] **Hierarchy**: Tool spans are children of agent_turn, which are children of agent_session
- [ ] **Error capture**: Failed tool calls show error details via `err` attribute
- [ ] **No duplicates**: Provider-level `chat_streaming` spans properly nested (not duplicated)
- [ ] **Performance**: No noticeable latency increase from instrumentation

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Large inputs/outputs exceed attribute limits | Truncate to 10KB max |
| Sensitive data in traces | Use opt-in content recording, truncate by default |
| Performance impact from instrumentation | Batch exports (already configured), sample if needed |
| Breaking changes to streaming logic | Extract to separate function first, comprehensive testing |
| Span not closed on error | `#[instrument(err)]` handles this automatically |
| Session ID not propagated | Pass through from Qbit session system |

## Success Criteria

- [ ] **Sessions**: Traces grouped by `langsmith.trace.session_id` in LangSmith UI
- [ ] **LLM**: Spans show model, provider, input/output tokens, timing
- [ ] **Tools**: Spans show name, call_id, input args, output result, success/error
- [ ] **Hierarchy**: Proper parent-child: session → turn → (llm + tools)
- [ ] **Async safety**: No manual span management, all via `#[instrument]`
- [ ] **Standards compliant**: Using OTel GenAI semantic conventions
