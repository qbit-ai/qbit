# Repository Guidelines

## Project Structure & Module Organization

- `frontend/`: React 19 + TypeScript (Vite, Tailwind v4). Key areas: `components/`, `components/ui/`, `hooks/` (includes fullterm mode auto-switching), `lib/` typed Tauri wrappers, `store/` (RenderMode: timeline/fullterm), `pages/`, `styles/`, `test/`.
- `frontend/components/ui/`: shadcn/ui primitives; regenerate via `pnpm dlx shadcn@latest`, do not hand-edit.
- `backend/src/`: Rust backend for Tauri 2. Major modules: `ai/`, `pty/`, `sidecar/`, `settings/`, `indexer/`, `tools/`, `tavily/`, `web_fetch.rs`, `commands/`, `cli/`, `bin/`, `session/`, `runtime/`.
- `backend/crates/`: local crates (e.g. `rig-anthropic-vertex/`).
- `e2e/`: Playwright end-to-end tests.
- `docs/` and `public/`: documentation and static assets.
- `dist/`: build output; do not commit manual edits.

## Build, Test, and Development Commands

Prefer `just` (run `just --list` for all recipes):

Development
- `just dev [path]`: full app (optional workspace path override).
- `just dev-fe`: frontend only (Vite).

Testing
- `just test`: all tests (frontend + Rust).
- `just test-fe`: Vitest single run.
- `just test-watch`: Vitest watch.
- `just test-ui`: Vitest UI.
- `just test-coverage`: coverage.
- `just test-rust`: Rust tests (with `local-tools` feature).
- `just test-e2e [args]`: Playwright.

Quality
- `just check`: format + lint + typecheck (frontend + Rust).
- `just check-fe`, `just check-rust`.
- `just fix`: Biome auto-fix.
- `just fmt`: format all.
- `just lint`, `just lint-fix`.

Build / CLI
- `just build`: production build (Tauri + CLI).
- `just build-fe`, `just build-rust`, `just build-rust-release`.
- `just build-cli`: CLI binary only.

Evals
- `just eval`: Rust-native evals using rig (see `docs/rig-evals.md`).

Other useful commands: `pnpm install`, `pnpm tauri dev`, `pnpm preview`, `pnpm exec playwright test`, `just precommit`.

## Coding Style & Naming Conventions

- Frontend uses Biome (`biome.json`): 2-space indent, 100-column lines, double quotes, semicolons, trailing commas (ES5).
- TypeScript/React: functional components; hooks live in `frontend/hooks/`; global state in `frontend/store/`.
- Tauri calls: use typed wrappers in `frontend/lib/`; avoid raw `invoke()`.
- Rust: follow rustfmt defaults; keep clippy clean (`cargo clippy -D warnings`).
- Naming: `PascalCase` for React components, `kebab-case` for folders, `snake_case.rs` for Rust files.

## Testing Guidelines

- Frontend: Vitest + React Testing Library + jsdom; name tests `*.test.ts(x)` near the code they cover; setup in `frontend/test/`.
- Rust: `cargo test` (uses `--features local-tools` in `just test-rust`).
- Evals: Rust-native framework using rig; run via `just eval` (see `docs/rig-evals.md`).
- E2E: Playwright specs `e2e/*.spec.ts`.

## Commit & Pull Request Guidelines

- Use Conventional Commits style messages: `feat(scope): ...`, `fix: ...`, `refactor: ...`, `chore: ...`.
- PRs should describe intent, link issues, note tests run (`just precommit` is a good baseline), and include screenshots for UI changes.

## Terminal Render Modes

The terminal supports two render modes:
- `timeline` (default): Parsed command blocks in unified timeline view
- `fullterm`: Full xterm.js terminal for interactive apps (vim, htop, ssh, etc.)

Auto-switching is handled in `useTauriEvents.ts` via the `FULLTERM_COMMANDS` set. The status bar shows a "Full Term" indicator when active.

## Configuration Notes

- Root `.env` for AI providers (Vertex AI required; optional TAVILY). Never commit secrets.
- Settings file: `~/.qbit/settings.toml` (auto-generated on first run).
- Sessions stored in `~/.qbit/sessions/` (override with `VT_SESSION_DIR`).
- Workspace override: `just dev /path/to/workspace` or set `QBIT_WORKSPACE` env var.
