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
just test             # All tests (frontend + Rust)
just test-fe          # Frontend tests (Vitest, single run)
just test-watch       # Frontend tests (watch mode)
just test-rust        # Rust tests
just test-e2e         # E2E tests (Playwright)
pnpm test:coverage    # Frontend coverage report

# Code Quality
just check            # All checks (biome + clippy + fmt)
just fix              # Auto-fix frontend (biome --write)
just fmt              # Format all (frontend + Rust)

# Build
just build            # Production build
just build-rust       # Rust only (debug)

# CLI Binary (headless mode)
cargo build -p qbit --features cli,local-tools --no-default-features --bin qbit-cli
./target/debug/qbit-cli -e "prompt" --auto-approve
```

## Architecture

```
React Frontend (frontend/)
        |
        v (invoke / listen)
  Tauri Commands & Events
        |
        v
Rust Backend Workspace (backend/crates/) - 29 crates in 4 layers
    |
    Layer 4 (Application):
    +-- qbit (main crate - Tauri commands, CLI entry)
    |
    Layer 3 (Domain):
    +-- qbit-ai (agent orchestration - depends on all Layer 2)
    |       +-- vtcode-core (external crate)
    |
    Layer 2 (Infrastructure):
    +-- qbit-artifacts (artifact management)
    +-- qbit-cli-output (CLI output formatting)
    +-- qbit-context (token budget, context pruning)
    +-- qbit-directory-ops (directory operations)
    +-- qbit-evals (evaluation framework)
    +-- qbit-file-ops (file operations)
    +-- qbit-hitl (human-in-the-loop approval)
    +-- qbit-indexer (code indexing state)
    +-- qbit-llm-providers (provider configuration types)
    +-- qbit-loop-detection (agent loop protection)
    +-- qbit-planner (planning system)
    +-- qbit-pty (terminal sessions)
    +-- qbit-runtime (Tauri/CLI runtime abstraction)
    +-- qbit-session (conversation persistence)
    +-- qbit-settings (TOML config management)
    +-- qbit-shell-exec (shell execution)
    +-- qbit-sidecar (context capture)
    +-- qbit-sub-agents (sub-agent definitions and execution)
    +-- qbit-synthesis (session synthesis)
    +-- qbit-tool-policy (tool access control)
    +-- qbit-tools (tool system, registry)
    +-- qbit-udiff (unified diff system)
    +-- qbit-web (web search, content fetching)
    +-- qbit-workflow (graph-based multi-step tasks)
    +-- rig-anthropic-vertex (Vertex AI provider)
    +-- rig-zai (Z.AI GLM provider)
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
    PlanProgress/         # Task plan progress visualization
    SessionBrowser/       # Session management UI
    Settings/             # Settings dialog (AI, Terminal, Codebases, Advanced)
    Sidecar/              # Context capture panel
    StatusBar/            # Status bar/footer
    Terminal/             # xterm.js terminal component with fullterm mode
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
    useSlashCommands.ts   # Slash command parsing
    useTauriEvents.ts     # Terminal/PTY event subscriptions
    useTheme.tsx          # Theme management
  lib/
    ai.ts                 # AI-specific invoke wrappers
    indexer.ts            # Indexer invoke wrappers (codebase management)
    settings.ts           # Settings invoke wrappers
    sidecar.ts            # Sidecar invoke wrappers
    tauri.ts              # Typed wrappers for PTY/shell invoke() calls
    theme/                # Theme system (ThemeManager, ThemeLoader, registry)
    tools.ts              # Tool definition helpers
    utils.ts              # General utilities
  store/index.ts          # Zustand store (single file, Immer middleware)
  mocks.ts                # Tauri IPC mock adapter for browser-only development

backend/crates/           # Rust workspace (modular crate architecture)
  qbit/                   # Main application crate (Layer 4)
    src/
      ai/commands/        # AI-specific Tauri commands
      commands/           # General Tauri commands (PTY, shell, themes, files)
      cli/                # CLI-specific code (args, runner, output)
      evals/              # Evaluation framework with scenarios
      bin/qbit-cli.rs     # Headless CLI binary entry point
      lib.rs              # Command registration and app entry point
  qbit-core/              # Foundation crate (Layer 1, zero internal deps)
    src/
      events.rs           # Core event types
      runtime.rs          # Runtime trait definitions
      session.rs          # Session types
      hitl.rs             # HITL interfaces
      plan.rs             # Planning types
  qbit-ai/                # AI orchestration crate (Layer 3)
    src/
      agent_bridge.rs     # Bridge between Tauri and vtcode agent
      agentic_loop.rs     # Main agent execution loop
      llm_client.rs       # LLM provider abstraction
      tool_executors.rs   # Tool implementation handlers
      tool_definitions.rs # Tool schemas and configs
      sub_agent.rs        # Sub-agent definitions and registry
      sub_agent_executor.rs # Sub-agent execution
      system_prompt.rs    # System prompt generation
  qbit-context/           # Context management crate (Layer 2)
    src/
      context_manager.rs  # Context window orchestration
      context_pruner.rs   # Semantic context pruning
      token_budget.rs     # Token budget tracking
      token_trunc.rs      # Token truncation utilities
  qbit-hitl/              # HITL crate (Layer 2)
    src/
      approval_recorder.rs # Approval pattern learning
  qbit-loop-detection/    # Loop protection crate (Layer 2)
    src/lib.rs            # Loop detection and prevention
  qbit-session/           # Session persistence crate (Layer 2)
    src/lib.rs            # Conversation history and archival
  qbit-tool-policy/       # Tool policy crate (Layer 2)
    src/lib.rs            # Tool access control and constraints
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
      session.rs          # Session file operations
      processor.rs        # Event processing + state updates
      artifacts.rs        # Artifact management
      synthesis.rs        # Session synthesis
      events.rs           # Sidecar event system
  qbit-settings/          # Settings crate (Layer 2)
    src/
      schema.rs           # QbitSettings struct definitions
      loader.rs           # File loading with env var interpolation
  qbit-indexer/           # Indexer crate (Layer 2)
    src/
      state.rs            # Indexer state management
      paths.rs            # Index path resolution
  qbit-tools/             # Tool system crate (Layer 2)
    src/
      definitions.rs      # Tool definitions
      registry.rs         # Tool registry (vtcode-core replacement)
      file_ops.rs         # File operations
      directory_ops.rs    # Directory operations
      shell.rs            # Shell execution
      udiff/              # Unified diff system
      planner/            # Planning system
  qbit-runtime/           # Runtime crate (Layer 2)
    src/
      tauri.rs            # Tauri-specific runtime
      cli.rs              # CLI-specific runtime
  rig-anthropic-vertex/   # Anthropic on Vertex AI provider

docs/                     # Documentation
  rig-evals.md            # Rust evaluation framework documentation
  cli-plan.md             # CLI development roadmap

e2e/                      # End-to-end tests (Playwright)
```

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `tauri` | GUI application (Tauri window) | Yes |
| `cli` | Headless CLI binary | No |
| `local-tools` | Local tool/session implementations (migration from vtcode-core) | Yes |
| `local-llm` | Local LLM via mistral.rs (Metal GPU) - **currently disabled** | No |
| `evals` | Evaluation framework for agent testing | No |

Flags `tauri` and `cli` are mutually exclusive.

## Environment Setup

Create `.env` in project root:
```bash
# Required for Vertex AI (or set in ~/.qbit/settings.toml)
GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
VERTEX_AI_PROJECT_ID=your-project-id
VERTEX_AI_LOCATION=us-east5

# Optional: for web search tool
TAVILY_API_KEY=your-key
```

Settings file: `~/.qbit/settings.toml` (auto-generated on first run, see `backend/crates/qbit-settings/src/template.toml`)

Sessions stored in: `~/.qbit/sessions/` (override with `VT_SESSION_DIR` env var)

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
  - `commands/*.rs` - PTY, shell, themes, files
  - `ai/commands/*.rs` - AI agent commands
  - `settings/commands.rs` - Settings commands
  - `sidecar/commands.rs` - Sidecar commands
  - `indexer/commands.rs` - Code indexer commands
- All commands registered in `lib.rs`
- Frontend listens via `@tauri-apps/api/event`

## Key Dependencies

| Purpose | Package |
|---------|---------|
| AI/LLM | vtcode-core (external), rig-core |
| AI routing | rig-anthropic-vertex, rig-zai (local crates) |
| Terminal | portable-pty, vte, @xterm/xterm |
| Workflows | graph-flow |
| Web search | tavily, reqwest, readability |
| UI | React 19, shadcn/ui, Radix primitives, Tailwind v4 |
| State | Zustand + Immer |
| Markdown | react-markdown, react-syntax-highlighter, remark-gfm |
| CLI | clap |
| Serialization | serde, serde_json, toml |
| Testing | Vitest (frontend), Playwright (E2E), proptest (Rust) |

### Internal Workspace Crates (29 total)
| Crate | Layer | Purpose |
|-------|-------|---------|
| qbit-core | 1 (Foundation) | Core types, traits, zero internal deps |
| qbit-artifacts | 2 (Infra) | Artifact management |
| qbit-cli-output | 2 (Infra) | CLI output formatting |
| qbit-context | 2 (Infra) | Token budget, context pruning |
| qbit-directory-ops | 2 (Infra) | Directory operations |
| qbit-evals | 2 (Infra) | Evaluation framework |
| qbit-file-ops | 2 (Infra) | File operations |
| qbit-hitl | 2 (Infra) | Human-in-the-loop approval system |
| qbit-indexer | 2 (Infra) | Code indexing state |
| qbit-llm-providers | 2 (Infra) | Provider configuration types |
| qbit-loop-detection | 2 (Infra) | Agent loop protection |
| qbit-planner | 2 (Infra) | Planning system |
| qbit-pty | 2 (Infra) | PTY/terminal management |
| qbit-runtime | 2 (Infra) | Tauri/CLI runtime abstraction |
| qbit-session | 2 (Infra) | Conversation persistence |
| qbit-settings | 2 (Infra) | TOML configuration management |
| qbit-shell-exec | 2 (Infra) | Shell execution |
| qbit-sidecar | 2 (Infra) | Context capture |
| qbit-sub-agents | 2 (Infra) | Sub-agent definitions and execution |
| qbit-synthesis | 2 (Infra) | Session synthesis |
| qbit-tool-policy | 2 (Infra) | Tool access control |
| qbit-tools | 2 (Infra) | Tool system and registry |
| qbit-udiff | 2 (Infra) | Unified diff system |
| qbit-web | 2 (Infra) | Web search, content fetching |
| qbit-workflow | 2 (Infra) | Graph-based multi-step tasks |
| rig-anthropic-vertex | 2 (Infra) | Vertex AI Anthropic provider |
| rig-zai | 2 (Infra) | Z.AI GLM provider |
| qbit-ai | 3 (Domain) | Agent orchestration |
| qbit | 4 (App) | Main crate, Tauri commands, CLI |

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
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --eval

# Run specific scenario
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --eval --scenario bug-fix

# List available scenarios
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --list-scenarios
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

## Gotchas

- Shell integration uses OSC 133 sequences; test with real shell sessions
- The `ui/` components are shadcn-generated; modify via `pnpm dlx shadcn@latest`, not directly
- vtcode-core is an external dependency (not in `backend/crates/`); check crates.io for docs
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
