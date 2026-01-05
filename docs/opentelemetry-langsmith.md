# OpenTelemetry & LangSmith Integration

This document explains how to set up OpenTelemetry tracing with LangSmith for observability of Qbit's LLM interactions and agent behavior.

## Overview

Qbit uses OpenTelemetry (OTEL) to export traces to LangSmith, providing visibility into:

- LLM API calls (model, prompts, completions, token usage)
- Agent workflow steps and decision points
- Performance metrics and latencies

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Qbit Application                         │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────┐  │
│  │  tracing crate  │───▶│ tracing-otel    │───▶│ OTEL SDK    │  │
│  │  (info_span!)   │    │ (bridge layer)  │    │ (BatchProc) │  │
│  └─────────────────┘    └─────────────────┘    └──────┬──────┘  │
└──────────────────────────────────────────────────────│──────────┘
                                                       │
                                                       │ OTLP/HTTP
                                                       │ (reqwest + tokio)
                                                       ▼
                                        ┌──────────────────────────┐
                                        │  LangSmith API           │
                                        │  /otel/v1/traces         │
                                        │                          │
                                        │  Headers:                │
                                        │  - x-api-key             │
                                        │  - langsmith-project     │
                                        └──────────────────────────┘
```

## Configuration

### Settings File

Add the following to `~/.qbit/settings.toml`:

```toml
[telemetry.langsmith]
# Enable LangSmith tracing
enabled = true

# LangSmith API key (or use $LANGSMITH_API_KEY env var)
api_key = "$LANGSMITH_API_KEY"

# Project name in LangSmith (defaults to "default")
project = "my-qbit-agent"

# Optional: LangSmith API endpoint
# Default: https://api.smith.langchain.com (US)
# EU region: https://eu.api.smith.langchain.com
# endpoint = "https://api.smith.langchain.com"

# Optional: Sampling ratio (0.0 to 1.0)
# Default: 1.0 (sample everything)
# Use lower values for high-traffic production deployments
# sampling_ratio = 1.0
```

### Environment Variables

Alternatively, configure via environment variables:

```bash
export LANGSMITH_API_KEY="lsv2_pt_..."
export LANGSMITH_PROJECT="my-qbit-agent"  # optional
export LANGSMITH_ENDPOINT="https://api.smith.langchain.com"  # optional
```

## Getting Started

1. **Sign up for LangSmith** at [smith.langchain.com](https://smith.langchain.com)

2. **Create an API key** in Settings > API Keys

3. **Configure Qbit** using either:
   - Settings file: Add `[telemetry.langsmith]` section as shown above
   - Environment variable: `export LANGSMITH_API_KEY="lsv2_pt_..."`

4. **Enable tracing** by setting `enabled = true` in settings

5. **Run Qbit** - traces will automatically be exported to LangSmith

6. **View traces** in the LangSmith dashboard under your project

## Current Implementation

### What's Traced

LLM calls via `rig-anthropic-vertex` are traced with GenAI semantic conventions:

| Attribute | Description |
|-----------|-------------|
| `langsmith.span.kind` | Set to "LLM" for LangSmith to recognize as an LLM call |
| `gen_ai.system` | Provider name ("anthropic") |
| `gen_ai.operation.name` | Operation type ("chat") |
| `gen_ai.request.model` | Model ID (e.g., "claude-opus-4-5@20251101") |
| `gen_ai.prompt` | The user's input message |

### Span Types

| Span Name | Description |
|-----------|-------------|
| `chat_streaming` | Streaming LLM completion request |
| `chat` | Non-streaming LLM completion request |

### Current Limitations

**Output and token usage not captured**: The `chat_streaming` span ends when the stream is *created* (returned from the provider), not when it's fully *consumed*. This means:

- `gen_ai.completion` (output text) is not recorded
- `gen_ai.usage.input_tokens` is not recorded
- `gen_ai.usage.output_tokens` is not recorded

To capture these, instrumentation needs to be added where the stream is consumed (in the agentic loop).

## Implementation Details

### Crate Dependencies

The integration uses these OpenTelemetry crates:

| Crate | Version | Purpose |
|-------|---------|---------|
| `opentelemetry` | 0.27 | Core OTEL API |
| `opentelemetry_sdk` | 0.27 | OTEL SDK with Tokio runtime |
| `opentelemetry-otlp` | 0.27 | OTLP HTTP exporter (reqwest) |
| `opentelemetry-semantic-conventions` | 0.27 | Standard attribute names |
| `tracing-opentelemetry` | 0.28 | Bridge between `tracing` and OTEL |

### Key Files

| File | Description |
|------|-------------|
| `backend/crates/qbit/src/telemetry.rs` | Core telemetry module, OTEL setup |
| `backend/crates/qbit-settings/src/schema.rs` | `LangSmithSettings` struct |
| `backend/crates/qbit/src/lib.rs` | Tauri entry point initialization |
| `backend/crates/rig-anthropic-vertex/src/completion.rs` | LLM provider with GenAI spans |

### Initialization Flow

1. **Load settings** from `~/.qbit/settings.toml` (inside tokio runtime)
2. **Create LangSmithConfig** from settings (if enabled)
3. **Build OTLP HTTP exporter** with LangSmith endpoint and headers
4. **Create BatchSpanProcessor** with tokio runtime (required for reqwest async HTTP)
5. **Set up TracerProvider** with sampler and resource attributes
6. **Create OpenTelemetryLayer** and register with tracing subscriber
7. **Return TelemetryGuard** to keep tracer alive for app lifetime

### OTLP Export Configuration

- **Protocol**: OTLP/HTTP (not gRPC) via reqwest
- **Endpoint**: `{base_url}/otel/v1/traces`
- **Authentication**: `x-api-key` header with LangSmith API key
- **Project routing**: `langsmith-project` header (lowercase)
- **Batch processor**:
  - Max queue size: 2048 spans
  - Scheduled delay: 1 second
  - Max batch size: 512 spans
  - Runtime: Tokio (required for reqwest)
- **Resource attributes**:
  - `service.name`: "qbit"
  - `service.version`: Package version

### Why BatchSpanProcessor with Tokio?

The OTEL SDK provides two span processors:

1. **SimpleSpanProcessor**: Exports synchronously using `futures_executor::block_on`
2. **BatchSpanProcessor**: Exports asynchronously with a configurable runtime

We use `BatchSpanProcessor` with `runtime::Tokio` because:
- The OTLP exporter uses `reqwest` with tokio features
- `reqwest`'s async HTTP client requires a tokio runtime context
- `SimpleSpanProcessor`'s `futures_executor::block_on` doesn't provide this context
- `BatchSpanProcessor` properly spawns export tasks on the tokio runtime

### Tracing Filter

When LangSmith is enabled, the tracing filter includes:
- `qbit={log_level}` - Main application logs
- `rig=info` - LLM provider spans (required for `chat_streaming` spans)

## Sampling

For production deployments with high traffic, configure sampling to reduce trace volume:

```toml
[telemetry.langsmith]
enabled = true
sampling_ratio = 0.1  # Sample 10% of traces
```

Sampling options:
- `1.0` - Sample everything (default, good for development)
- `0.5` - Sample 50% of traces
- `0.1` - Sample 10% of traces (good for production)
- `0.0` - Disable sampling (no traces exported)

## Troubleshooting

### Traces not appearing in LangSmith

1. **Check API key**: Should start with `lsv2_pt_` for personal tokens
2. **Verify endpoint**: US uses `api.smith.langchain.com`, EU uses `eu.api.smith.langchain.com`
3. **Check project name**: Ensure the project exists or use "default"
4. **Check for 401 errors**: Invalid API key will show in logs as `BatchSpanProcessor.Flush.ExportError`
5. **Enable debug logging**: Add to settings to see OTEL activity:
   ```rust
   // In telemetry.rs, add to filter directives:
   "opentelemetry=debug",
   "opentelemetry_sdk=debug",
   "reqwest=debug",
   ```

### LLM spans not appearing

1. **Check filter includes `rig=info`**: Required for spans with target `rig::completions`
2. **Verify provider is instrumented**: Check `rig-anthropic-vertex/src/completion.rs` has span instrumentation
3. **Ensure async instrumentation**: Use `.instrument(span).await`, not `span.enter()` for async code

### High latency impact

If tracing adds noticeable latency:
1. Reduce sampling ratio
2. Increase batch scheduled delay
3. Check network connectivity to LangSmith

## Future Enhancements

### Phase 1: Full Request/Response Tracing

To capture output and token usage, instrument the stream consumption point in the agentic loop:

1. Create a parent span in `agentic_loop.rs` for the full LLM turn
2. Record accumulated response text after streaming completes
3. Record token usage from the final `MessageDelta` event

### Phase 2: Additional Instrumentation

- Tool execution spans with inputs/outputs
- Agent turn spans showing the full agentic loop
- Context pruning operations
- Sub-agent invocations

### Phase 3: Metrics Export

Add OTEL metrics for:
- Token usage over time
- Tool execution counts
- Error rates
- Response latencies

## References

- [LangSmith Documentation](https://docs.smith.langchain.com/)
- [LangSmith OTEL Integration](https://docs.smith.langchain.com/observability/how_to_guides/tracing/trace_with_opentelemetry)
- [OpenTelemetry Rust](https://opentelemetry.io/docs/languages/rust/)
- [OTEL GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/)
