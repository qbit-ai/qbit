# Qbit - Tauri Terminal App
# Run `just` to see all available commands

# Default recipe - show help
default:
    @just --list

# ============================================
# Development
# ============================================

# Start development server (frontend + backend)
# Usage: just dev [path]
# Example: just dev ~/Code/my-project
dev path="":
    {{ if path == "" { "pnpm tauri dev" } else { "pnpm tauri dev -- " + path } }}

# Start only the frontend dev server
dev-fe:
    pnpm dev

# ============================================
# Testing
# ============================================

# Run all tests (frontend + backend)
test: test-fe test-rust

# Run frontend tests
test-fe:
    pnpm test:run

# Run frontend tests in watch mode
test-watch:
    pnpm test

# Run frontend tests with UI
test-ui:
    pnpm test:ui

# Run frontend tests with coverage
test-coverage:
    pnpm test:coverage

# Run e2e tests (Playwright)
test-e2e *args:
    pnpm exec playwright test {{args}}

# Run Rust tests
# Note: compat_layer tests use --features local-tools to avoid vtcode-core's HITL prompts
test-rust:
    cd src-tauri && cargo test --features local-tools

# Run Rust tests with output
test-rust-verbose:
    cd src-tauri && cargo test --features local-tools -- --nocapture

# ============================================
# Building
# ============================================

# Build for production
build:
    cd src-tauri && cargo build --features cli,local-tools --no-default-features --bin qbit-cli --release
    pnpm tauri build

# Build frontend only
build-fe:
    pnpm build

# Build Rust backend only (debug)
build-rust:
    cd src-tauri && cargo build

# Build Rust backend (release)
build-rust-release:
    cd src-tauri && cargo build --release

# ============================================
# Code Quality
# ============================================

# Run all checks (format, lint, typecheck)
check: fmt check-fe check-rust

# Check frontend (biome + typecheck)
check-fe:
    pnpm check
    pnpm typecheck

# Check Rust (clippy + fmt check)
check-rust:
    cd src-tauri && cargo clippy -- -D warnings
    cd src-tauri && cargo fmt --check

# Fix frontend issues (biome)
fix:
    pnpm check:fix

# Format all code
fmt: fmt-fe fmt-rust

# Format frontend
fmt-fe:
    pnpm format

# Format Rust
fmt-rust:
    cd src-tauri && cargo fmt

# Lint frontend
lint:
    pnpm lint

# Lint and fix frontend
lint-fix:
    pnpm lint:fix

# ============================================
# Cleaning
# ============================================

# Clean all build artifacts
clean: clean-fe clean-rust

# Clean frontend
clean-fe:
    rm -rf dist node_modules/.vite

# Clean Rust
clean-rust:
    cd src-tauri && cargo clean

# Deep clean (includes node_modules)
clean-all: clean
    rm -rf node_modules

# ============================================
# Dependencies
# ============================================

# Install all dependencies
install:
    pnpm install

# Update frontend dependencies
update-fe:
    pnpm update

# Update Rust dependencies
update-rust:
    cd src-tauri && cargo update

# ============================================
# CLI & Server
# ============================================

# Build CLI binary (without server feature)
build-cli:
    cd src-tauri && cargo build --no-default-features --features cli --bin qbit-cli

# Build server binary (with HTTP/SSE support)
build-server:
    cd src-tauri && cargo build --no-default-features --features server --bin qbit-cli

# Run the eval server on default port (8080)
server port="8080":
    @just build-server
    ./src-tauri/target/debug/qbit-cli --server --port {{port}}

# Run the eval server on a random available port
server-random:
    @just build-server
    ./src-tauri/target/debug/qbit-cli --server --port 0

# ============================================
# Evaluations
# ============================================

# Run all evals (builds server, starts it, runs tests, stops server)
# QBIT_WORKSPACE is set to qbit-go-testbed for file operation tests
eval *args:
    @just build-server
    @echo "Starting server..."
    @./src-tauri/target/debug/qbit-cli --server --port 8080 &
    @sleep 2
    -cd evals && QBIT_WORKSPACE="../../qbit-go-testbed" QBIT_EVAL_MODEL="claude-haiku-4-5@20251001" RUN_API_TESTS=1 uv run pytest {{args}} -v
    @pkill -f "qbit-cli --server" 2>/dev/null || true

# Run evals without LLM calls (fast, no API key needed)
eval-fast *args:
    @just build-server
    @echo "Starting server..."
    @./src-tauri/target/debug/qbit-cli --server --port 8080 &
    @sleep 2
    -cd evals && QBIT_WORKSPACE="../../qbit-go-testbed" uv run pytest -k "not requires_api" {{args}} -v
    @pkill -f "qbit-cli --server" 2>/dev/null || true

# ============================================
# Utilities
# ============================================

# Kill any running dev processes (including server)
kill:
    -pkill -f "target/debug/qbit" 2>/dev/null
    -pkill -f "qbit-cli" 2>/dev/null
    -pkill -f "vite" 2>/dev/null
    -lsof -ti:1420 | xargs kill -9 2>/dev/null

# Restart dev (kill + dev)
restart: kill dev

# Show Rust dependency tree
deps:
    cd src-tauri && cargo tree

# Open Rust docs
docs:
    cd src-tauri && cargo doc --open

# Run a quick sanity check before committing
precommit: check test
    @echo "✓ All checks passed!"
