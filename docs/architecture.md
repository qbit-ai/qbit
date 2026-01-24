# Architecture

Qbit is built with Tauri 2 (Rust backend, React 19 frontend).

## High-Level Overview

```
React Frontend (frontend/)
        |
        v (invoke / listen)
  Tauri Commands & Events
        |
        v
Rust Backend Workspace (backend/crates/) - 35 crates in 4 layers
```

## Crate Layers

The Rust workspace follows strict architectural layers where each layer can only depend on layers below it.

### Layer 1: Foundation

| Crate | Purpose |
|-------|---------|
| `qbit-core` | Core types, traits, errors. Zero internal dependencies. |

### Layer 2: Infrastructure

| Crate | Purpose |
|-------|---------|
| `qbit-artifacts` | Artifact management |
| `qbit-ast-grep` | AST-based code search |
| `qbit-cli-output` | CLI output formatting |
| `qbit-context` | Token budget, context pruning |
| `qbit-directory-ops` | Directory operations |
| `qbit-evals` | Evaluation framework |
| `qbit-file-ops` | File operations |
| `qbit-hitl` | Human-in-the-loop approval system |
| `qbit-indexer` | Code indexing state |
| `qbit-llm-providers` | Provider configuration types |
| `qbit-loop-detection` | Agent loop protection |
| `qbit-planner` | Planning system |
| `qbit-pty` | PTY/terminal management |
| `qbit-runtime` | Tauri/CLI runtime abstraction |
| `qbit-session` | Conversation persistence |
| `qbit-settings` | TOML configuration management |
| `qbit-shell-exec` | Shell execution |
| `qbit-sidecar` | Context capture |
| `qbit-sub-agents` | Sub-agent definitions and execution |
| `qbit-synthesis` | Session synthesis |
| `qbit-tool-policy` | Tool access control |
| `qbit-tools` | Tool system and registry |
| `qbit-udiff` | Unified diff system |
| `qbit-web` | Web search, content fetching |
| `qbit-workflow` | Graph-based multi-step tasks |
| `rig-anthropic-vertex` | Vertex AI Anthropic provider |
| `rig-openai-responses` | OpenAI Responses API adapter |
| `rig-zai` | Z.AI GLM provider |
| `rig-zai-anthropic` | Z.AI Anthropic SSE transformer |

### Layer 3: Domain

| Crate | Purpose |
|-------|---------|
| `qbit-ai` | Agent orchestration. Depends on infrastructure crates and vtcode-core (external). |

### Layer 4: Application

| Crate | Purpose |
|-------|---------|
| `qbit` | Main crate. Tauri commands, CLI entry point. |

## Project Structure

### Frontend (`frontend/`)

```
frontend/
  components/
    ui/                   # shadcn/ui primitives (modify via shadcn CLI only)
    AgentChat/            # AI chat UI (messages, tool cards, approval dialogs)
    CommandBlock/         # Command history block display
    CommandPalette/       # Command palette/fuzzy finder
    DiffView/             # Unified diff visualization
    PaneContainer/        # Split pane layout system
      PaneLeaf.tsx        # Individual pane content (uses portal targets)
    InlineTaskPlan/       # Task plan row above input
    SlashCommandPopup/    # Slash command popup (prompts + skills)
    SessionBrowser/       # Session management UI
    Settings/             # Settings dialog (AI, Terminal, Codebases, Advanced)
    Sidecar/              # Context capture panel
    NotificationWidget/   # Notification badge and popup
    TabBar/               # Tab bar header with notifications
    Terminal/             # xterm.js terminal component with fullterm mode
      TerminalLayer.tsx   # Renders all Terminals via React portals
    ThinkingBlock/        # Extended thinking display
    ToolCallDisplay/      # Tool execution display
    UdiffResultBlock/     # Unified diff result block
    UnifiedInput/         # Mode-switching input (terminal/agent toggle)
    UnifiedTimeline/      # Main content view (commands + agent messages)
    WelcomeScreen/        # Initial welcome UI
    WorkflowTree/         # Multi-step workflow visualization
  hooks/
    useAiEvents.ts        # AI streaming event subscriptions (30+ event types)
    useCommandHistory.ts  # Command history management
    usePathCompletion.ts  # Path completion logic
    useSidecarEvents.ts   # Sidecar-specific event subscriptions
    useSlashCommands.ts   # Slash commands (prompts + skills discovery)
    useTauriEvents.ts     # Terminal/PTY event subscriptions
    useTerminalPortal.tsx # Terminal portal context for state persistence
    useTheme.tsx          # Theme management
  lib/
    ai.ts                 # AI-specific invoke wrappers
    indexer.ts            # Indexer invoke wrappers
    settings.ts           # Settings invoke wrappers
    sidecar.ts            # Sidecar invoke wrappers
    tauri.ts              # Typed wrappers for PTY/shell/skills
    theme/                # Theme system (ThemeManager, ThemeLoader, registry)
    tools.ts              # Tool definition helpers
    utils.ts              # General utilities
  store/index.ts          # Zustand store (single file, Immer middleware)
  mocks.ts                # Tauri IPC mock adapter for browser-only development
```

### Backend (`backend/crates/`)

```
backend/crates/
  qbit/                   # Main application crate (Layer 4)
    src/
      ai/commands/        # AI-specific Tauri commands
      commands/           # General Tauri commands (PTY, shell, themes, files, skills)
      cli/                # CLI-specific code
        args.rs           # CLI argument parsing
        bootstrap.rs      # CLI bootstrapping
        eval.rs           # Eval runner integration
        repl.rs           # Interactive REPL mode
        runner.rs         # CLI execution runner
      bin/qbit-cli.rs     # Headless CLI binary entry point
      lib.rs              # Command registration and app entry point
  qbit-ai/                # AI orchestration crate (Layer 3)
    src/
      agent_bridge.rs     # Bridge between Tauri and vtcode agent
      agentic_loop.rs     # Main agent execution loop
      llm_client.rs       # LLM provider abstraction
      summarizer.rs       # Context compaction summarizer
      tool_executors.rs   # Tool implementation handlers
      tool_definitions.rs # Tool schemas and configs
      system_prompt.rs    # System prompt generation
      transcript.rs       # Transcript recording for context compaction
  qbit-core/              # Foundation crate (Layer 1)
    src/
      events.rs           # Core event types
      runtime.rs          # Runtime trait definitions
      session/            # Session types
      hitl.rs             # HITL interfaces
      plan.rs             # Planning types
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | Tauri 2 |
| Frontend | React 19, TypeScript, Vite 7, Tailwind v4 |
| State | Zustand 5 + Immer |
| Terminal | xterm.js, portable-pty, vte |
| UI | shadcn/ui, Radix primitives |
| AI | rig-core, vtcode-core |
| Workflows | graph-flow |

## Key Dependencies

| Purpose | Package |
|---------|---------|
| AI/LLM | vtcode-core (external), rig-core |
| AI routing | rig-anthropic-vertex, rig-zai, rig-openai-responses (local) |
| Terminal | portable-pty, vte, @xterm/xterm |
| Workflows | graph-flow |
| Web search | tavily, reqwest, readability |
| UI | React 19, shadcn/ui, Radix primitives, Tailwind v4 |
| State | Zustand + Immer |
| Markdown | react-markdown, react-syntax-highlighter, remark-gfm |
| CLI | clap |
| Serialization | serde, serde_json, toml |
| Testing | Vitest (frontend), Playwright (E2E), proptest (Rust) |

## Terminal Portal Architecture

Terminals use React portals to persist state across pane structure changes (splits, closes). This prevents xterm.js instances from being unmounted/remounted when the pane tree is restructured.

**Components**:
- `TerminalPortalProvider` (in `useTerminalPortal.tsx`) - Wraps the app, maintains registry of portal targets
- `TerminalLayer` (in `Terminal/TerminalLayer.tsx`) - Renders all Terminals at a stable position using `createPortal`
- `PaneLeaf` - Registers a portal target element via `useTerminalPortalTarget` hook

**Flow**:
1. `PaneLeaf` mounts and registers its portal target element with the provider
2. `TerminalLayer` renders Terminal components, each portaled into its registered target
3. When pane structure changes (split/close), Terminals stay mounted because they're rendered at the provider level
4. Only the portal targets move; Terminal instances (and xterm.js state) are preserved

## Context Compaction

When the agent's context window approaches capacity, automatic compaction is triggered:

1. **Detection**: `ContextManager::should_compact()` checks token usage against model limits
2. **Transcript**: Conversation history is read from `~/.qbit/transcripts/{session_id}/`
3. **Summarization**: A dedicated summarizer LLM call generates a concise summary
4. **Reset**: Message history is cleared and replaced with the summary
5. **Continuation**: The summary is added to the system prompt via `## Continuation` section

**Key files**:
- `qbit-ai/src/agentic_loop.rs` - `maybe_compact()`, `perform_compaction()`, `apply_compaction()`
- `qbit-ai/src/summarizer.rs` - Summarizer LLM call
- `qbit-ai/src/system_prompt.rs` - `append_continuation_summary()`, `update_continuation_summary()`
- `qbit-context/src/context_manager.rs` - `CompactionState`, `should_compact()`
