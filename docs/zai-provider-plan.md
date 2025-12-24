# Z.AI Provider Implementation Plan

## Overview

Add support for Z.AI's GLM Coding Plan API as a new LLM provider, enabling access to GLM-4.7 and GLM-4.5-air models with full thinking mode support.

## API Details

### Endpoint (OpenAI-compatible)

**Use this base URL:**
```
https://api.z.ai/api/coding/paas/v4
```

This is the Coding Plan API endpoint. The `/chat/completions` path is appended automatically by OpenAI-compatible clients, making the full URL:
```
https://api.z.ai/api/coding/paas/v4/chat/completions
```

> **Important**: Do NOT use the General API (`https://api.z.ai/api/paas/v4`). The Coding Plan endpoint is required to use GLM Coding Plan subscriptions.

### Authentication
- Bearer token via `Authorization: Bearer <ZAI_API_KEY>`
- Environment variable: `ZAI_API_KEY`

### Supported Models
| Model | Context | Description |
|-------|---------|-------------|
| `glm-4.7` | 200K | Latest flagship with best coding performance |
| `glm-4.5-air` | 200K | Lightweight, faster responses |

### Key Features
- **OpenAI-compatible**: Uses standard `/chat/completions` endpoint format
- **Thinking mode**: Native support via `thinking` parameter
- **Streaming**: Full SSE streaming support
- **Tool use**: Function calling compatible with OpenAI format

### Thinking Mode
```json
{
  "thinking": {
    "type": "enabled"  // or "disabled"
  }
}
```

Response includes:
- `reasoning_content`: Model's internal reasoning (streamed first)
- `content`: Final response text
- `tool_calls`: Any function calls

### Request Format
```json
{
  "model": "glm-4.7",
  "messages": [{"role": "user", "content": "..."}],
  "max_tokens": 4096,
  "temperature": 1.0,
  "stream": true,
  "thinking": {"type": "enabled"},
  "tools": [...]
}
```

### Streaming Response Format
```
data: {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}
data: {"choices":[{"delta":{"content":"response..."}}]}
data: {"choices":[{"delta":{"tool_calls":[...]}}]}
data: [DONE]
```

## Implementation Approach

### Option A: New Crate (Recommended)
Create `backend/crates/rig-zai/` similar to `rig-anthropic-vertex/`.

**Pros:**
- Full control over thinking mode handling
- Clean separation of concerns
- Can handle z.ai-specific response format quirks
- Better type safety for thinking mode

**Cons:**
- More code to maintain
- Duplicates some OpenAI-compatible logic

### Option B: OpenAI Provider with Custom Base URL
Use existing rig OpenAI provider with base URL override.

**Pros:**
- Less code
- Reuses existing infrastructure

**Cons:**
- Limited thinking mode support
- May miss z.ai-specific features
- rig-core doesn't currently support custom base URLs for OpenAI

**Decision**: Option A (new crate) for full thinking mode support and clean integration.

## Implementation Steps

### Phase 1: Create rig-zai Crate

#### 1.1 Create crate structure
```
backend/crates/rig-zai/
├── Cargo.toml
└── src/
    ├── lib.rs          # Public exports and model constants
    ├── client.rs       # HTTP client with auth
    ├── completion.rs   # CompletionModel implementation
    ├── streaming.rs    # SSE stream parser
    ├── types.rs        # Request/response types
    └── error.rs        # Error types
```

#### 1.2 Cargo.toml dependencies
```toml
[package]
name = "rig-zai"
version = "0.1.0"
edition = "2021"

[dependencies]
rig-core = { workspace = true }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
bytes = "1"
thiserror = "1"
tracing = "0.1"
```

#### 1.3 Key types (types.rs)
```rust
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,  // "enabled" or "disabled"
}

pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    pub thinking: Option<ThinkingConfig>,
    pub tools: Option<Vec<ToolDefinition>>,
}

// Response mirrors OpenAI format with additions
pub struct Choice {
    pub message: Option<AssistantMessage>,
    pub delta: Option<StreamDelta>,
    pub finish_reason: Option<String>,
}

pub struct StreamDelta {
    pub content: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

#### 1.4 Client implementation (client.rs)
```rust
pub struct Client {
    http_client: reqwest::Client,
    api_key: String,
    base_url: String,  // configurable for standard vs coding plan
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Self;
    pub fn with_coding_plan(api_key: impl Into<String>) -> Self;
    pub fn completion_model(&self, model: &str) -> CompletionModel;
}
```

#### 1.5 Streaming implementation (streaming.rs)
- Parse SSE `data:` lines
- Handle `reasoning_content` as ThinkingDelta
- Handle `content` as TextDelta
- Handle `tool_calls` as ToolUseStart/ToolInputDelta
- Emit `[DONE]` as completion signal

### Phase 2: Backend Integration

#### 2.1 Add to llm_client.rs
```rust
pub enum LlmClient {
    // ... existing variants ...
    RigZai(rig_zai::CompletionModel),
}

pub struct ZaiClientConfig<'a> {
    pub workspace: PathBuf,
    pub model: &'a str,
    pub api_key: &'a str,
    pub use_coding_plan: bool,  // true for coding plan endpoint
}

pub async fn create_zai_components(
    config: ZaiClientConfig<'_>,
) -> Result<AgentBridgeComponents>;
```

#### 2.2 Add to ProviderConfig enum
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "provider", rename_all = "snake_case")]
pub enum ProviderConfig {
    // ... existing variants ...
    Zai {
        workspace: String,
        model: String,
        api_key: String,
        #[serde(default)]
        use_coding_plan: bool,
    },
}
```

### Phase 3: Settings Integration

#### 3.1 Add to AiProvider enum (schema.rs)
```rust
pub enum AiProvider {
    // ... existing variants ...
    Zai,
}
```

#### 3.2 Add ZaiSettings struct
```rust
pub struct ZaiSettings {
    pub api_key: Option<String>,
    pub use_coding_plan: bool,  // default: true
    pub show_in_selector: bool,
}
```

#### 3.3 Add to AiSettings
```rust
pub struct AiSettings {
    // ... existing fields ...
    pub zai: ZaiSettings,
}
```

### Phase 4: Agentic Loop Integration

#### 4.1 Add thinking mode support
The agentic loop already handles `StreamedAssistantContent::Reasoning` via the generic loop. The z.ai crate must emit:
- `RawStreamingChoice::Reasoning` for `reasoning_content`
- `RawStreamingChoice::Message` for `content`
- `RawStreamingChoice::ToolCall` for `tool_calls`

#### 4.2 Update run_agentic_loop_generic
May need to add z.ai-specific handling if response format differs significantly from other OpenAI-compatible providers.

### Phase 5: Frontend Updates

#### 5.1 Add provider to model selector
Update frontend to recognize `zai` provider and display GLM models.

#### 5.2 Add settings UI
Add Z.AI section to settings with:
- API key input
- Coding plan toggle
- Model selection

## Testing Strategy

1. **Unit tests** for types serialization/deserialization
2. **Integration tests** with mocked HTTP responses
3. **E2E tests** with real API (optional, requires key)
4. **Streaming tests** to verify thinking mode parsing

## Configuration Example

```toml
# ~/.qbit/settings.toml
[ai]
default_provider = "zai"
default_model = "glm-4.7"

[ai.zai]
api_key = "$ZHIPU_API_KEY"
use_coding_plan = true
show_in_selector = true
```

## API Documentation Sources

- [Z.AI Developer Docs](https://docs.z.ai/devpack/overview)
- [OpenAI-compatible Integration](https://docs.z.ai/devpack/tool/others) - Base URL and model names
- [Thinking Mode](https://docs.z.ai/guides/capabilities/thinking-mode)
- [GLM-4.7 Blog](https://z.ai/blog/glm-4.7)
- [Mastra Z.AI Provider](https://mastra.ai/models/providers/zai-coding-plan)

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| API format changes | Version pin, comprehensive tests |
| Thinking mode incompatibilities | Dedicated crate allows custom handling |
| Rate limiting on coding plan | Document quota system in UI |
| API downtime | Graceful error handling, fallback provider option |

## Estimated Scope

- **rig-zai crate**: ~800-1000 lines
- **Backend integration**: ~200-300 lines
- **Settings updates**: ~100 lines
- **Frontend updates**: ~200 lines
- **Tests**: ~500 lines

## Open Questions

1. Should we support both standard and coding plan endpoints simultaneously?
2. Do we need preserved thinking (returning `reasoning_content` in history)?
3. Should thinking mode be configurable per-request or globally?
