AI-powered terminal emulator built with Tauri 2 (Rust backend, React 19 frontend).

## About This Project

This is **Qbit's own codebase**. If you are Qbit, then you are the AI agent being developed here.
The system prompt you operate under is defined in `backend/crates/qbit-ai/src/system_prompt.rs`.
When working on this project, you have unique insight into how changes will affect your own behavior.

## Commands

```bash
# Development
just dev              # Full app (in current directory)
just dev ~/Code/foo   # Full app (opens in specified directory)
just dev-fe           # Frontend only (Vite on port 1420)

# Testing
just test             # All tests (frontend + Rust default-members)
just test-fe          # Frontend tests (Vitest, single run)
just test-watch       # Frontend tests (watch mode)
just test-rust        # Rust tests (cargo nextest, default-members only)
just test-rust-all    # Rust tests (all workspace crates incl. app/evals)
just test-e2e         # E2E tests (Playwright)
pnpm test:coverage    # Frontend coverage report

# Code Quality
just check            # All checks (biome + clippy + fmt + tests, full workspace)
just check-rust       # Fast Rust check (cargo check + fmt, default-members)
just lint-rust        # Rust lint (clippy + fmt, full workspace)
just fix              # Auto-fix frontend (biome --write)
just fmt              # Format all (frontend + Rust)

# Build
just build            # Production build
just build-rust       # Rust only (debug)

# Headless CLI mode
cargo build -p qbit
./target/debug/qbit --headless -e "prompt" --auto-approve
./target/debug/qbit -e "prompt"  # -e implies --headless
```

## Architecture

```
React Frontend (frontend/)
        |
        v (invoke / listen)
  Tauri Commands & Events
        |
        v
Rust Backend Workspace (backend/crates/) - 28 crates in 4 layers
    |
    Layer 4 (Application):
    +-- qbit (main crate - Tauri commands, CLI entry, runtime, history, CLI output)
    |
    Layer 3 (Domain):
    +-- qbit-ai (agent orchestration, planning, HITL, loop detection, tool policy, indexing)
    |
    Layer 2 (Infrastructure):
    +-- qbit-artifacts (artifact management)
    +-- qbit-context (token budget, context pruning)
    +-- qbit-evals (evaluation framework)
    +-- qbit-llm-providers (provider configuration types)
    +-- qbit-mcp (MCP client for external tool servers)
    +-- qbit-pty (terminal sessions)
    +-- qbit-session (conversation persistence)
    +-- qbit-settings (TOML config management)
    +-- qbit-shell-exec (shell execution)
    +-- qbit-sidecar (context capture)
    +-- qbit-sub-agents (sub-agent definitions and execution)
    +-- qbit-synthesis (session synthesis)
    +-- qbit-tools (tool system, registry, file ops, directory ops, AST search)
    +-- qbit-udiff (unified diff system)
    +-- qbit-web (web search, content fetching)
    +-- qbit-workflow (graph-based multi-step tasks)
    +-- rig-anthropic-vertex (Vertex AI Anthropic provider)
    +-- rig-gemini-vertex (Vertex AI Gemini provider)
    +-- rig-zai (Z.AI GLM provider)
    +-- rig-zai-anthropic (Z.AI Anthropic SSE transformer)
    +-- rig-openai-responses (OpenAI Responses adapter with explicit reasoning/text stream separation)
    |
    Layer 1 (Foundation):
    +-- qbit-core (zero internal deps)
```

## Project Structure

```
frontend/                 # React frontend
  components/
    ui/                   # shadcn/ui primitives (modify via shadcn CLI only)
    AgentChat/            # AI chat UI (messages, tool cards, approval dialogs)
    CommandBlock/         # Command history block display
    CommandPalette/       # Command palette/fuzzy finder
    DiffView/             # Unified diff visualization
    HomeView/             # Home tab view (projects list, recent directories)
      HomeView.tsx        # Main view component
      NewWorktreeModal.tsx # Modal for creating new git worktrees
      SetupProjectModal.tsx # Modal for adding new projects
    ImageModal/           # Modal for viewing expanded image attachments
    PaneContainer/        # Split pane layout system
      PaneLeaf.tsx        # Individual pane content (uses portal targets for Terminals)
    InlineTaskPlan/       # Task plan row above input
    PlanProgress/         # Alternate task plan progress visualization (not currently wired)
    SlashCommandPopup/    # Slash command popup (prompts + skills)
    SessionBrowser/       # Session management UI
    Settings/             # Settings dialog (AI, Terminal, Codebases, Advanced)
    Sidecar/              # Context capture panel
    NotificationWidget/   # Notification badge and popup (rendered in TabBar)
    StatusBar/            # Status bar component (not currently rendered in App)
    TabBar/               # Tab bar header with notifications
    Terminal/             # xterm.js terminal component with fullterm mode
      TerminalLayer.tsx   # Renders all Terminals via React portals for state persistence
    ThinkingBlock/        # Extended thinking display
    ToolCallDisplay/      # Tool execution display
    UdiffResultBlock/     # Unified diff result block
    UnifiedInput/         # Mode-switching input (terminal/agent toggle) + footer status row (agent mode + context usage badges)
      ImageAttachment.tsx # Image attachment preview and removal
    UnifiedTimeline/      # Main content view (commands + agent messages)
    WelcomeScreen/        # Initial welcome UI
    WorkflowTree/         # Multi-step workflow visualization
  hooks/
    useAiEvents.ts        # AI streaming event subscriptions (30+ event types)
    useCommandHistory.ts  # Command history management
    useCreateTerminalTab.ts # Terminal tab creation (PTY, AI init, git status)
    usePathCompletion.ts  # Path completion logic
    useSidecarEvents.ts   # Sidecar-specific event subscriptions
    useSlashCommands.ts   # Slash commands (prompts + skills discovery)
    useTauriEvents.ts     # Terminal/PTY event subscriptions
    useTerminalPortal.tsx # Terminal portal context for state persistence
    useTheme.tsx          # Theme management
  lib/
    ai.ts                 # AI-specific invoke wrappers
    indexer.ts            # Indexer invoke wrappers (codebase management)
    projects.ts           # Project configuration API
    settings.ts           # Settings invoke wrappers
    sidecar.ts            # Sidecar invoke wrappers
    tauri.ts              # Typed wrappers for PTY/shell/skills invoke() calls
    theme/                # Theme system (ThemeManager, ThemeLoader, registry)
    tools.ts              # Tool definition helpers
    utils.ts              # General utilities
  store/index.ts          # Zustand store (single file, Immer middleware)
  mocks.ts                # Tauri IPC mock adapter for browser-only development

backend/crates/           # Rust workspace (modular crate architecture)
  qbit/                   # Main application crate (Layer 4)
    src/
      ai/commands/        # AI-specific Tauri commands
      commands/           # General Tauri commands (PTY, shell, themes, files, skills)
      cli/                # CLI-specific code
        args.rs           # CLI argument parsing
        bootstrap.rs      # CLI bootstrapping
        eval.rs           # Eval runner integration
        repl.rs           # Interactive REPL mode (supports slash commands)
        runner.rs         # CLI execution runner
      indexer/            # Code indexer commands
        commands.rs       # Indexer Tauri commands
        mod.rs            # Module definition
      projects/           # Project configuration storage (~/.qbit/projects/)
        schema.rs         # ProjectConfig, ProjectCommands, WorktreeConfig
        storage.rs        # TOML file operations (list, load, save, delete)
        commands.rs       # Tauri commands for project management
        mod.rs            # Module definition
      main.rs             # Unified entry point (GUI default, --headless for CLI)
      lib.rs              # Command registration and app entry point
  qbit-evals/             # Evaluation framework crate (Layer 2)
    src/
      scenarios/          # Eval scenario definitions
      runner.rs           # Scenario execution
  qbit-core/              # Foundation crate (Layer 1, zero internal deps)
    src/
      events.rs           # Core event types
      runtime.rs          # Runtime trait definitions
      session/            # Session types (directory)
        archive.rs        # Session archival
        listing.rs        # Session listing
        message.rs        # Message types
        storage.rs        # Session storage
      hitl.rs             # HITL interfaces
      plan.rs             # Planning types
  qbit-ai/                # AI orchestration crate (Layer 3)
    src/
      agent_bridge.rs     # Bridge between Tauri and vtcode agent
      agentic_loop.rs     # Main agent execution loop (includes context compaction)
      llm_client.rs       # LLM provider abstraction
      summarizer.rs       # Context compaction summarizer
      tool_executors.rs   # Tool implementation handlers
      tool_definitions.rs # Tool schemas and configs
      system_prompt.rs    # System prompt generation (includes continuation summary)
      transcript.rs       # Transcript recording for context compaction
  qbit-sub-agents/        # Sub-agent crate (Layer 2)
    src/
      defaults.rs         # Default sub-agent configurations
      definition.rs       # Sub-agent type definitions
      executor.rs         # Sub-agent execution logic
      schemas.rs          # Sub-agent schemas
      transcript.rs       # Sub-agent transcript handling
  qbit-context/           # Context management crate (Layer 2)
    src/
      context_manager.rs  # Context window orchestration
      token_budget.rs     # Token budget tracking
      token_trunc.rs      # Token truncation utilities
  qbit-session/           # Session persistence crate (Layer 2)
    src/lib.rs            # Conversation history and archival
  qbit-web/               # Web services crate (Layer 2)
    src/
      tavily.rs           # Tavily web search integration
      web_fetch.rs        # Web content fetching
  qbit-workflow/          # Workflow crate (Layer 2)
    src/
      models.rs           # Workflow traits and types
      registry.rs         # Workflow registry
      runner.rs           # Workflow execution
      definitions/        # Built-in workflow definitions
  qbit-pty/               # PTY crate (Layer 2)
    src/
      manager.rs          # PTY session lifecycle
      parser.rs           # VTE/OSC sequence parsing
      shell.rs            # Shell integration
  qbit-sidecar/           # Context capture crate (Layer 2)
    src/
      capture.rs          # Context capture logic
      commits.rs          # Git commit tracking
      config.rs           # Sidecar configuration
      events.rs           # Sidecar event system
      processor.rs        # Event processing + state updates
      session.rs          # Session file operations
      state.rs            # Sidecar state management
  qbit-settings/          # Settings crate (Layer 2)
    src/
      schema.rs           # QbitSettings struct definitions
      loader.rs           # File loading with env var interpolation
  qbit-tools/             # Tool system crate (Layer 2)
    src/
      definitions.rs      # Tool definitions
      registry.rs         # Tool registry
      error.rs            # Tool error types
      directory_ops/      # Directory operations (merged module)
      file_ops/           # File operations (merged module)
      ast_grep/           # AST-based code search (merged module)
  rig-anthropic-vertex/   # Anthropic on Vertex AI provider
  rig-gemini-vertex/      # Gemini on Vertex AI provider

docs/                     # Documentation
  rig-evals.md            # Rust evaluation framework documentation

e2e/                      # End-to-end tests (Playwright)
  slash-commands.spec.ts  # Slash commands E2E tests (prompts + skills)
```

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `local-llm` | Local LLM via mistral.rs (Metal GPU) - **currently disabled** | No |
| `evals` | Evaluation framework for agent testing | No |

The unified binary supports both GUI and CLI modes:
- `qbit` or `qbit [path]` - Launches GUI (default)
- `qbit --headless` - Runs in headless CLI mode
- `qbit -e "prompt"` - Execute a single prompt (implies `--headless`)

## Environment Setup

Create `.env` in project root:
```bash
# Optional: for web search tool
TAVILY_API_KEY=your-key
```

Settings file: `~/.qbit/settings.toml` (auto-generated on first run, see `backend/crates/qbit-settings/src/template.toml`)

Proxy configuration (in `~/.qbit/settings.toml` or via Settings > Advanced > Proxy):
```toml
[proxy]
url = "http://proxy:8080"           # HTTP, HTTPS, or SOCKS5 proxy URL
# username = "proxy_user"           # Optional proxy auth
# password = "$PROXY_PASSWORD"      # Supports $ENV_VAR syntax
# no_proxy = "localhost,127.0.0.1"  # Comma-separated bypass list
# ca_cert_path = "/path/to/ca.pem"  # Custom CA certificate (PEM)
# accept_invalid_certs = false      # Skip TLS verification (dev only)
```

Sessions stored in: `~/.qbit/sessions/` (override with `VT_SESSION_DIR` env var)

Projects stored in: `~/.qbit/projects/`

Logs: `~/.qbit/frontend.log` and `~/.qbit/backend.log`

Workspace override: `just dev /path/to/project` or set `QBIT_WORKSPACE` env var

## Event System

### Terminal Events
| Event | Payload | Description |
|-------|---------|-------------|
| `terminal_output` | `{session_id, data}` | Raw PTY output |
| `command_block` | `CommandBlock` | Parsed command with output |

### AI Events (emitted as `ai-event`)
| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `started` | `turn_id` | Agent turn started |
| `system_hooks_injected` | `hooks` | System hooks injected into the conversation (rendered as a dedicated timeline entry) |
| `text_delta` | `delta`, `accumulated` | Streaming text chunk |
| `tool_approval_request` | `request_id`, `tool_name`, `args`, `risk_level` | Requires user approval |
| `tool_auto_approved` | `request_id`, `reason` | Auto-approved by pattern |
| `tool_result` | `request_id`, `success`, `result` | Tool execution completed |
| `reasoning` | `content` | Extended thinking content |
| `completed` | `response`, `tokens_used` | Turn finished |
| `error` | `message`, `error_type` | Error occurred |
| `workflow_*` | `workflow_id`, `step_*` | Workflow lifecycle events |
| `context_*` | utilization metrics | Context window management |
| `loop_*` | detection stats | Loop protection events |
| `compaction_started` | `tokens_before`, `messages_before` | Context compaction initiated |
| `compaction_completed` | `tokens_before`, `messages_before/after`, `summary_length` | Compaction succeeded |
| `compaction_failed` | `tokens_before`, `messages_before`, `error` | Compaction failed |

UI note: `system_hooks_injected` is persisted into the unified timeline as a `UnifiedBlock` of type `system_hook`.

## Conventions

### TypeScript/React
- Path alias: `@/*` maps to `./frontend/*`
- Components: PascalCase directories with `index.ts` barrel exports
- State: Single Zustand store with Immer middleware (`enableMapSet()` for Set/Map)
- Tauri calls: Always use typed wrappers from `lib/*.ts`, never raw `invoke()`
- Formatting: Biome (2-space indent, double quotes, semicolons, trailing commas ES5)

### Rust
- Module structure: `mod.rs` re-exports public items
- Error handling: `anyhow::Result` for commands, `thiserror` for domain errors
- Async: Tokio runtime (full features)
- Events: `app.emit("event-name", payload)` for frontend communication
- Logging: `tracing` crate (`debug!`, `info!`, `warn!`, `error!`)

### Tauri Integration
- Commands distributed across modules in main crate (`backend/crates/qbit/src/`):
  - `commands/*.rs` - PTY, shell, themes, files, skills
  - `ai/commands/*.rs` - AI agent commands
  - `settings/commands.rs` - Settings commands
  - `sidecar/commands.rs` - Sidecar commands
  - `indexer/commands.rs` - Code indexer commands
  - `commands/skills.rs` - Skill discovery and loading
- All commands registered in `lib.rs`
- Frontend listens via `@tauri-apps/api/event`

## Key Dependencies

| Purpose | Package |
|---------|---------|
| AI/LLM | rig-core |
| AI routing | rig-anthropic-vertex, rig-gemini-vertex, rig-zai, rig-openai-responses (local crates) |
| Terminal | portable-pty, vte, @xterm/xterm |
| Workflows | graph-flow |
| Web search | tavily, reqwest, readability |
| UI | React 19, shadcn/ui, Radix primitives, Tailwind v4 |
| State | Zustand + Immer |
| Markdown | react-markdown, react-syntax-highlighter, remark-gfm |
| CLI | clap |
| Serialization | serde, serde_json, toml |
| Testing | Vitest (frontend), Playwright (E2E), proptest (Rust) |

### Internal Workspace Crates
| Crate | Layer | Purpose |
|-------|-------|---------|
| qbit-core | 1 (Foundation) | Core types, traits, zero internal deps |
| qbit-artifacts | 2 (Infra) | Artifact management |
| qbit-context | 2 (Infra) | Token budget, context pruning |
| qbit-evals | 2 (Infra) | Evaluation framework |
| qbit-llm-providers | 2 (Infra) | Provider configuration types |
| qbit-mcp | 2 (Infra) | MCP client for external tool servers |
| qbit-pty | 2 (Infra) | PTY/terminal management |
| qbit-session | 2 (Infra) | Conversation persistence |
| qbit-settings | 2 (Infra) | TOML configuration management |
| qbit-shell-exec | 2 (Infra) | Shell execution |
| qbit-sidecar | 2 (Infra) | Context capture |
| qbit-sub-agents | 2 (Infra) | Sub-agent definitions and execution |
| qbit-synthesis | 2 (Infra) | Session synthesis |
| qbit-tools | 2 (Infra) | Tool system, registry, file ops, directory ops, AST search |
| qbit-udiff | 2 (Infra) | Unified diff system |
| qbit-web | 2 (Infra) | Web search, content fetching |
| qbit-workflow | 2 (Infra) | Graph-based multi-step tasks |
| rig-anthropic-vertex | 2 (Infra) | Vertex AI Anthropic provider |
| rig-gemini-vertex | 2 (Infra) | Vertex AI Gemini provider |
| rig-zai | 2 (Infra) | Z.AI GLM provider |
| rig-zai-anthropic | 2 (Infra) | Z.AI Anthropic SSE transformer |
| qbit-ai | 3 (Domain) | Agent orchestration, planning, HITL, loop detection, tool policy, indexing |
| qbit | 4 (App) | Main crate, Tauri commands, CLI, runtime, history, CLI output |

## Testing

- Frontend: Vitest + React Testing Library + jsdom
- E2E: Playwright (tests in `e2e/` directory)
- Tauri mocks: `frontend/test/mocks/tauri-event.ts` (aliased in vitest.config.ts)
- Browser mocks: `frontend/mocks.ts` (for browser-only development and E2E tests)
- Rust: Standard `cargo test` (includes proptest for property-based tests)
- Setup file: `frontend/test/setup.ts`

## Evaluations

Rust-native evaluation framework using rig for end-to-end agent capability testing.

```bash
# Run all eval scenarios
cargo run -p qbit --features evals -- --eval

# Run specific scenario
cargo run -p qbit --features evals -- --eval --scenario bug-fix

# List available scenarios
cargo run -p qbit --features evals -- --list-scenarios
```

See `docs/rig-evals.md` for complete documentation.

## Fullterm Mode

The terminal supports two render modes controlled by `RenderMode` in the store:
- `timeline`: Default mode showing parsed command blocks in the unified timeline
- `fullterm`: Full xterm.js terminal for interactive applications

**Auto-detection via ANSI sequences**: The terminal automatically switches to fullterm mode when it detects an application entering the alternate screen buffer (via ANSI CSI sequence `ESC[?1049h`). It switches back when the application exits the alternate screen buffer (`ESC[?1049l`). This covers most TUI apps like vim, htop, less, tmux, etc.

**Fallback list**: Some apps (like AI coding agents) don't use the alternate screen buffer but still need fullterm mode. Built-in defaults:
- AI tools: claude, cc, codex, cdx, aider, cursor, gemini

**Custom commands**: Users can add additional commands via `~/.qbit/settings.toml`:
```toml
[terminal]
fullterm_commands = ["my-custom-tui", "another-app"]
```
These are merged with the built-in defaults.

**UI**: Status bar shows "Full Term" indicator when in fullterm mode. Toggle available via Command Palette.

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

**Key hooks**:
- `useTerminalPortalTarget(sessionId)` - Used by `PaneLeaf` to register its target element
- `useTerminalPortalTargets()` - Used by `TerminalLayer` to get all registered targets

## Context Compaction

When the agent's context window approaches capacity, automatic compaction is triggered:

1. **Detection**: `ContextManager::should_compact()` checks token usage against model limits
2. **Transcript**: Conversation history is read from `~/.qbit/transcripts/{session_id}/transcript.json` (JSONL format — one JSON object per line, with backward-compatible JSON array fallback for legacy files)
3. **Summarization**: A dedicated summarizer LLM call generates a concise summary. The summarizer uses XML tag extraction (`<analysis>`/`<summary>`) to separate internal reasoning from the final summary. Reasoning/thinking from agent turns is excluded from summarizer input to save context budget.
4. **Reset**: Message history is cleared and replaced with:
   - A context summary (wrapped in `[Context Summary - ...]` markers)
   - The most recent user message (to maintain conversational flow)
5. **Continuation**: The summary is added to the system prompt via `## Continuation` section

**Transcript format**: Events are written as append-only JSONL (one timestamped JSON object per line). Streaming events (`TextDelta`, `Reasoning`, `ToolOutputChunk`) and sub-agent internal events (`SubAgentToolRequest`, `SubAgentToolResult`) are filtered out — their content is captured in aggregate by `Completed` and `ToolResult` events. Filtering is centralized in `transcript::should_transcript()`.

**Artifacts saved**:
- `~/.qbit/artifacts/compaction/summarizer-input-{timestamp}.md` - Input to summarizer
- `~/.qbit/artifacts/summaries/summary-{timestamp}.md` - Generated summary

**Key files**:
- `qbit-ai/src/agentic_loop.rs` - `maybe_compact()`, `perform_compaction()`, `apply_compaction()`
- `qbit-ai/src/transcript.rs` - `TranscriptWriter`, `should_transcript()`, `read_transcript()`, `format_for_summarizer()`
- `qbit-ai/src/summarizer.rs` - Summarizer LLM call, `extract_summary_text()`
- `qbit-ai/src/system_prompt.rs` - `append_continuation_summary()`, `update_continuation_summary()`
- `qbit-context/src/context_manager.rs` - `CompactionState`, `should_compact()`

**Configuration** (in `~/.qbit/settings.toml`):
```toml
[ai]
summarizer_model = "claude-3-5-haiku-latest"  # Optional: model for summarization
```

## Agent Skills

Agent Skills follow the [agentskills.io](https://agentskills.io) specification. Skills are directory-based extensions that provide specialized instructions to the AI agent.

**Directory Structure**:
```
~/.qbit/skills/           # Global skills
  skill-name/
    SKILL.md              # Required: YAML frontmatter + instructions
    scripts/              # Optional: executable scripts
    references/           # Optional: reference documents
    assets/               # Optional: assets

<project>/.qbit/skills/   # Local skills (override global)
```

**SKILL.md Format**:
```markdown
---
name: skill-name
description: Short description (1-1024 chars)
license: MIT                        # Optional
compatibility: Claude 3.5+          # Optional (1-500 chars)
allowed-tools: read_file write_file # Optional: space-delimited
metadata:                           # Optional: arbitrary key-value pairs
  author: your-name
  version: 1.0.0
---

Your skill instructions here. This markdown content is sent to the AI
when the skill is invoked via `/<skill-name>` in the input field.
```

**Skill Name Rules**:
- 1-64 characters
- Lowercase alphanumeric + hyphens only
- No consecutive hyphens
- No leading/trailing hyphens

**Discovery**:
- Skills are discovered from both global and local directories
- Local skills override global skills with the same name
- Prompts (`.qbit/prompts/*.md`) take precedence over skills with the same name

**Usage**:
- Type `/` in the input field to see available prompts and skills
- Skills display with a puzzle icon and their description
- Tab completes the command name, Enter executes
- Arguments can be appended: `/skill-name some arguments`

**Key Files**:
- `backend/crates/qbit/src/commands/skills.rs` - Skill discovery, parsing, Tauri commands
- `frontend/components/SlashCommandPopup/SlashCommandPopup.tsx` - Unified popup UI
- `frontend/hooks/useSlashCommands.ts` - Merges prompts and skills into unified list
- `frontend/lib/tauri.ts` - Skill invoke wrappers (`listSkills`, `readSkillBody`, etc.)

## Gotchas

- Shell integration uses OSC 133 sequences; test with real shell sessions
- The `ui/` components are shadcn-generated; modify via `pnpm dlx shadcn@latest`, not directly
- Streaming blocks use interleaved text/tool pattern; see `streamingBlocks` in store
- Feature flags are mutually exclusive: `--features tauri` (default) vs `--features cli`

## Adding New Features

### New Tauri Command
1. Create function in appropriate file in `backend/crates/qbit/src/`:
   - `commands/*.rs` for general commands
   - `ai/commands/*.rs` for AI-specific commands
   - `settings/commands.rs`, `sidecar/commands.rs`, `indexer/commands.rs` for domain commands
2. Annotate with `#[tauri::command]`
3. Add to `tauri::generate_handler![]` in `lib.rs`
4. Add typed wrapper in `frontend/lib/*.ts`

### New AI Tool
1. Add tool definition in `backend/crates/qbit-ai/src/tool_definitions.rs`
2. Add executor in `backend/crates/qbit-ai/src/tool_executors.rs`
3. Register in the tool registry

### New AI Event
1. Add variant to `AiEvent` enum in `backend/crates/qbit-core/src/events.rs`
2. Emit via `app.emit("ai-event", event)`
3. Handle in `frontend/hooks/useAiEvents.ts`

### New Infrastructure Crate
1. Create new crate in `backend/crates/` following Layer 2 pattern
2. Add to workspace members in `backend/Cargo.toml`
3. Add as dependency to consuming crates (typically qbit-ai or qbit)
4. Re-export public API through qbit crate if needed for Tauri commands
