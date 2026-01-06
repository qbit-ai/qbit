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
enabled = true
api_key = "$LANGSMITH_API_KEY"
project = "my-qbit-agent"

# Optional: EU region endpoint
# endpoint = "https://eu.api.smith.langchain.com"

# Optional: Sampling ratio for high-traffic deployments (0.0 to 1.0)
# sampling_ratio = 0.1
```

### Environment Variables

Alternatively, configure via environment variables:

```bash
export LANGSMITH_API_KEY="lsv2_sk_..."
export LANGSMITH_PROJECT="my-qbit-agent"  # optional
```

## Getting Started

1. **Sign up for LangSmith** at [smith.langchain.com](https://smith.langchain.com)

2. **Create an API key** in Settings > API Keys

3. **Configure Qbit** using either:
   - Settings file: Add `[telemetry.langsmith]` section as shown above
   - Environment variable: `export LANGSMITH_API_KEY="lsv2_sk_..."`

4. **Run Qbit** - traces will automatically be exported to LangSmith

5. **View traces** in the LangSmith dashboard under your project

## Sampling

For production deployments with high traffic, configure sampling to reduce trace volume:

```toml
[telemetry.langsmith]
enabled = true
sampling_ratio = 0.1  # Sample 10% of traces
```

| Value | Behavior |
|-------|----------|
| `1.0` | Sample everything (default) |
| `0.1` | Sample 10% of traces |
| `0.0` | Disable tracing |

## Troubleshooting

### Traces not appearing in LangSmith

1. **Check API key format**: Should start with `lsv2_sk_`
2. **Verify endpoint**: US uses `api.smith.langchain.com`, EU uses `eu.api.smith.langchain.com`
3. **Enable debug logging**: Run with `RUST_LOG=debug` to see export errors

### High latency impact

If tracing adds noticeable latency, reduce the sampling ratio or check network connectivity to LangSmith.

## References

- [LangSmith Documentation](https://docs.smith.langchain.com/)
- [LangSmith OTEL Integration](https://docs.smith.langchain.com/observability/how_to_guides/tracing/trace_with_opentelemetry)
