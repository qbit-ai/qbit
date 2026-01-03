# System Prompt Design Brief for Qbit

## Project Overview

**Qbit** is an AI-powered terminal emulator built with Tauri 2 (Rust backend, React frontend). It serves as an intelligent software engineering assistant that operates within a terminal environment and can delegate work to specialized sub-agents.

The system prompt for Qbit is dynamically generated at runtime based on:
- Available tools (configured via presets and custom allow/block lists)
- Registered sub-agents
- LLM provider being used (Anthropic, OpenAI, Gemini, etc.)
- Agent mode (Default, Planning, AutoApprove)
- Project-specific memory files (similar to CLAUDE.md)

---

## Architecture: Dynamic Prompt Composition

### Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    build_system_prompt_with_contributions        │
├─────────────────────────────────────────────────────────────────┤
│  1. Base Prompt (static sections)                               │
│     - Identity & Environment                                     │
│     - Core Workflow (Investigate → Plan → Approve → Execute)    │
│     - File Operation Rules                                       │
│     - Delegation Decision Tree                                   │
│     - Sub-Agent Specifications                                   │
│     - Security Boundaries                                        │
│                                                                  │
│  2. Agent Mode Instructions (conditional)                        │
│     - Planning Mode: Read-only restrictions                      │
│     - AutoApprove Mode: Caution note                            │
│     - Default Mode: Empty                                        │
│                                                                  │
│  3. Project Instructions (optional)                              │
│     - Loaded from configured memory file (e.g., CLAUDE.md)       │
│                                                                  │
│  4. Dynamic Contributions (from PromptContributorRegistry)       │
│     - SubAgentPromptContributor: Lists available sub-agents      │
│     - ProviderBuiltinToolsContributor: Provider-specific tools   │
└─────────────────────────────────────────────────────────────────┘
```

### Prompt Priority System

Contributions are ordered by priority (lower = earlier in prompt):

| Priority | Value | Purpose |
|----------|-------|---------|
| Core | 0 | Identity and environment |
| Workflow | 100 | Behavior rules |
| Tools | 200 | Tool documentation |
| Features | 300 | Feature-specific instructions |
| Provider | 400 | Provider-specific instructions |
| Context | 500 | Dynamic runtime context |

### PromptContext (passed to contributors)

```rust
pub struct PromptContext {
    pub provider: String,           // "anthropic", "openai", "vertex_ai", etc.
    pub model: String,              // "claude-sonnet-4-20250514"
    pub available_tools: Vec<String>,
    pub has_web_search: bool,       // Tavily or provider-specific
    pub has_native_web_tools: bool, // Claude's native web tools
    pub has_sub_agents: bool,       // Depth check passed
    pub workspace: Option<String>,
}
```

---

## Available Tools

### Tool Presets

| Preset | Description | Tools |
|--------|-------------|-------|
| **Minimal** | Essential file ops only | `read_file`, `edit_file`, `write_file`, `run_pty_cmd` |
| **Standard** | Core development tools (default) | `grep_file`, `list_files`, `read_file`, `create_file`, `edit_file`, `write_file`, `delete_file`, `run_pty_cmd`, `web_fetch`, `update_plan` |
| **Full** | All vtcode tools | Everything available |

### Main Agent Configuration

The main agent uses Standard preset with additions:
- Added: `execute_code`, `apply_patch`
- Hidden: `run_pty_cmd` (exposed as `run_command` with friendlier name)

### Core File Operation Tools

| Tool | Purpose |
|------|---------|
| `read_file` | Read file contents (with optional line range) |
| `edit_file` | Make targeted edits to existing files |
| `write_file` | Write entire file content (create or overwrite) |
| `create_file` | Create a new file (fails if exists) |
| `delete_file` | Delete a file |
| `list_files` | List files matching a pattern |
| `list_directory` | List directory contents |
| `grep_file` | Search file contents with regex |
| `find_files` | Find files by name pattern |

### Shell Execution

| Tool | Purpose |
|------|---------|
| `run_command` | Execute shell command (wraps `run_pty_cmd` with better name) |
| `run_pty_cmd` | Raw PTY command execution (hidden from main agent) |

### Code Indexer Tools

| Tool | Purpose |
|------|---------|
| `indexer_search_code` | Regex search across indexed workspace |
| `indexer_search_files` | Find files by glob pattern |
| `indexer_analyze_file` | Get semantic analysis with tree-sitter |
| `indexer_extract_symbols` | Extract functions, classes, imports, etc. |
| `indexer_get_metrics` | Get code metrics (LOC, comments, etc.) |
| `indexer_detect_language` | Detect programming language |

### Web Tools

| Tool | Availability | Purpose |
|------|--------------|---------|
| `web_search` | If Tavily configured | Search the web for information |
| `web_search_answer` | If Tavily configured | Get AI-synthesized answer from search |
| `web_fetch` | Always (Standard preset) | Fetch and parse web page content |
| `web_extract` | If Tavily configured | Extract content from specific URLs |

### Planning Tool

| Tool | Purpose |
|------|---------|
| `update_plan` | Create/update task plan with steps and progress |

### Code Editing

| Tool | Purpose |
|------|---------|
| `apply_patch` | Apply unified diff patches for multi-hunk edits |
| `execute_code` | Execute code in a sandbox |

---

## Sub-Agents

Sub-agents are specialized agents that can be delegated to for specific tasks. They're exposed as tools prefixed with `sub_agent_`.

### Default Sub-Agents

#### 1. `coder`
**Purpose**: Applies surgical code edits using unified diff format
**Max Iterations**: 20
**Tools**: `read_file`, `list_files`, `grep_file`
**Output**: Standard git-style unified diffs that are parsed and applied automatically
**Use When**: Multiple related edits to a single file

#### 2. `analyzer`
**Purpose**: Deep semantic analysis of code (read-only)
**Max Iterations**: 30
**Tools**: `read_file`, `grep_file`, `list_directory`, `find_files`, all `indexer_*` tools
**Use When**: Understanding code structure, finding patterns, code metrics
**Key Rule**: AFTER explorer identifies key files

#### 3. `explorer`
**Purpose**: Navigates and maps codebases to build context
**Max Iterations**: 40
**Tools**: `read_file`, `list_files`, `list_directory`, `grep_file`, `find_files`, `run_pty_cmd`
**Use When**: Unfamiliar code, tracing dependencies, finding integration points
**Key Rule**: Ideal FIRST step for unfamiliar code

#### 4. `researcher`
**Purpose**: In-depth web research
**Max Iterations**: 25
**Tools**: `web_search`, `web_fetch`, `read_file`
**Use When**: Multi-source documentation, complex API lookup, best practices

#### 5. `executor`
**Purpose**: Complex shell command orchestration
**Max Iterations**: 30
**Tools**: `run_pty_cmd`, `read_file`, `list_directory`
**Use When**: Multi-step builds, chained git operations, long-running pipelines

### Sub-Agent Tool Schema

All sub-agent tools use this schema:
```json
{
  "type": "object",
  "properties": {
    "task": {
      "type": "string",
      "description": "The specific task or question for this sub-agent to handle"
    },
    "context": {
      "type": "string",
      "description": "Optional additional context to help the sub-agent understand the task"
    }
  },
  "required": ["task"]
}
```

### Sub-Agent Recursion

- **Maximum depth**: 5 levels
- Sub-agents can spawn other sub-agents (within depth limit)
- Each sub-agent maintains its own context and variables

---

## Agent Modes

### Default Mode
Standard operation with HITL approval for tool calls.

### Planning Mode
**Read-only restrictions** - Cannot modify files or execute state-changing commands.

Allowed:
- Reading files
- Code analysis
- Web research
- Creating plans

Forbidden:
- File modifications
- State-changing shell commands
- Code execution

### AutoApprove Mode
All tool operations automatically approved. Used for trusted automation scenarios.

---

## Current System Prompt Structure

The base system prompt currently includes these sections:

### 1. Identity & Environment
```
You are Qbit, an intelligent and highly advanced software engineering assistant.

## Environment
- **Working Directory**: {workspace}
- **Date**: {date}

## Communication Style
- Direct answers without preambles or postambles
```

### 2. Core Workflow (5 phases with gates)

| Phase | Gate Condition |
|-------|---------------|
| **Investigate** | Clear understanding before proceeding |
| **Plan** | Concrete, not abstract plan with `update_plan` |
| **Approve** | Explicit user approval (skip for trivial changes) |
| **Execute** | Use appropriate agents, run verification |
| **Verify (CRITICAL)** | Must run lint/typecheck and tests |

### 3. File Operation Rules

| Action | Requirement |
|--------|-------------|
| Edit existing | MUST read file first |
| Multiple edits (same file) | Use `coder` agent |
| Create new | Use `write_file` (last resort) |
| Multiple edits (different files) | Prefer `edit_file` over `write_file` |

### 4. Delegation Decision Tree

**Delegate When**:
1. Unfamiliar code → `explorer`
2. Cross-module changes → `explorer`
3. Architectural questions → `explorer` → `analyzer`
4. Tracing dependencies → `analyzer`
5. Multi-edit same file → `coder`
6. Complex shell pipelines → `executor`
7. In-depth research → `researcher`
8. Quick commands → `run_command` directly

**Handle Directly When**:
- Single file you've already read
- User provides exact file and exact change
- Trivial fixes (typos, formatting)
- Question answerable from current context

### 5. Sub-Agent Specifications
Each sub-agent has purpose, use cases, tools, and patterns documented.

### 6. Security Boundaries
- NEVER expose secrets in logs/output
- NEVER commit credentials
- NEVER generate code that logs sensitive data

---

## Provider-Specific Instructions

The system includes provider-specific prompt sections:

### Anthropic (claude, vertex_ai)
- Native web tools: `web_search`, automatic citations
- Tavily fallback for web search if native not enabled

### OpenAI
- Web search via Responses API
- Clear, specific queries
- Source citation

### Gemini
- Google Search grounding
- Real-time web information
- Source citation

---

## Design Requirements for New System Prompt

### Deliverables

You must produce system prompts for:

1. **Main Agent (Qbit)** - The primary orchestrating agent
2. **coder** - Code editing sub-agent
3. **analyzer** - Code analysis sub-agent
4. **explorer** - Codebase navigation sub-agent
5. **researcher** - Web research sub-agent
6. **executor** - Shell command sub-agent

Each sub-agent prompt should be tailored to its specific role, tools, and constraints while maintaining consistency with the main agent's communication style.

### Goals

1. **Reduce verbosity** while maintaining critical behavioral guidance
2. **Improve structure** for better LLM parsing
3. **Strengthen verification gates** - the model often skips verification
4. **Improve delegation decisions** - model sometimes does sub-agent work directly
5. **Better tool guidance** - clearer when to use which tool
6. **Maintain dynamic composition** - support contributor system

### Constraints

1. Must work with multiple LLM providers (Anthropic, OpenAI, Gemini, etc.)
2. Must support conditional sections based on available tools/features
3. Must remain modular - sub-agent docs are appended dynamically
4. Cannot rely on any provider-specific features in base prompt
5. Keep base prompt under ~4000 tokens

### Pain Points to Address

1. **Skipping verification**: Model often claims completion without running tests
2. **Premature execution**: Acting before fully understanding requirements
3. **Sub-agent confusion**: Not clear when to delegate vs handle directly
4. **Over-communication**: Too verbose in responses, repeated explanations
5. **Plan updates**: Often forgets to update plans as work progresses

### Must Preserve

1. The 5-phase workflow with gates
2. File operation safety rules (read before edit)
3. Delegation decision tree concept
4. Security boundaries
5. Agent mode support (Planning, AutoApprove)
6. Project instructions placeholder

---

## Questions for the Agent

1. Should we use XML tags for structured sections vs markdown headers?
2. How should we handle the dynamic tool list - document all possible tools or generate contextually?
3. Should sub-agent documentation be inline or always appended dynamically?
4. What's the optimal structure for the delegation decision tree?
5. How can we make verification gates more enforceable through prompt design?

