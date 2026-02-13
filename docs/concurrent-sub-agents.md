# Concurrent Sub-Agents

## Overview

The agent can dispatch multiple sub-agent tool calls concurrently. When the LLM emits 2+ `sub_agent_*` calls in a single response, they execute in parallel via `futures::future::join_all` rather than sequentially. This is transparent to the LLM — it doesn't need to know about concurrency. It just emits multiple tool calls when the tasks are independent.

Non-sub-agent tool calls (file ops, shell commands, etc.) always execute sequentially.

## Architecture

### Tool Execution Flow

```
LLM response contains N tool calls
        │
        ▼
partition_tool_calls(tool_calls)
        │
        ├── sub_agent_* calls (M of them)
        │       │
        │       ├── M >= 2 → futures::future::join_all (concurrent)
        │       └── M < 2  → sequential (no spawn overhead)
        │
        └── other calls (N - M of them)
                │
                └── sequential (always)
        │
        ▼
merge results by original index
        │
        ▼
feed all results back to LLM as one User message
```

### Key Implementation Details

**Partitioning** (`agentic_loop.rs`):
- `is_sub_agent_tool(name)` — checks `name.starts_with("sub_agent_")`
- `partition_tool_calls(calls)` — splits into `(Vec<(index, call)>, Vec<(index, call)>)` preserving original indices

**Concurrent dispatch** (`agentic_loop.rs`):
- Each sub-agent call runs through `execute_single_tool_call()` which handles loop detection, HITL approval, execution, event emission, truncation, and post-tool hooks
- `join_all` collects `(original_index, result)` tuples
- Results are placed into a pre-allocated `Vec<Option<...>>` by index, then flattened in order

**Shared state** — all references are concurrency-safe:
- `AgenticLoopContext` fields are `&Arc<RwLock<T>>` — cloneable across tasks
- `LoopCaptureContext` uses `std::sync::Mutex` for interior mutability (`&self`, not `&mut self`)
- `ToolRegistry::execute_tool` takes `&self` (uses `read()` lock, not `write()`)
- `SubAgentContext.depth` is passed by value — no shared mutable state
- Each sub-agent gets its own `SubAgentTranscriptWriter` keyed by request ID

**HITL approval** works naturally — each concurrent sub-agent independently creates its own `oneshot` channel and emits its own `ToolApprovalRequest` event. Since `join_all` runs all futures concurrently, all approval requests appear in the UI simultaneously.

### File Conflict Handling

No file locking is used. When two concurrent sub-agents edit the same file, `edit_file`'s `old_text` exact-match semantics act as natural conflict detection — the second edit fails because the file content changed. The sub-agent sees the error, re-reads the file, and retries. In practice, conflicts are rare since the main agent dispatches agents to different tasks.

## Worker Agent

The `worker` is a general-purpose sub-agent with access to all standard tools. Unlike specialized agents (coder, explorer, analyzer, etc.), it has no fixed persona — its system prompt is automatically generated from a template tailored to each task.

### Definition

| Property | Value |
|----------|-------|
| ID | `worker` |
| Tools | All 13 standard tools (file ops, search, shell, web) |
| Max iterations | 30 |
| Timeout | 600s (10 min) |
| Idle timeout | 180s (3 min) |
| `prompt_template` | `WORKER_PROMPT_TEMPLATE` (auto-generates system prompt) |

### Tool Parameters

```json
{
  "task": "The specific task for this worker to handle (required)",
  "context": "Optional additional context"
}
```

The worker's system prompt is automatically generated via a prompt template before each execution — the main agent doesn't need to craft one. See the Prompt Generation Pipeline section below.

### When the Main Agent Should Use Workers

The system prompt instructs the agent:

- Use `worker` for tasks that don't fit a specialist
- Call multiple `worker` agents in a single response for independent tasks (runs concurrently)
- Do NOT parallelize when tasks have dependencies

## Prompt Generation Pipeline

### Problem

When the main agent calls `sub_agent_worker(task="implement X")`, the worker needs a focused, task-specific system prompt to produce high-quality results. Relying on the main agent to craft this prompt wastes tokens and produces inconsistent quality.

### Solution

Automatic prompt generation: every worker dispatch makes a lightweight LLM call to generate an optimized system prompt before the worker starts. This always runs — the goal is to always produce a tailored prompt for the specific task.

### Flow

```
Main agent calls sub_agent_worker(task="add error handling to parse_config")
        │
        ▼
execute_sub_agent_inner() sees prompt_template is Some(...)
        │
        ▼
Substitute {task} and {context} into the template
        │
        ▼
Make LLM call with the populated template
        │
        ▼
LLM returns: "You are a Rust expert focused on error handling..."
        │
        ▼
Use generated prompt as the worker's system prompt
        │
        ▼
Worker executes with the generated prompt
```

### Meta-Prompt Template

The `WORKER_PROMPT_TEMPLATE` constant in `defaults.rs` instructs the LLM how to generate a good worker system prompt. It uses `{task}` and `{context}` placeholders that are substituted before the LLM call.

```
You are a prompt engineer. Generate a focused system prompt for an AI coding agent
that will execute the following task.

Task: {task}
{context}

The agent has access to these tools: read_file, write_file, create_file, edit_file,
delete_file, list_files, list_directory, grep_file, ast_grep, ast_grep_replace,
run_pty_cmd, web_search, web_fetch.

Generate a system prompt that:
1. Defines the agent's role and expertise relevant to this specific task
2. Outlines a clear approach for completing the task
3. Specifies quality criteria and constraints
4. Is concise (under 500 words)

Return ONLY the system prompt text, no explanation or markdown formatting.
```

### When Prompt Generation Runs

- **Always runs** for agents with `prompt_template: Some(...)` (currently only `worker`)
- **Never runs** for specialized agents (`prompt_template: None`)

### Implementation Location

The prompt generation step lives in `execute_sub_agent_inner()` in `backend/crates/qbit-sub-agents/src/executor.rs`, right after extracting args. When `agent_def.prompt_template` is `Some(template)`, it substitutes placeholders, makes an LLM call, and uses the result as the system prompt. The definition's `system_prompt` field is used as a fallback if generation fails.

The model used for prompt generation is the same model passed to the sub-agent executor (typically the main agent's model). This is a single non-streaming completion call with low `max_tokens` (512) and no tools.

### Configuration

The meta-prompt template is defined as `WORKER_PROMPT_TEMPLATE` in `defaults.rs`. It's not configurable via settings — the template is an implementation detail that should be tuned by developers, not users.

A `summarizer_model` style override could be added later if prompt generation needs a cheaper/faster model than the main agent's model.

## Files Involved

| File | Role |
|------|------|
| `qbit-ai/src/agentic_loop.rs` | Concurrent dispatch: `partition_tool_calls`, `execute_single_tool_call`, `join_all` |
| `qbit-sub-agents/src/executor.rs` | Sub-agent execution, prompt generation step |
| `qbit-sub-agents/src/definition.rs` | `SubAgentDefinition.prompt_template` field |
| `qbit-sub-agents/src/defaults.rs` | Worker agent definition with `with_prompt_template()`, `WORKER_PROMPT_TEMPLATE` constant |
| `qbit-ai/src/tool_definitions.rs` | Sub-agent tool definitions (task + context params) |
| `qbit-ai/src/system_prompt.rs` | Agent instructions for concurrent dispatch and worker usage |
| `qbit-tools/src/registry.rs` | `execute_tool(&self)` — `&self` for concurrent read access |

## Testing

### Unit Tests

| Test | Location | What it verifies |
|------|----------|-----------------|
| `test_is_sub_agent_tool` | `agentic_loop.rs` | Tool name classification |
| `test_partition_tool_calls_*` | `agentic_loop.rs` | Partitioning with index preservation |
| `test_loop_capture_context_is_send_sync` | `agentic_loop.rs` | `LoopCaptureContext` is thread-safe |
| `test_loop_capture_context_concurrent_access` | `agentic_loop.rs` | Concurrent `process()` calls don't panic |
| `test_worker_has_broad_tool_access` | `defaults.rs` | Worker has all 13 tools |
| `test_worker_has_prompt_template` | `defaults.rs` | Worker has `prompt_template` with `{task}` placeholder |
| `test_specialized_agents_do_not_have_prompt_template` | `defaults.rs` | Other agents have `None` |
| `test_sub_agent_definition_with_prompt_template` | `definition.rs` | Builder method works |
| `test_sub_agent_tool_definitions_no_system_prompt_param` | `tool_definitions.rs` | No `system_prompt` param exposed to LLM |
| All existing HITL tests | `test_utils.rs` | `&LoopCaptureContext` works with existing flows |

### Manual Testing

```bash
# Headless mode — ask for concurrent workers explicitly
./target/debug/qbit -e "Run these two tasks in parallel using worker agents: 1) Find all TODO comments in the codebase 2) List all test files" --auto-approve

# Check backend log for concurrent execution
grep "Executing sub-agent tool calls concurrently" ~/.qbit/backend.log
```
