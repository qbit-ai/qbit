# Eval Framework vs Production Agent: Detailed Differences

This document provides a comprehensive comparison between the eval framework and the production agent implementation.

## Executive Summary

The eval framework uses the **same unified agentic loop** (`run_agentic_loop_unified`) as production, ensuring evaluations test real agent behavior. However, several features are intentionally disabled or simplified for unattended, deterministic test execution.

## Architecture Comparison

### Shared Components

| Component | Status | Notes |
|-----------|--------|-------|
| Unified Agentic Loop | ✅ Same | `run_agentic_loop_unified()` |
| Tool Registry | ✅ Same | vtcode-core tools |
| Loop Detection | ✅ Same | Same defaults (100 iterations, 5 repeats) |
| Token Tracking | ✅ Same | Both track usage |
| Streaming | ✅ Same | Same stream processing |
| LLM Providers | ✅ Same | Vertex, OpenAI, Z.AI |

### Disabled/Different Components

| Component | Production | Evals |
|-----------|-----------|-------|
| HITL Approval | Full 8-step flow | Bypassed (AutoApprove) |
| Context Pruning | Enabled | Disabled |
| Sub-Agents | 5 registered | Empty registry |
| Session Persistence | Full | In-memory only |
| Sidecar Capture | Optional | Disabled |
| Indexer State | Optional | Disabled |
| Tavily State | Optional | Disabled |
| Runtime | Tauri/CLI | None |

---

## 1. Tool Availability

### Production Agent Tools

```
Standard Preset:
├── grep_file, list_files        # Search/Discovery
├── read_file, create_file       # File Operations
├── edit_file, write_file        # File Operations
├── delete_file                  # File Operations
├── run_pty_cmd (run_command)    # Shell
├── web_fetch                    # Web
└── update_plan                  # Planning

Main Agent Additions:
├── execute_code                 # Code execution
└── apply_patch                  # Patch-based editing

Indexer Tools (6):
├── indexer_search_code
├── indexer_search_files
├── indexer_analyze_file
├── indexer_extract_symbols
├── indexer_get_metrics
└── indexer_detect_language

Web Search (conditional):
├── web_search (native or Tavily)
├── web_search_answer (Tavily)
└── web_extract (Tavily)

Sub-Agent Tools (5):
├── sub_agent_coder
├── sub_agent_analyzer
├── sub_agent_explorer
├── sub_agent_researcher
└── sub_agent_executor
```

### Eval Framework Tools

```
Standard Preset Only:
├── grep_file, list_files
├── read_file, create_file
├── edit_file, write_file
├── delete_file
├── run_pty_cmd
├── web_fetch
└── update_plan

Indexer Tools: DISABLED (indexer_state = None)
Tavily Tools: DISABLED (tavily_state = None)
Sub-Agent Tools: DISABLED (empty registry)

Native Web Search: ENABLED (provider-specific)
├── web_search_20250305 (Claude)
├── web_search_preview (OpenAI)
└── Google grounding (Gemini)
```

### Missing Tools in Evals

| Tool | Why Missing |
|------|-------------|
| `execute_code` | Not in standard preset |
| `apply_patch` | Not in standard preset |
| `indexer_*` | indexer_state = None |
| `web_search_answer` | tavily_state = None |
| `web_extract` | tavily_state = None |
| `sub_agent_*` | Empty registry |

---

## 2. HITL Approval Flow

### Production Agent (8-step hierarchy)

```
1. Agent Mode Check → Planning mode denies writes
         ↓
2. Policy Denial → Tool explicitly denied
         ↓
3. Constraint Violations → Policy constraints violated
         ↓
4. Policy Allow → ALLOW_TOOLS list (auto-approve)
         ↓
5. Learned Patterns → Previously approved pattern
         ↓
6. Agent Mode Auto-approve → AgentMode::AutoApprove
         ↓
7. Runtime Flag → --auto-approve CLI flag
         ↓
8. HITL Prompt → User approval (300s timeout)
```

### Eval Framework

```
Setup:
  agent_mode = AgentMode::AutoApprove  // Forces step 6

Flow:
  Step 1-5: Checked but irrelevant
  Step 6: AutoApprove triggers → ALL tools approved
  Step 7-8: Never reached
```

**Key Code** (`eval_support.rs:184-185`):
```rust
let agent_mode = Arc::new(RwLock::new(AgentMode::AutoApprove));
```

---

## 3. Context Management

### Production Agent

```rust
ContextManagerConfig {
    enabled: true,              // Enforced
    compaction_threshold: 0.80, // Prune at 80%
    protected_turns: 2,         // Protect recent turns
    cooldown_seconds: 60,       // Throttle pruning
}
```

Features:
- Per-component token tracking (system, user, assistant, tool)
- Semantic scoring (System=950, User=850, Tool=600, Assistant=500)
- Protected recent turns (last 2 by default)
- Alert levels (Warning@75%, Alert@85%, Critical@100%+)
- Tool response truncation (25k tokens max)
- Event emission (ContextWarning, ContextPruned)

### Eval Framework

```rust
ContextManagerConfig {
    enabled: false,  // DISABLED
    ..Default::default()
}
```

Implications:
- No token budget tracking
- No message pruning
- Full history sent to LLM every turn
- No protection against context overflow
- Assumes test scenarios stay within limits

---

## 4. Session Management

### Production Agent

| Feature | Implementation |
|---------|----------------|
| Persistence | `~/.qbit/sessions/{session_id}/` |
| Format | JSON with metadata, transcript, messages |
| Saving | Incremental saves during conversation |
| Finalization | One-shot archive creation |
| Tool Tracking | Distinct tools used per session |
| Token Tracking | Optional total tokens field |

### Eval Framework

| Feature | Implementation |
|---------|----------------|
| Persistence | None (in-memory only) |
| Format | `EvalAgentOutput` struct |
| Saving | N/A |
| Multi-turn | Manual history accumulation |
| Recovery | Cannot resume failed evals |

**Multi-turn History Management** (`eval_support.rs`):
```rust
// Manual accumulation between turns
let mut current_history: Vec<Message> = Vec::new();

for user_prompt in prompts {
    current_history.push(user_message);
    let (response, new_history, tokens) = run_agentic_loop_unified(...);
    current_history = new_history;  // Update for next turn
}
```

---

## 5. Sub-Agent System

### Production Agent

```
SubAgentRegistry:
├── coder: Surgical code edits via unified diffs
├── analyzer: Deep code analysis via indexer
├── explorer: Codebase navigation and mapping
├── researcher: Web search and documentation
└── executor: Complex shell operations

Depth Limiting:
├── MAX_AGENT_DEPTH = 5
├── Tools hidden at depth ≥ 4
└── Each call increments depth

Prompt Contribution:
├── SubAgentPromptContributor adds docs
└── "## Available Sub-Agents" section
```

### Eval Framework

```
SubAgentRegistry: EMPTY (no agents registered)

No sub-agent tools available
No sub-agent documentation in prompt
No depth tracking needed
```

**Key Code** (`eval_support.rs:158-159`):
```rust
// Create empty sub-agent registry (no sub-agents in evals)
let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
```

---

## 6. Sidecar Context Capture

### Production Agent

| Feature | Implementation |
|---------|----------------|
| State Tracking | `SidecarState` with session file ops |
| Event Processing | Captures ToolRequest, ToolResult, Reasoning |
| Storage | `~/.qbit/sessions/{id}/state.md` |
| Artifacts | Tracks patches, diffs, file changes |
| Limits | Tool output: 2000 chars, Diffs: 4000 chars |

### Eval Framework

```rust
sidecar_state: None  // DISABLED
```

Implications:
- No context capture during evals
- No session state.md files
- No artifact tracking
- No decision pattern analysis
- Evals are "transparent" to sidecar

---

## 7. System Prompt

### Production Agent

```
build_system_prompt_with_contributions():
├── <identity> block (Qbit description)
├── <environment> block (workspace, date)
├── <style> block (concise, direct)
├── <workflow> block (5 phases with gates)
├── Tool documentation tables
├── Delegation guidelines
├── Security constraints
├── Project instructions (CLAUDE.md)
└── Dynamic contributions:
    ├── SubAgentPromptContributor
    └── ProviderBuiltinToolsContributor
```

### Eval Framework

**Default** (`executor.rs:EVAL_SYSTEM_PROMPT`):
```
Minimal, focused prompt:
- Lists available tools
- Direct task completion
- No preambles
- Auto-approved execution
```

**Scenario Override**:
```rust
fn system_prompt(&self) -> Option<&str> {
    Some(r#"
    You are an AI coding assistant.
    Complete the task efficiently.
    ...
    "#)
}
```

---

## 8. Loop Detection

### Both Paths (Identical)

| Setting | Value |
|---------|-------|
| `max_tool_loops` | 100 |
| `max_repeated_tool_calls` | 5 |
| `warning_threshold` | 0.6 (warn at 3/5) |
| Detector Reset | Per turn |
| Blocking | Same behavior |

**No differences** - both use identical loop detection configuration.

---

## 9. LLM Client Configuration

### Production Agent

```rust
// Full configuration with all features
let completion_model = vertex_client
    .completion_model(model)
    .with_default_thinking()  // Extended thinking
    .with_web_search();       // Native web search

// All provider features available
let client = LlmClient::VertexAnthropic(completion_model);
```

### Eval Framework

```rust
// Same configuration
let completion_model = vertex_client
    .completion_model(model)
    .with_default_thinking()
    .with_web_search();

// Identical client creation per provider
```

**No differences** in LLM configuration.

---

## 10. Event Handling

### Production Agent

```
Event Flow:
  AgenticLoop → event_tx → TauriRuntime → Frontend
                        → SidecarState → state.md
                        → ApprovalRecorder → patterns

Event Types:
  Started, TextDelta, Reasoning, ToolRequest,
  ToolApprovalRequest, ToolAutoApproved, ToolResult,
  ContextWarning, ContextPruned, LoopWarning,
  Completed, Error
```

### Eval Framework

```
Event Flow:
  AgenticLoop → event_tx → EvalAgentOutput.events
                        → (optionally to temp dir logs)

No sidecar, no frontend, no persistent capture
```

---

## Summary Table

| Feature | Production | Evals | Impact |
|---------|-----------|-------|--------|
| Agentic Loop | Unified | Same | ✅ Same behavior |
| Tool Set | Full + extras | Standard only | ⚠️ Some tools missing |
| HITL | 8-step | Bypassed | ⚠️ No approval testing |
| Context Pruning | Enabled | Disabled | ⚠️ No overflow protection |
| Sub-Agents | 5 registered | None | ❌ Cannot test delegation |
| Session Persistence | Full | None | ⚠️ No recovery |
| Sidecar | Optional | Disabled | ⚠️ No context capture |
| Indexer | Optional | Disabled | ⚠️ No code analysis tools |
| Tavily | Optional | Disabled | ⚠️ Fallback search unavailable |
| Loop Detection | Same | Same | ✅ Same protection |
| Token Tracking | Same | Same | ✅ Same tracking |

---

## Implications for Testing

### What Evals CAN Test

1. Core agent reasoning and response quality
2. Tool usage patterns (standard tools)
3. File operations (read, write, edit)
4. Shell command execution
5. Native web search (provider-specific)
6. Multi-turn conversation continuity
7. Reasoning ID preservation (OpenAI)
8. System prompt instruction following

### What Evals CANNOT Test

1. Sub-agent delegation behavior
2. HITL approval flow
3. Context window management
4. Approval pattern learning
5. Indexer-based code analysis
6. Tavily web search fallback
7. `execute_code` and `apply_patch` tools
8. Sidecar context capture

### Recommendations

See [Eval Framework Gaps](eval-framework-gaps.md) for identified gaps and potential improvements.
