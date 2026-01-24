# Configuration

Settings are stored in `~/.qbit/settings.toml` (auto-generated on first run).

Most settings can also be configured through the in-app Settings UI.

## LLM Providers

Qbit supports multiple LLM providers. Configure via environment variables or settings file.

### Anthropic (Vertex AI)

**Option A: Application Default Credentials (recommended for development)**

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

**Option B: Service Account (recommended for production)**

```toml
[ai.vertex_ai]
credentials_path = "/path/to/service-account.json"
project_id = "your-project-id"
location = "us-east5"
```

Or via environment:
```bash
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
VERTEX_AI_PROJECT_ID=your-project-id
```

### Anthropic (Direct API)

```bash
echo "ANTHROPIC_API_KEY=sk-ant-..." >> .env
```

Or in settings:
```toml
[ai]
default_provider = "anthropic"
```

### OpenAI

```bash
echo "OPENAI_API_KEY=sk-..." >> .env
```

For reasoning models (`o1*`, `o3*`, `gpt-5*`), optionally configure:
```toml
[ai]
default_reasoning_effort = "medium" # low | medium | high
```

### OpenRouter

```bash
echo "OPENROUTER_API_KEY=sk-or-..." >> .env
```

### Google Gemini

```bash
echo "GEMINI_API_KEY=..." >> .env
```

### Groq

```bash
echo "GROQ_API_KEY=gsk_..." >> .env
```

### xAI (Grok)

```bash
echo "XAI_API_KEY=..." >> .env
```

### Z.AI (GLM)

```bash
echo "ZAI_API_KEY=..." >> .env
```

### Ollama (Local)

No API key needed. Just have Ollama running locally.

```toml
[ai]
default_provider = "ollama"
```

## Web Search

Enable Tavily-powered web search:

```bash
echo "TAVILY_API_KEY=tvly-..." >> .env
```

## Environment Variables

Create `.env` in project root for development:

```bash
# LLM Providers (use one)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-...
GEMINI_API_KEY=...
GROQ_API_KEY=gsk_...
XAI_API_KEY=...
ZAI_API_KEY=...

# Vertex AI
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
VERTEX_AI_PROJECT_ID=your-project-id

# Web Search
TAVILY_API_KEY=tvly-...
```

## File Locations

| Path | Purpose |
|------|---------|
| `~/.qbit/settings.toml` | Main configuration file |
| `~/.qbit/sessions/` | Session storage (override with `VT_SESSION_DIR`) |
| `~/.qbit/frontend.log` | Frontend logs |
| `~/.qbit/backend.log` | Backend logs |
| `~/.qbit/skills/` | Global agent skills |
| `~/.qbit/prompts/` | Global prompts |
| `~/.qbit/artifacts/` | Generated artifacts (compaction, summaries) |
| `~/.qbit/transcripts/` | Conversation transcripts |

## Context Compaction

Configure the summarizer model:

```toml
[ai]
summarizer_model = "claude-3-5-haiku-latest"  # Optional
```

## Terminal Settings

```toml
[terminal]
# Additional commands that should trigger fullterm mode
fullterm_commands = ["my-custom-tui", "another-app"]
```

Built-in fullterm commands: claude, cc, codex, cdx, aider, cursor, gemini

## Workspace Override

Override the working directory:

```bash
just dev /path/to/project
# or
QBIT_WORKSPACE=/path/to/project just dev
```
