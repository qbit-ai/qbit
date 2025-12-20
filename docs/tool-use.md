# Tool Use Architecture

This document explains how the AI agent's tool system works in Qbit.

## Overview

The tool system enables the AI agent to interact with the local environment through defined actions like reading files, executing shell commands, and searching the web. Tools flow through three layers:

```
Tool Definitions (schemas)
        ↓
Tool Execution (handlers)
        ↓
HITL Approval (safety)
```

## Key Files

| File | Purpose |
|------|---------|
| `backend/src/ai/tool_definitions.rs` | Tool schemas and presets |
| `backend/src/ai/tool_executors.rs` | Tool execution handlers |
| `backend/src/ai/agentic_loop.rs` | Execution flow and HITL integration |
| `backend/src/ai/hitl/approval_recorder.rs` | Approval tracking and risk levels |

## Tool Categories

### 1. Standard Tools (from vtcode-core)

Provided by the vtcode-core crate via `build_function_declarations()`:

- **File operations**: `read_file`, `write_file`, `edit_file`, `delete_file`, `create_file`
- **Search**: `grep_file`, `list_files`
- **Shell**: `run_command` (wraps `run_pty_cmd` internally)
- **Web**: `web_fetch`
- **Code execution**: `execute_code`, `apply_patch`

### 2. Indexer Tools (custom)

Semantic code analysis tools defined in `get_indexer_tool_definitions()`:

- `indexer_search_code` - Regex search in indexed workspace
- `indexer_search_files` - Glob pattern file search
- `indexer_analyze_file` - Semantic analysis with tree-sitter
- `indexer_extract_symbols` - Extract functions, classes, imports
- `indexer_get_metrics` - Code metrics (LOC, comments, etc.)
- `indexer_detect_language` - Language detection

### 3. Tavily Tools (custom)

Web search tools defined in `get_tavily_tool_definitions()`:

- `web_search` - Search with result snippets
- `web_search_answer` - AI-generated answer from search
- `web_extract` - Extract content from URLs

### 4. Sub-agent Tools (dynamic)

Generated from `SubAgentRegistry` in `get_sub_agent_tool_definitions()`:

- `sub_agent_code_explorer` - Navigate and map codebases
- `sub_agent_code_analyzer` - Deep semantic analysis
- `sub_agent_code_writer` - Implement code changes
- `sub_agent_researcher` - Web research
- `sub_agent_shell_executor` - Complex command orchestration

### 5. Workflow Tools (custom)

Multi-step AI workflows defined in `get_workflow_tool_definitions()`:

- `run_workflow` - Execute pre-defined task pipelines

## Tool Presets

Three preset levels control default tool availability:

```rust
pub enum ToolPreset {
    Minimal,   // read, edit, write, run_command (4 tools)
    Standard,  // Core development tools (default)
    Full,      // All vtcode tools
}
```

## Tool Configuration

`ToolConfig` provides fine-grained control:

```rust
pub struct ToolConfig {
    pub preset: ToolPreset,      // Base preset
    pub additional: Vec<String>, // Extra tools to enable
    pub disabled: Vec<String>,   // Tools to disable
}
```

The main agent uses `ToolConfig::main_agent()`:

```rust
pub fn main_agent() -> Self {
    Self {
        preset: ToolPreset::Standard,
        additional: vec![
            "execute_code".to_string(),
            "apply_patch".to_string(),
        ],
        disabled: vec![],  // All tools enabled
    }
}
```

## Execution Flow

### 1. Tool Request

The LLM receives tool definitions via `CompletionRequest.tools` and returns tool calls in its response.

### 2. HITL Check (`execute_with_hitl`)

Located in `agentic_loop.rs`:

1. **Policy denial check** - Is tool blocked by policy?
2. **Constraint application** - Apply file patterns, timeouts, etc.
3. **Policy allow check** - Does policy explicitly allow?
4. **Auto-approval check** - Should auto-approve based on learned patterns?
5. **User approval request** - Emit event, wait for response (5-minute timeout)

### 3. Tool Execution (`execute_tool_direct`)

Routes to specialized executors:

```rust
if is_indexer_tool(tool_name) {
    return execute_indexer_tool(...);
}
if tool_name == "web_fetch" {
    return execute_web_fetch_tool(...);
}
if is_tavily_tool(tool_name) {
    return execute_tavily_tool(...);
}
if tool_name == "run_workflow" {
    return execute_workflow_tool(...);
}
if tool_name.starts_with("sub_agent_") {
    return execute_sub_agent(...);
}
// Fall through to vtcode registry
registry.execute_tool(tool_name, tool_args).await
```

## Risk Levels

Defined in `RiskLevel::for_tool()`:

| Level | Tools | Behavior |
|-------|-------|----------|
| Low | `read_file`, `grep_file`, `list_files`, indexer tools | Can auto-approve |
| Medium | `write_file`, `edit_file` | Can auto-approve after learning |
| High | `run_command`, `run_pty_cmd`, `send_pty_input` | Always requires approval |
| Critical | `delete_file`, `execute_code` | Always requires approval |

### Auto-Approval Learning

For tools not in `always_require_approval`:

- Minimum 3 approvals required
- 80% approval rate threshold
- Pattern matching based on tool name

## Adding a New Tool

### Step 1: Define the Tool

In `tool_definitions.rs`, add to appropriate function:

```rust
ToolDefinition {
    name: "my_tool".to_string(),
    description: "What this tool does".to_string(),
    parameters: json!({
        "type": "object",
        "properties": {
            "param": {
                "type": "string",
                "description": "Parameter description"
            }
        },
        "required": ["param"]
    }),
}
```

### Step 2: Implement the Executor

In `tool_executors.rs`:

```rust
pub async fn execute_my_tool(
    tool_name: &str,
    args: &serde_json::Value,
) -> ToolResult {
    let param = args.get("param")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing param"))?;

    // Execute tool logic
    let result = do_something(param)?;

    Ok((json!({ "result": result }), true))
}
```

### Step 3: Route the Executor

In `agentic_loop.rs` `execute_tool_direct()`:

```rust
if tool_name == "my_tool" {
    let (value, success) = execute_my_tool(tool_name, tool_args).await?;
    return Ok(ToolExecutionResult { value, success });
}
```

### Step 4: Set Risk Level

In `approval_recorder.rs` `RiskLevel::for_tool()`:

```rust
"my_tool" => RiskLevel::Medium,
```

### Step 5: Add to Preset (Optional)

In `ToolPreset::tool_names()` if it should be available by default:

```rust
ToolPreset::Standard => Some(vec![
    // existing tools...
    "my_tool",
]),
```

## Frontend Integration

### Events

Tool approval requests emit `AiEvent::ToolApprovalRequest`:

```rust
AiEvent::ToolApprovalRequest {
    request_id: String,
    tool_name: String,
    args: serde_json::Value,
    risk_level: RiskLevel,
    can_learn: bool,
}
```

### Approval Response

Frontend calls `respond_to_tool_approval` Tauri command:

```rust
pub struct ApprovalDecision {
    pub request_id: String,
    pub approved: bool,
    pub reason: Option<String>,
    pub remember: bool,
    pub always_allow: bool,
}
```

## Testing

Run tool-related tests:

```bash
# Rust tests
cargo test -p qbit tool

# Specific module
cargo test -p qbit tool_definitions
cargo test -p qbit tool_executors
```
