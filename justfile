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
    cd backend && cargo test --features local-tools

# Run Rust tests with output
test-rust-verbose:
    cd backend && cargo test --features local-tools -- --nocapture

# ============================================
# Building
# ============================================

# Build for production
build:
    cd backend && cargo build --features cli,local-tools --no-default-features --bin qbit-cli --release
    pnpm tauri build

# Build frontend only
build-fe:
    pnpm build

# Build Rust backend only (debug)
build-rust:
    cd backend && cargo build

# Build Rust backend (release)
build-rust-release:
    cd backend && cargo build --release

# ============================================
# Code Quality
# ============================================

# Run all checks (format, lint, typecheck, rust tests)
check: fmt check-fe check-rust test-rust

# Check frontend (biome + typecheck)
check-fe:
    pnpm check
    pnpm typecheck

# Check Rust (clippy + fmt check)
# Use --all-targets to compile test targets too, sharing artifacts with cargo test
check-rust:
    cd backend && cargo clippy --all-targets --features local-tools -- -D warnings
    cd backend && cargo fmt --check

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
    cd backend && cargo fmt

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
    cd backend && cargo clean

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
    cd backend && cargo update

# ============================================
# CLI & Evaluations
# ============================================

# Build CLI binary
build-cli:
    cd backend && cargo build --no-default-features --features cli --bin qbit-cli

# Run all Rust eval scenarios
eval *args:
    cd backend && cargo run --no-default-features --features evals,cli --bin qbit-cli -- --eval {{args}}

# List available eval scenarios
eval-list:
    cd backend && cargo run --no-default-features --features evals,cli --bin qbit-cli -- --list-scenarios

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
    cd backend && cargo tree

# Open Rust docs
docs:
    cd backend && cargo doc --open

# Run a quick sanity check before committing
precommit: check test
    @echo "✓ All checks passed!"

# Run full CI suite (check + e2e + evals)
ci: check test-e2e eval build
    @echo "✓ Full CI suite passed!"
