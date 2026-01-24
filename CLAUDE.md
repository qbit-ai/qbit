AI-powered terminal emulator built with Tauri 2 (Rust backend, React 19 frontend).

## About This Project

This is **Qbit's own codebase**. If you are Qbit, then you are the AI agent being developed here.
The system prompt you operate under is defined in `backend/crates/qbit-ai/src/system_prompt.rs`.
When working on this project, you have unique insight into how changes will affect your own behavior.

## Quick Reference

| Resource | Location |
|----------|----------|
| [Architecture](docs/architecture.md) | Crate layers, project structure, tech stack |
| [Development](docs/development.md) | Commands, testing, adding features |
| [Configuration](docs/configuration.md) | Providers, environment, settings |
| [Event System](docs/event-system.md) | Terminal and AI event reference |
| [Contributing](docs/contributing.md) | Code conventions, commit format |
| [Agent Skills](docs/agent-skills.md) | Custom skill development |
| [Evaluations](docs/rig-evals.md) | Agent testing framework |

## Commands

```bash
# Development
just dev              # Full app (in current directory)
just dev ~/Code/foo   # Full app (opens in specified directory)
just dev-fe           # Frontend only (Vite on port 1420)

# Testing
just test             # All tests (frontend + Rust)
just test-fe          # Frontend tests (Vitest)
just test-rust        # Rust tests
just test-e2e         # E2E tests (Playwright)

# Code Quality
just check            # All checks (biome + clippy + fmt)
just fix              # Auto-fix frontend (biome --write)
just fmt              # Format all (frontend + Rust)

# Build
just build            # Production build

# CLI Binary (headless mode)
cargo build -p qbit --features cli --no-default-features --bin qbit-cli
./target/debug/qbit-cli -e "prompt" --auto-approve
```

## Architecture Overview

```
React Frontend (frontend/)
        |
        v (invoke / listen)
  Tauri Commands & Events
        |
        v
Rust Backend Workspace (backend/crates/) - 35 crates in 4 layers
    Layer 4: qbit (main crate - Tauri commands, CLI)
    Layer 3: qbit-ai (agent orchestration)
    Layer 2: Infrastructure crates (20+ crates)
    Layer 1: qbit-core (foundation, zero deps)
```

See [Architecture](docs/architecture.md) for complete crate listing and project structure.

## Key Locations

### Frontend

| Path | Purpose |
|------|---------|
| `frontend/components/` | UI components (shadcn/ui in `ui/`, custom elsewhere) |
| `frontend/hooks/useAiEvents.ts` | AI streaming event subscriptions |
| `frontend/lib/*.ts` | Typed Tauri invoke wrappers |
| `frontend/store/index.ts` | Zustand store with Immer |

### Backend

| Path | Purpose |
|------|---------|
| `backend/crates/qbit/src/commands/` | Tauri commands (PTY, shell, themes) |
| `backend/crates/qbit/src/ai/commands/` | AI-specific Tauri commands |
| `backend/crates/qbit-ai/src/tool_definitions.rs` | Tool schemas |
| `backend/crates/qbit-ai/src/tool_executors.rs` | Tool implementations |
| `backend/crates/qbit-ai/src/system_prompt.rs` | System prompt generation |
| `backend/crates/qbit-core/src/events.rs` | Event type definitions |

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `tauri` | GUI application | Yes |
| `cli` | Headless CLI binary | No |
| `evals` | Evaluation framework | No |

Flags `tauri` and `cli` are mutually exclusive.

## Conventions

### TypeScript/React

- Path alias: `@/*` maps to `./frontend/*`
- Components: PascalCase directories with `index.ts` barrel exports
- State: Single Zustand store with Immer (`enableMapSet()` for Set/Map)
- Tauri calls: Always use typed wrappers from `lib/*.ts`
- Formatting: Biome (2-space indent, double quotes, semicolons)

### Rust

- Module structure: `mod.rs` re-exports public items
- Error handling: `anyhow::Result` for commands, `thiserror` for domain errors
- Async: Tokio runtime (full features)
- Events: `app.emit("event-name", payload)` for frontend communication
- Logging: `tracing` crate (`debug!`, `info!`, `warn!`, `error!`)

See [Contributing](docs/contributing.md) for full guidelines.

## Adding Features

### New Tauri Command

1. Create function in `backend/crates/qbit/src/commands/*.rs` or `ai/commands/*.rs`
2. Annotate with `#[tauri::command]`
3. Add to `tauri::generate_handler![]` in `lib.rs`
4. Add typed wrapper in `frontend/lib/*.ts`

### New AI Tool

1. Add tool definition in `qbit-ai/src/tool_definitions.rs`
2. Add executor in `qbit-ai/src/tool_executors.rs`
3. Register in the tool registry

### New AI Event

1. Add variant to `AiEvent` enum in `qbit-core/src/events.rs`
2. Emit via `app.emit("ai-event", event)`
3. Handle in `frontend/hooks/useAiEvents.ts`

See [Development](docs/development.md) for detailed instructions.

## Gotchas

- Shell integration uses OSC 133 sequences; test with real shell sessions
- The `ui/` components are shadcn-generated; modify via `pnpm dlx shadcn@latest`, not directly
- vtcode-core is an external dependency; check crates.io for docs
- Streaming blocks use interleaved text/tool pattern; see `streamingBlocks` in store
- Feature flags are mutually exclusive: `--features tauri` (default) vs `--features cli`

## Environment

Settings file: `~/.qbit/settings.toml`
Sessions: `~/.qbit/sessions/`
Logs: `~/.qbit/frontend.log`, `~/.qbit/backend.log`

See [Configuration](docs/configuration.md) for all options.
