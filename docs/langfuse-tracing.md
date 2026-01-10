# Langfuse Tracing Integration

Qbit integrates with [Langfuse](https://langfuse.com) for LLM observability using OpenTelemetry (OTel). This provides visibility into agent behavior, token usage, tool execution, and context management.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Qbit Application                          │
│                                                                  │
│  ┌──────────────┐    ┌──────────────────┐    ┌───────────────┐  │
│  │  tracing     │───▶│ tracing-otel     │───▶│ OTel SDK      │  │
│  │  macros      │    │ layer            │    │ (0.31)        │  │
│  └──────────────┘    └──────────────────┘    └───────┬───────┘  │
│                                                       │          │
└───────────────────────────────────────────────────────┼──────────┘
                                                        │
                                                        ▼
                                          ┌─────────────────────┐
                                          │ opentelemetry-      │
                                          │ langfuse exporter   │
                                          │ (BatchSpanProcessor)│
                                          └──────────┬──────────┘
                                                     │
                                                     ▼ HTTPS
                                          ┌─────────────────────┐
                                          │   Langfuse Cloud    │
                                          │ (or self-hosted)    │
                                          └─────────────────────┘
```

## Configuration

### Via Settings File

Add to `~/.qbit/settings.toml`:

```toml
[telemetry.langfuse]
enabled = true

# Langfuse host (defaults to https://cloud.langfuse.com)
# host = "https://cloud.langfuse.com"

# Langfuse public key (or use $LANGFUSE_PUBLIC_KEY env var)
public_key = "$LANGFUSE_PUBLIC_KEY"

# Langfuse secret key (or use $LANGFUSE_SECRET_KEY env var)
secret_key = "$LANGFUSE_SECRET_KEY"

# Sampling ratio for traces (0.0 to 1.0, default 1.0 = sample everything)
# sampling_ratio = 1.0
```

### Via Environment Variables

```bash
export LANGFUSE_PUBLIC_KEY="pk-lf-..."
export LANGFUSE_SECRET_KEY="sk-lf-..."
export LANGFUSE_HOST="https://cloud.langfuse.com"  # optional
```

## Trace Hierarchy

Langfuse organizes data into Sessions, Traces, and Observations. Qbit maps its agentic loop to this hierarchy:

```
Session (conversation)
  └── chat_message (Trace)
        └── agent (Observation: type=agent)
              ├── context_pruned (Observation: type=event)
              ├── max_iterations_reached (Observation: type=event)
              ├── llm_completion (Observation: type=generation)
              │     ├── loop_blocked (Observation: type=event)
              │     ├── read_file (Observation: type=tool)
              │     └── write_file (Observation: type=tool)
              └── llm_completion (Observation: type=generation)
                    └── bash (Observation: type=tool)
```

### Observation Types

| Type | Span Name | Description |
|------|-----------|-------------|
| **trace** | `chat_message` | Root span representing one user message → agent response cycle |
| **agent** | `agent` | The agentic loop execution with model/provider info |
| **generation** | `llm_completion` | Individual LLM API calls with token usage |
| **tool** | `{tool_name}` | Tool executions (read_file, write_file, bash, etc.) |
| **event** | Various | Point-in-time events (context_pruned, loop_blocked, etc.) |

## Span Attributes

### chat_message (Trace)

| Attribute | Description |
|-----------|-------------|
| `langfuse.session.id` | Session ID for grouping traces |
| `langfuse.observation.input` | User's input message (truncated to 2000 chars) |
| `langfuse.observation.output` | Agent's final response (truncated to 2000 chars) |

### agent

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"agent"` |
| `langfuse.session.id` | Session ID |
| `agent_type` | Agent label (e.g., "main", "sub-agent-name") |
| `model` | Model name (e.g., "claude-sonnet-4-20250514") |
| `provider` | Provider name (e.g., "anthropic", "vertex-ai") |
| `langfuse.observation.input` | User's input message |
| `langfuse.observation.output` | Agent's final response |

### llm_completion (Generation)

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"generation"` |
| `gen_ai.operation.name` | `"chat_completion"` |
| `gen_ai.request.model` | Model name |
| `gen_ai.system` | Provider name |
| `gen_ai.request.temperature` | Temperature setting |
| `gen_ai.request.max_tokens` | Max completion tokens |
| `gen_ai.usage.prompt_tokens` | Input token count |
| `gen_ai.usage.completion_tokens` | Output token count |
| `gen_ai.prompt` | Prompt summary (truncated) |
| `gen_ai.completion` | Completion text or "[N tool call(s)]" |
| `langfuse.observation.input` | Same as gen_ai.prompt |
| `langfuse.observation.output` | Same as gen_ai.completion |
| `iteration` | Loop iteration number |

### Tool Observations

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"tool"` |
| `otel.name` | Tool name (for display) |
| `langfuse.span.name` | Tool name (for display) |
| `tool.name` | Tool name |
| `tool.id` | Tool call ID |
| `langfuse.observation.input` | Tool arguments (JSON, truncated to 1000 chars) |
| `langfuse.observation.output` | Tool result (truncated) |
| `success` | Boolean success status |

### Event Observations

#### context_pruned

Emitted when the context window is compacted to fit within token limits.

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"event"` |
| `messages_removed` | Number of messages pruned |
| `utilization_before` | Context utilization before pruning (e.g., "95.2%") |
| `utilization_after` | Context utilization after pruning (e.g., "72.1%") |

#### max_iterations_reached

Emitted when the agent hits the maximum tool iteration limit.

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"event"` |
| `max_iterations` | The iteration limit that was reached |

#### loop_blocked

Emitted when the loop detector blocks a repetitive tool call.

| Attribute | Description |
|-----------|-------------|
| `langfuse.observation.type` | `"event"` |
| `tool_name` | Name of the blocked tool |
| `details` | Loop detection details (repeat_count, max) |

## Implementation Details

### Async Span Handling

Rust's `tracing` crate requires special handling for async code. The standard `span.enter()` pattern doesn't work across `.await` points. Instead, Qbit uses explicit parent relationships:

```rust
// Create parent span
let agent_span = tracing::info_span!("agent", ...);

// Create child with explicit parent (works across .await)
let llm_span = tracing::info_span!(
    parent: &agent_span,
    "llm_completion",
    ...
);

// Record values on span (works without entering)
llm_span.record("gen_ai.usage.prompt_tokens", token_count);
```

### Batch Processing

Spans are exported using `BatchSpanProcessor` with the Tokio async runtime. This batches spans for efficient network transmission and ensures proper flushing on shutdown via `TelemetryGuard`.

### Sampling

Configurable via `sampling_ratio`:
- `1.0` (default): Sample all traces
- `0.5`: Sample 50% of traces
- `0.0`: Disable sampling (no traces exported)

## Crate Dependencies

```toml
[dependencies]
opentelemetry = { version = "0.31", features = ["trace"] }
opentelemetry_sdk = { version = "0.31", features = ["rt-tokio"] }
opentelemetry-langfuse = "0.6"
tracing = "0.1"
tracing-opentelemetry = "0.30"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Troubleshooting

### Traces not appearing in Langfuse

1. Verify credentials are set correctly:
   ```bash
   echo $LANGFUSE_PUBLIC_KEY
   echo $LANGFUSE_SECRET_KEY
   ```

2. Check that `enabled = true` in settings.toml

3. Look for initialization logs:
   ```
   INFO Langfuse tracing enabled langfuse_host=https://cloud.langfuse.com
   ```

4. Ensure the application exits cleanly (TelemetryGuard flush)

### Traces showing separately instead of grouped

Verify parent-child relationships are set correctly using `parent: &span` syntax. Spans without explicit parents become separate traces.

### Missing token usage

Token usage is recorded after the LLM response is received. If a request fails or is cancelled, usage may not be recorded.

## Related Files

| File | Purpose |
|------|---------|
| `backend/crates/qbit/src/telemetry.rs` | Telemetry initialization and config |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Span instrumentation |
| `backend/crates/qbit-settings/src/schema.rs` | LangfuseSettings struct |
| `backend/crates/qbit-settings/src/template.toml` | Settings template |
