# Development Guide

## Prerequisites

- macOS (primary development platform)
- Node.js 20+
- pnpm
- Rust (stable)
- [just](https://github.com/casey/just) command runner

## Getting Started

```bash
git clone https://github.com/qbit-ai/qbit.git
cd qbit
just install
just dev
```

## Commands

### Development

```bash
just dev              # Full app with hot reload
just dev ~/Code/foo   # Full app (opens in specified directory)
just dev-fe           # Frontend only (Vite on port 1420)
```

### Testing

```bash
just test             # All tests (frontend + Rust)
just test-fe          # Frontend tests (Vitest, single run)
just test-watch       # Frontend tests (watch mode)
just test-rust        # Rust tests
just test-e2e         # E2E tests (Playwright)
pnpm test:coverage    # Frontend coverage report
```

### Code Quality

```bash
just check            # All checks (biome + clippy + fmt)
just fix              # Auto-fix frontend (biome --write)
just fmt              # Format all (frontend + Rust)
```

### Build

```bash
just build            # Production build
just build-rust       # Rust only (debug)
```

Run `just --list` for all available commands.

## Frontend-Only Development

The frontend runs standalone with full mock support:

```bash
just dev-fe
```

This spins up Vite with a mock Tauri environment. Useful for rapid UI iteration without LLM costs.

## CLI Binary

Build and run the headless CLI:

```bash
cargo build -p qbit --features cli --no-default-features --bin qbit-cli
./target/debug/qbit-cli -e "prompt" --auto-approve
```

## Feature Flags

| Flag | Description | Default |
|------|-------------|---------|
| `tauri` | GUI application (Tauri window) | Yes |
| `cli` | Headless CLI binary | No |
| `local-llm` | Local LLM via mistral.rs (Metal GPU) - **currently disabled** | No |
| `evals` | Evaluation framework for agent testing | No |

Flags `tauri` and `cli` are mutually exclusive.

## Testing

### Frontend

- **Framework**: Vitest + React Testing Library + jsdom
- **Setup file**: `frontend/test/setup.ts`
- **Tauri mocks**: `frontend/test/mocks/tauri-event.ts` (aliased in vitest.config.ts)
- **Browser mocks**: `frontend/mocks.ts` (for browser-only development and E2E tests)

### E2E

- **Framework**: Playwright
- **Location**: `e2e/` directory

### Rust

- Standard `cargo test`
- Property-based testing with proptest

## Evaluations

Rust-native evaluation framework for end-to-end agent capability testing.

```bash
# Run all eval scenarios
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --eval

# Run specific scenario
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --eval --scenario bug-fix

# List available scenarios
cargo run --no-default-features --features evals,cli --bin qbit-cli -- --list-scenarios
```

See [rig-evals.md](./rig-evals.md) for complete documentation.

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
4. Add event handler in `frontend/hooks/useAiEvents.ts`

### New AI Event

1. Add variant to `AiEvent` enum in `backend/crates/qbit-core/src/events.rs`
2. Emit via `app.emit("ai-event", event)`
3. Handle in `frontend/hooks/useAiEvents.ts`

### New Infrastructure Crate

1. Create new crate in `backend/crates/` following Layer 2 pattern
2. Add to workspace members in `backend/Cargo.toml`
3. Add as dependency to consuming crates (typically qbit-ai or qbit)
4. Re-export public API through qbit crate if needed for Tauri commands

## Gotchas

- Shell integration uses OSC 133 sequences; test with real shell sessions
- The `ui/` components are shadcn-generated; modify via `pnpm dlx shadcn@latest`, not directly
- vtcode-core is an external dependency (not in `backend/crates/`); check crates.io for docs
- Streaming blocks use interleaved text/tool pattern; see `streamingBlocks` in store
- Feature flags are mutually exclusive: `--features tauri` (default) vs `--features cli`
