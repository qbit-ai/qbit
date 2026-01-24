# Contributing

## Code Conventions

### TypeScript/React

- **Path alias**: `@/*` maps to `./frontend/*`
- **Components**: PascalCase directories with `index.ts` barrel exports
- **State**: Single Zustand store with Immer middleware (`enableMapSet()` for Set/Map)
- **Tauri calls**: Always use typed wrappers from `lib/*.ts`, never raw `invoke()`
- **Formatting**: Biome (2-space indent, double quotes, semicolons, trailing commas ES5)

### Rust

- **Module structure**: `mod.rs` re-exports public items
- **Error handling**: `anyhow::Result` for commands, `thiserror` for domain errors
- **Async**: Tokio runtime (full features)
- **Events**: `app.emit("event-name", payload)` for frontend communication
- **Logging**: `tracing` crate (`debug!`, `info!`, `warn!`, `error!`)

### Tauri Integration

Commands are distributed across modules in `backend/crates/qbit/src/`:

| Location | Purpose |
|----------|---------|
| `commands/*.rs` | PTY, shell, themes, files, skills |
| `ai/commands/*.rs` | AI agent commands |
| `settings/commands.rs` | Settings commands |
| `sidecar/commands.rs` | Sidecar commands |
| `indexer/commands.rs` | Code indexer commands |

All commands are registered in `lib.rs`. Frontend listens via `@tauri-apps/api/event`.

## Commit Format

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(ai): add context window management
fix(terminal): prevent resize flicker
docs: clarify approval system behavior
refactor(session): simplify storage layer
test(e2e): add slash command tests
chore: update dependencies
```

### Scopes

Common scopes:
- `ai` - AI/agent functionality
- `terminal` - Terminal/PTY features
- `ui` - Frontend components
- `session` - Session management
- `tools` - Tool system
- `config` - Configuration/settings

## Pull Request Guidelines

1. Run `just check` before submitting
2. Ensure all tests pass (`just test`)
3. Include tests for new functionality
4. Update documentation if needed
5. Keep PRs focused and reasonably sized

CI runs checks, tests, and AI evals on every PR.

## Crate Guidelines

When adding a new infrastructure crate:

1. Follow the Layer 2 pattern (depend only on Layer 1)
2. Add to workspace members in `backend/Cargo.toml`
3. Use `thiserror` for domain errors
4. Include unit tests
5. Document public API

## UI Component Guidelines

- The `ui/` directory contains shadcn-generated components
- Modify via `pnpm dlx shadcn@latest`, not directly
- Custom components go in their own directories under `components/`
- Use Radix primitives for accessibility

## Documentation

- Update `CLAUDE.md` for significant architectural changes
- Add inline documentation for complex logic
- Update relevant docs in `docs/` directory
