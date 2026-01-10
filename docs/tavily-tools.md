# Tavily Web Tools Implementation

This document provides a comprehensive guide to the Tavily web tools integration in Qbit. It covers architecture, configuration, tool descriptions, and how the tools contribute to system prompts.

## Overview

Tavily provides AI-powered web search capabilities. Qbit integrates 5 Tavily tools that enable the agent to search the web, extract content from URLs, crawl websites, and map site structures.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Qbit Agent                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────────────┐         ┌────────────────────────────┐   │
│   │  Tool Registry   │────────▶│  ToolDefinition (to LLM)   │   │
│   │  (qbit-tools)    │         │  - name, description       │   │
│   └────────┬─────────┘         │  - parameters schema       │   │
│            │                   └────────────────────────────┘   │
│            │                                                     │
│   ┌────────▼─────────┐         ┌────────────────────────────┐   │
│   │  TavilyState     │         │  TavilyToolsContributor    │   │
│   │  (qbit-web)      │         │  (system prompt section)   │   │
│   └────────┬─────────┘         └────────────────────────────┘   │
│            │                                                     │
│   ┌────────▼─────────┐                                          │
│   │  Tavily API      │                                          │
│   │  api.tavily.com  │                                          │
│   └──────────────────┘                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Key Files

| File | Purpose |
|------|---------|
| `backend/crates/qbit-web/src/tavily.rs` | Tavily API client (`TavilyState`) |
| `backend/crates/qbit-web/src/tool.rs` | Tool implementations (`WebSearchTool`, etc.) |
| `backend/crates/qbit-web/src/lib.rs` | Crate exports |
| `backend/crates/qbit-tools/src/registry.rs` | Tool registration logic |
| `backend/crates/qbit-ai/src/contributors/tavily_tools.rs` | System prompt contributor |
| `backend/crates/qbit-ai/src/contributors/mod.rs` | Contributor registration |

## Available Tools

### 1. `web_search`

Search the web for information. Returns relevant results with titles, URLs, and content snippets.

**Use case**: Current information, news, documentation, or facts beyond training data.

**Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | ✓ | The search query |
| `max_results` | integer | | Maximum results to return (default: 5) |
| `search_depth` | enum | | `"basic"` or `"advanced"` (default: basic) |
| `topic` | string | | Category like "general", "news" |
| `include_domains` | string[] | | Domains to include |
| `exclude_domains` | string[] | | Domains to exclude |

### 2. `web_search_answer`

Get an AI-generated answer synthesized from web search results.

**Use case**: Direct questions needing a consolidated answer from multiple sources.

**Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | ✓ | The question to answer |

### 3. `web_extract`

Extract and parse content from specific URLs.

**Use case**: Getting full page content for deeper analysis when you have specific URLs.

**Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `urls` | string[] | ✓ | URLs to extract content from |
| `query` | string | | Optional focus for extraction |
| `extract_depth` | enum | | `"basic"` or `"advanced"` |
| `format` | enum | | `"markdown"` or `"text"` (default: markdown) |

### 4. `web_crawl`

Crawl a website starting from a URL, following links to extract content from multiple pages.

**Use case**: Comprehensive site analysis or documentation gathering.

**Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url` | string | ✓ | Base URL to start crawling from |
| `max_depth` | integer | | Maximum crawl depth |
| `max_breadth` | integer | | Maximum pages per level |
| `limit` | integer | | Maximum total pages |
| `instructions` | string | | Natural language focus instructions |
| `allow_external` | boolean | | Follow external links |

### 5. `web_map`

Map the structure of a website, returning discovered URLs.

**Use case**: Discover site structure before crawling or extracting specific pages.

**Parameters**:
| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url` | string | ✓ | Base URL to map |
| `max_depth` | integer | | Maximum exploration depth |
| `max_breadth` | integer | | Maximum links per level |
| `limit` | integer | | Maximum URLs to return |
| `instructions` | string | | Natural language focus instructions |

## Configuration

### Settings File

Configure Tavily in `~/.qbit/settings.toml`:

```toml
[tools]
web_search = true  # Enable/disable Tavily tools

[api_keys]
tavily = "tvly-your-api-key-here"
```

### Environment Variable Fallback

If `api_keys.tavily` is not set in settings, Qbit falls back to:

```bash
export TAVILY_API_KEY="tvly-your-api-key-here"
```

### Behavior Without API Key

- Tools are **still registered** when `web_search = true`
- Execution fails with a helpful error message
- This allows the LLM to see the tools but prevents actual API calls

## Architecture

### Tool Registration Flow

```
QbitSettings
    │
    ├── tools.web_search = true?
    │       │
    │       ▼
    │   Resolve API key (settings → env fallback)
    │       │
    │       ▼
    │   TavilyState::from_api_key(Option<String>)
    │       │
    │       ▼
    │   create_tavily_tools(Arc<TavilyState>)
    │       │
    │       ▼
    │   Register each tool in ToolRegistry.tools HashMap
    │
    ▼
ToolRegistry::get_tool_definitions()
    │
    ▼
Vec<rig::completion::ToolDefinition>  →  LLM API
```

### Tool Trait Implementation

Each tool implements `qbit_core::Tool`:

```rust
#[async_trait::async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "web_search"
    }

    fn description(&self) -> &'static str {
        "Search the web for information..."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": { ... },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value, _workspace: &Path) -> Result<Value> {
        // Call Tavily API via self.tavily.search(...)
    }
}
```

### Shared State

All 5 tools share an `Arc<TavilyState>`:

```rust
pub fn create_tavily_tools(tavily: Arc<TavilyState>) -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(WebSearchTool::new(tavily.clone())),
        Arc::new(WebSearchAnswerTool::new(tavily.clone())),
        Arc::new(WebExtractTool::new(tavily.clone())),
        Arc::new(WebCrawlTool::new(tavily.clone())),
        Arc::new(WebMapTool::new(tavily)),
    ]
}
```

## System Prompt Integration

Tavily tools integrate with the system prompt in **two ways**:

### 1. ToolDefinition (Primary)

Tool descriptions flow to the LLM via `ToolDefinition` objects:

```
Tool::description() → ToolRegistry::get_tool_definitions() → LLM API
```

This is the standard mechanism for tool documentation.

### 2. TavilyToolsContributor (Secondary)

A `PromptContributor` adds detailed usage instructions to the system prompt:

**Location**: `backend/crates/qbit-ai/src/contributors/tavily_tools.rs`

**Activation conditions**:
- `ctx.has_web_search == true` (Tavily tools registered)
- `ctx.has_native_web_tools == false` (not using Claude's built-in web search)

**Priority**: `PromptPriority::Tools`

```rust
impl PromptContributor for TavilyToolsContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        if !ctx.has_web_search || ctx.has_native_web_tools {
            return None;
        }
        Some(vec![PromptSection::new(
            "tavily_tools",
            PromptPriority::Tools,
            TAVILY_TOOLS_DOCUMENTATION,
        )])
    }
}
```

### Context Flag Detection

In `agent_bridge.rs`, `has_web_search` is set by checking for registered tools:

```rust
let has_web_search = self
    .tool_registry
    .read()
    .await
    .available_tools()
    .iter()
    .any(|t| t.starts_with("web_"));
```

## Native vs Tavily Web Tools

Qbit supports **two** web search backends:

| Feature | Native (Claude) | Tavily |
|---------|-----------------|--------|
| Provider | Anthropic only | Any LLM |
| Tools | `web_search` | `web_search`, `web_extract`, `web_crawl`, `web_map`, `web_search_answer` |
| Context flag | `has_native_web_tools` | `has_web_search` |
| System prompt | `ProviderBuiltinToolsContributor` | `TavilyToolsContributor` |

**Priority**: Native tools take precedence. When `has_native_web_tools` is true, `TavilyToolsContributor` does not contribute.

## Testing

### Unit Tests

Run Tavily-related tests:

```bash
# Tavily contributor tests
cargo test --package qbit-ai -- tavily

# Tavily tool tests
cargo test --package qbit-web
```

### Manual Testing

1. Set your Tavily API key:
   ```bash
   export TAVILY_API_KEY="tvly-..."
   ```

2. Enable web search in settings:
   ```toml
   # ~/.qbit/settings.toml
   [tools]
   web_search = true
   ```

3. Start Qbit and ask a question requiring web search:
   ```
   What's the latest news about Rust 2024 edition?
   ```

## Troubleshooting

### "Tavily API key not configured"

**Cause**: No API key in settings or environment.

**Fix**: Set `api_keys.tavily` in `~/.qbit/settings.toml` or export `TAVILY_API_KEY`.

### Tools not appearing in LLM context

**Cause**: `tools.web_search = false` in settings.

**Fix**: Set `tools.web_search = true` in `~/.qbit/settings.toml`.

### System prompt doesn't include Tavily documentation

**Cause**: Either `has_native_web_tools` is true (using Claude's built-in search) or `has_web_search` is false.

**Debug**: Check the prompt context flags in logs.

## Adding New Tavily Endpoints

To add a new Tavily API endpoint:

1. **Add API types in `tavily.rs`**:
   ```rust
   #[derive(Serialize)]
   struct NewEndpointRequest { ... }
   
   #[derive(Deserialize)]
   pub struct NewEndpointResponse { ... }
   ```

2. **Add method to `TavilyState`**:
   ```rust
   pub async fn new_endpoint(&self, ...) -> Result<NewEndpointResponse> {
       self.post_json("/new-endpoint", &request).await
   }
   ```

3. **Create tool struct in `tool.rs`**:
   ```rust
   pub struct NewEndpointTool {
       tavily: Arc<TavilyState>,
   }
   
   impl Tool for NewEndpointTool { ... }
   ```

4. **Register in `create_tavily_tools()`**:
   ```rust
   vec![
       // existing tools...
       Arc::new(NewEndpointTool::new(tavily)),
   ]
   ```

5. **Update contributor documentation** in `tavily_tools.rs`.

6. **Add tests** in both `tavily.rs` and `tool.rs`.
