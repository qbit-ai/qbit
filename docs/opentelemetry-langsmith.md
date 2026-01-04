# OpenTelemetry & LangSmith Integration

This document explains how to set up OpenTelemetry tracing with LangSmith for observability of Qbit's LLM interactions and agent behavior.

## Overview

Qbit uses OpenTelemetry (OTEL) to export traces to LangSmith, providing visibility into:

- LLM API calls (prompts, completions, token usage)
- Tool executions (file operations, shell commands, web searches)
- Agent workflow steps and decision points
- Performance metrics and latencies

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Qbit Application                         │
│  ┌─────────────────┐    ┌─────────────────┐    ┌─────────────┐  │
│  │  tracing crate  │───▶│ tracing-otel    │───▶│ OTEL SDK    │  │
│  │  (spans, logs)  │    │ (bridge layer)  │    │ (batching)  │  │
│  └─────────────────┘    └─────────────────┘    └──────┬──────┘  │
└──────────────────────────────────────────────────────│──────────┘
                                                       │
                                                       │ OTLP/HTTP
                                                       ▼
                                        ┌──────────────────────────┐
                                        │  LangSmith API           │
                                        │  /otel/v1/traces         │
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
export LANGSMITH_API_KEY="lsv2_sk_..."
export LANGSMITH_PROJECT="my-qbit-agent"  # optional
export LANGSMITH_ENDPOINT="https://api.smith.langchain.com"  # optional
```

## Getting Started

1. **Sign up for LangSmith** at [smith.langchain.com](https://smith.langchain.com)

2. **Create an API key** in Settings > API Keys

3. **Configure Qbit** using either:
   - Settings file: Add `[telemetry.langsmith]` section as shown above
   - Environment variable: `export LANGSMITH_API_KEY="lsv2_sk_..."`

4. **Enable tracing** by setting `enabled = true` in settings

5. **Run Qbit** - traces will automatically be exported to LangSmith

6. **View traces** in the LangSmith dashboard under your project

## Implementation Details

### Crate Dependencies

The integration uses these OpenTelemetry crates:

| Crate | Version | Purpose |
|-------|---------|---------|
| `opentelemetry` | 0.27 | Core OTEL API |
| `opentelemetry_sdk` | 0.27 | OTEL SDK with Tokio runtime |
| `opentelemetry-otlp` | 0.27 | OTLP exporter (HTTP) |
| `opentelemetry-semantic-conventions` | 0.27 | Standard attribute names |
| `tracing-opentelemetry` | 0.28 | Bridge between `tracing` and OTEL |

### Key Files

| File | Description |
|------|-------------|
| `backend/crates/qbit/src/telemetry.rs` | Core telemetry module |
| `backend/crates/qbit-settings/src/schema.rs` | `LangSmithSettings` struct |
| `backend/crates/qbit/src/lib.rs` | Tauri entry point initialization |
| `backend/crates/qbit/src/cli/bootstrap.rs` | CLI entry point initialization |

### Initialization Flow

1. **Load settings** from `~/.qbit/settings.toml`
2. **Create LangSmithConfig** from settings (if enabled)
3. **Initialize OTEL exporter** with LangSmith endpoint and API key
4. **Set up tracing subscriber** with both console and OTEL layers
5. **Return TelemetryGuard** to keep tracer alive for app lifetime

### OTLP Export Configuration

- **Protocol**: OTLP/HTTP (not gRPC)
- **Endpoint**: `{base_url}/otel/v1/traces`
- **Authentication**: `x-api-key` header with LangSmith API key
- **Batching**: Enabled via `with_batch_exporter` for performance
- **Resource attributes**:
  - `service.name`: "qbit"
  - `service.version`: Package version
  - `langsmith.project`: Configured project name

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

1. **Check API key format**: Should start with `lsv2_sk_` for secret keys
2. **Verify endpoint**: US uses `api.smith.langchain.com`, EU uses `eu.api.smith.langchain.com`
3. **Check project name**: Ensure the project exists in LangSmith
4. **Enable debug logging**: Run with `RUST_LOG=debug` to see export errors

### High latency impact

If tracing adds noticeable latency:
1. Reduce sampling ratio
2. Check network connectivity to LangSmith
3. Consider using EU endpoint if closer to your region

### Memory usage

The OTEL SDK batches spans before export. For long-running sessions:
- Spans are batched and exported periodically
- Memory is released after successful export
- Consider lower sampling ratio if memory is a concern

## Future Enhancements

Potential improvements for the integration:

1. **GenAI semantic conventions**: Add attributes following OTEL GenAI conventions
   - `gen_ai.request.model`
   - `gen_ai.usage.input_tokens`
   - `gen_ai.usage.output_tokens`

2. **Custom spans**: Instrument key code paths:
   - Agent turns
   - Tool executions
   - Context pruning operations

3. **Metrics export**: Add OTEL metrics for:
   - Token usage over time
   - Tool execution counts
   - Error rates

## References

- [LangSmith Documentation](https://docs.smith.langchain.com/)
- [LangSmith OTEL Integration](https://docs.smith.langchain.com/observability/how_to_guides/tracing/trace_with_opentelemetry)
- [OpenTelemetry Rust](https://opentelemetry.io/docs/languages/rust/)
- [OTEL GenAI Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/gen-ai/)
