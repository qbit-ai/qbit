# Tool Access in Qbit Agents

## Summary of Findings

### 1. Do you have access to any Tavily tools?

**No** - As an AI assistant running in this environment, I do NOT have direct access to Tavily tools. However, the Qbit codebase implements Tavily integration that gives agents web search capabilities.

**Tavily Tools Implemented in Qbit:**

Location: `backend/crates/qbit-web/src/tavily.rs` and `backend/crates/qbit-web/src/tool.rs`

| Tool Name | Description |
|-----------|-------------|
| `web_search` | Search the web for information. Returns relevant results with titles, URLs, and content snippets |
| `web_search_answer` | Get an AI-generated answer synthesized from web search results |
| `web_extract` | Extract and parse content from specific URLs |
| `web_crawl` | Crawl a website starting from a URL, following links to extract content from multiple pages |
| `web_map` | Map the structure of a website, returning a list of discovered URLs |

**Configuration:**
- Enabled via `settings.tools.web_search = true`
- API key from `settings.api_keys.tavily` or `TAVILY_API_KEY` environment variable
- Tools are registered in `ToolRegistry` even if API key is missing (errors occur at execution time)

### 2. Do you have any web search or web extract tools?

**No** - I do not have direct access to web search or web extract tools in my current environment. The web capabilities in Qbit are designed for agents, not for the assistant interface.

**Web-related Tools in Qbit:**

| Tool | Location | Purpose |
|------|----------|---------|
| `web_fetch` | `backend/crates/qbit-web/src/web_fetch.rs` | Custom fetch with Mozilla readability content extraction |
| `web_search` | Tavily | Web search via Tavily API |
| `web_search_answer` | Tavily | Synthesized answers from search results |
| `web_extract` | Tavily | Extract content from specific URLs |
| `web_crawl` | Tavily | Multi-page website crawling |
| `web_map` | Tavily | Website structure mapping |

**Note:** There's also support for **provider-native web tools**:
- Claude's native: `web_search_20250305`, `web_fetch_20250910` (Vertex AI Anthropic)
- OpenAI's native: `web_search_preview`

These are used when available, bypassing Tavily implementation.

### 3. Where in the codebase are agents given access to tools?

Tools are assigned to agents through a multi-layer architecture:

#### A. Tool Registry
**File:** `backend/crates/qbit-tools/src/registry.rs`

The `ToolRegistry` is the central registry that:
- Registers all available tools when initialized
- Manages tool execution
- Provides tool definitions to the agent system

**Core tools always registered:**
- File operations: `read_file`, `write_file`, `create_file`, `edit_file`, `delete_file`
- Directory operations: `list_files`, `list_directory`, `grep_file`
- Shell: `run_pty_cmd`
- AST-grep: `ast_grep`, `ast_grep_replace`

**Conditional tools:**
- Tavily web tools - registered if `settings.tools.web_search = true`

#### B. Tool Configuration & Presets
**File:** `backend/crates/qbit-ai/src/tool_definitions.rs`

`ToolConfig` determines which tools are enabled for a given agent:

| Preset | Tools Included |
|--------|----------------|
| `Minimal` | `read_file`, `edit_file`, `write_file`, `run_pty_cmd` |
| `Standard` (default) | All Minimal + `grep_file`, `list_files`, `ast_grep`, `ast_grep_replace`, `create_file`, `delete_file`, `web_fetch`, `update_plan` |
| `Full` | All available tools |

**Main agent configuration:**
```rust
ToolConfig::main_agent() -> Standard preset + execute_code + apply_patch - run_pty_cmd
```

#### C. Agentic Loop Tool Selection
**File:** `backend/crates/qbit-ai/src/agentic_loop.rs` (around lines 1120-1145)

The agentic loop determines which tools are exposed to the LLM:

```rust
// Priority order:
1. Native provider web tools (Claude/OpenAI) if available
2. Registry-based web tools (Tavily) if enabled and native not available
3. Sub-agent tools (if not at max depth)
```

**Key logic:**
```rust
if use_native_web_tools {
    // Skip Tavily - using Claude's web_search/web_fetch
} else if use_openai_web_search {
    // Skip Tavily - using OpenAI's web_search_preview
} else {
    // Add Tavily tools from registry if tool_config allows
    if tool.name.starts_with("web_") && tool_config.is_tool_enabled(&tool.name) {
        tools.push(tool);
    }
}
```

#### D. Tool Executors
**File:** `backend/crates/qbit-ai/src/tool_executors.rs`

Contains the actual execution logic for special tools:
- `execute_web_fetch_tool` - Routes to readability-based fetch
- `execute_plan_tool` - Updates task plans
- `execute_indexer_tool` - Code search via indexer

#### E. Sub-Agent Tool Registration
**File:** `backend/crates/qbit-sub-agents/src/defaults.rs`

Sub-agents have their own tool sets:

| Sub-Agent | Available Tools |
|-----------|-----------------|
| `analyzer` | `ast_grep`, `grep_file`, `read_file`, `web_fetch` |
| `researcher` | `web_search`, `web_fetch`, `read_file` |
| `executor` | `run_command`, `run_pty_cmd` |
| `explorer` | `list_files`, `grep_file`, `read_file`, `indexer_search_files`, `indexer_search_code` |
| `coder` | `edit_file`, `create_file`, `write_file`, `ast_grep_replace`, `read_file` |

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Qbit Tool System                          │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────┐                                        │
│  │   ToolRegistry   │                                        │
│  │  (registry.rs)   │                                        │
│  └────────┬─────────┘                                        │
│           │                                                   │
│           │ Registers on init:                                │
│           ├─ Core tools (file, dir, shell, ast-grep)          │
│           ├─ Tavily tools (if web_search=true)                │
│           └─ Returns ToolDefinition list                      │
│                                                              │
│  ┌──────────────────┐                                        │
│  │   ToolConfig     │                                        │
│  │(tool_definitions│                                        │
│  │      .rs)        │                                        │
│  └────────┬─────────┘                                        │
│           │                                                   │
│           │ Filters by preset:                                │
│           ├─ Minimal / Standard / Full                       │
│           └─ Additional / Disabled lists                     │
│                                                              │
│  ┌─────────────────────────────────────────────┐            │
│  │         Agentic Loop (agentic_loop.rs)        │            │
│  │                                              │            │
│  │  1. Check native provider web tools?         │            │
│  │     ├─ Yes → Use Claude/OpenAI web tools     │            │
│  │     └─ No  → Check Tavily registry tools    │            │
│  │                                              │            │
│  │  2. Add tools per ToolConfig                │            │
│  │                                              │            │
│  │  3. Add sub-agent tools (if depth < max)     │            │
│  └──────────────┬───────────────────────────────┘            │
│                 │                                             │
│                 ▼                                             │
│  ┌─────────────────────────────────────────────┐            │
│  │              LLM Tools Array                │            │
│  │    (What the agent can actually call)       │            │
│  └─────────────────────────────────────────────┘            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Files Reference

| File | Purpose |
|------|---------|
| `backend/crates/qbit-tools/src/registry.rs` | Central tool registration and execution |
| `backend/crates/qbit-ai/src/tool_definitions.rs` | Tool configuration presets |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Tool selection logic for agents |
| `backend/crates/qbit-ai/src/tool_executors.rs` | Special tool execution routing |
| `backend/crates/qbit-web/src/tavily.rs` | Tavily API integration |
| `backend/crates/qbit-web/src/tool.rs` | Tavily tool implementations |
| `backend/crates/qbit-web/src/web_fetch.rs` | Readability-based web fetch |
| `backend/crates/qbit-sub-agents/src/defaults.rs` | Sub-agent tool configurations |
