# Development

## Commands

```bash
just dev              # Full app with hot reload
just dev-fe           # Frontend only (Vite on port 1420)
just check            # All checks (biome + clippy + fmt)
just test             # All tests (frontend + Rust)
just test-e2e         # E2E tests (Playwright)
just build            # Production build
just eval             # Run evaluation scenarios
```

Run `just --list` for all available commands.

## Frontend-only development

```bash
just dev-fe
```

This starts Vite with a mock Tauri environment (useful for rapid UI iteration without LLM costs).

## Adding a new tool

1. Define schema in `backend/crates/qbit-ai/src/tool_definitions.rs`
2. Implement executor in `backend/crates/qbit-ai/src/tool_executors.rs`
3. Register in the tool registry
4. Add event handler in `frontend/hooks/useAiEvents.ts`

See also:
- [Browser-only frontend development](browser-dev.md)
- [Architecture](architecture.md)
