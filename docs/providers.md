# Providers

Qbit supports multiple LLM providers and can switch providers mid-session.

## Provider matrix

| Provider | Configuration |
|----------|---------------|
| Anthropic (Vertex AI) | `gcloud auth` or service account JSON |
| Anthropic (Direct API) | `ANTHROPIC_API_KEY` |
| OpenAI | `OPENAI_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq | `GROQ_API_KEY` |
| xAI (Grok) | `XAI_API_KEY` |
| Z.AI (GLM) | `ZAI_API_KEY` |
| Ollama | Local server (no API key needed) |

## OpenAI reasoning models

OpenAI reasoning models (e.g. `o1*`, `o3*`, `gpt-5*`) are auto-detected and routed through a dedicated Responses API adapter that keeps reasoning streaming deltas separate from text.

Optional configuration:

```toml
[ai]
default_reasoning_effort = "medium" # low | medium | high
```

## Vertex AI setup

### Option A: Application Default Credentials (recommended for development)

```bash
gcloud auth application-default login
```

Then add to `~/.qbit/settings.toml`:

```toml
[ai]
default_provider = "vertex_ai"

[ai.vertex_ai]
project_id = "your-project-id"
location = "us-east5"
```

### Option B: Service account (recommended for production)

```toml
[ai.vertex_ai]
credentials_path = "/path/to/service-account.json"
project_id = "your-project-id"
location = "us-east5"
```
