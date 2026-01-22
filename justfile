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
    @pnpm --silent test:run

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

# Run Rust tests (quiet - only shows failures)
test-rust:
    #!/usr/bin/env bash
    if output=$(cd backend && cargo test -q 2>&1); then
        echo "✓ All Rust tests passed"
    else
        echo "$output" | grep -E "(FAILED|error|thread.*panicked)" | head -30
        exit 1
    fi

# Run Rust tests with output
test-rust-verbose:
    cd backend && cargo test -- --nocapture

# ============================================
# Building
# ============================================

# Build for production
build:
    cd backend && cargo build --features cli --no-default-features --bin qbit-cli --release
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
check:
    @echo "Running checks silently..."
    @just fmt
    @just check-fe
    @just check-rust
    @just test-rust
    @echo "OK"

# Check frontend (biome + typecheck)
check-fe:
    @pnpm --silent check > /dev/null
    @pnpm --silent typecheck

# Check Rust (clippy + fmt check)
check-rust:
    @cd backend && cargo clippy -q -- -D warnings
    @cd backend && cargo fmt --check

# Fix frontend issues (biome)
fix:
    pnpm check:fix

# Format all code
fmt: fmt-fe fmt-rust

# Format frontend
fmt-fe:
    @pnpm --silent format > /dev/null

# Format Rust
fmt-rust:
    @cd backend && cargo fmt

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
    pnpm install --silent

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
# SWE-bench Evaluations
# ============================================

# Setup Python dependencies for SWE-bench
swebench-setup:
    #!/usr/bin/env bash
    set -euo pipefail

    VENV_DIR="$HOME/.qbit/swebench-venv"

    echo "Setting up SWE-bench environment..."

    # Check Docker first
    if docker info &> /dev/null; then
        echo "✓ Docker is running"
    else
        echo "✗ Docker is not running - required for SWE-bench evaluation"
        exit 1
    fi

    # Find system Python
    SYS_PYTHON=""
    if command -v python3 &> /dev/null; then
        SYS_PYTHON="python3"
    elif command -v python &> /dev/null; then
        SYS_PYTHON="python"
    else
        echo "✗ Error: Python not found."
        exit 1
    fi
    echo "✓ System Python: $($SYS_PYTHON --version 2>&1)"

    # Create venv
    if [ -d "$VENV_DIR" ]; then
        echo "✓ Venv exists at $VENV_DIR"
    else
        echo "Creating venv at $VENV_DIR..."
        $SYS_PYTHON -m venv "$VENV_DIR"
        echo "✓ Venv created"
    fi

    # Activate and install
    source "$VENV_DIR/bin/activate"

    echo "Installing swebench..."
    pip install -q --upgrade pip
    pip install swebench

    # Verify installation
    if python -c "import swebench" 2>/dev/null; then
        echo "✓ swebench installed successfully"
    else
        echo "✗ swebench installation failed"
        exit 1
    fi

    # Check for harness module
    if python -c "import swebench.harness.run_evaluation" 2>/dev/null; then
        echo "✓ swebench harness module available"
    else
        echo "⚠ swebench harness module not available (will use Docker fallback)"
    fi

    echo ""
    echo "Setup complete! Run 'just swebench' to start evaluation."

# Run SWE-bench evaluation
# Usage: just swebench [problems] [provider] [model]
# Examples:
#   just swebench                           # Run all with defaults
#   just swebench 0-9                       # Problems 0-9 with defaults
#   just swebench 0-49 vertex-claude claude-opus-4-5@20251101
swebench problems="0-49" provider="vertex-claude" model="claude-opus-4-5@20251101":
    #!/usr/bin/env bash
    set -euo pipefail

    VENV_DIR="$HOME/.qbit/swebench-venv"

    # Check Docker is running
    if ! docker info &> /dev/null; then
        echo "✗ Error: Docker is not running"
        echo "  Please start Docker and try again"
        exit 1
    fi
    echo "✓ Docker is running"

    # Find system Python (try python3 first, then python)
    SYS_PYTHON=""
    if command -v python3 &> /dev/null; then
        SYS_PYTHON="python3"
    elif command -v python &> /dev/null; then
        SYS_PYTHON="python"
    else
        echo "✗ Error: Python not found (tried python3 and python)"
        echo "  Please install Python and try again"
        exit 1
    fi

    # Create venv if it doesn't exist
    if [ ! -d "$VENV_DIR" ]; then
        echo "Creating Python venv at $VENV_DIR..."
        $SYS_PYTHON -m venv "$VENV_DIR"
    fi

    # Activate venv
    source "$VENV_DIR/bin/activate"
    echo "✓ Python venv: $($SYS_PYTHON --version 2>&1)"

    # Check/install swebench
    if python -c "import swebench" 2>/dev/null; then
        echo "✓ swebench package installed"
    else
        echo "Installing swebench..."
        pip install -q swebench
        if python -c "import swebench" 2>/dev/null; then
            echo "✓ swebench installed successfully"
        else
            echo "✗ Failed to install swebench (will use Docker fallback)"
        fi
    fi

    # Check harness availability
    if python -c "import swebench.harness.run_evaluation" 2>/dev/null; then
        echo "✓ swebench harness available"
    else
        echo "ℹ Using Docker fallback for evaluation"
    fi

    echo ""
    echo "Starting SWE-bench evaluation..."
    echo "  Problems: {{ problems }}"
    echo "  Provider: {{ provider }}"
    echo "  Model: {{ model }}"
    echo ""

    # Run the evaluation
    cd backend && cargo run --no-default-features --features evals --bin qbit-cli -- \
        --swebench \
        --problems {{ problems }} \
        --eval-provider {{ provider }} \
        --eval-model {{ model }} \
        --output ./swebench.json \
        --results-dir ./swebench \
        --json

# Run SWE-bench with verbose output
swebench-verbose problems="0-49" provider="vertex-claude" model="claude-opus-4-5@20251101":
    #!/usr/bin/env bash
    set -euo pipefail

    VENV_DIR="$HOME/.qbit/swebench-venv"

    # Check Docker is running
    if ! docker info &> /dev/null; then
        echo "✗ Error: Docker is not running"
        exit 1
    fi
    echo "✓ Docker is running"

    # Find system Python
    SYS_PYTHON=""
    if command -v python3 &> /dev/null; then
        SYS_PYTHON="python3"
    elif command -v python &> /dev/null; then
        SYS_PYTHON="python"
    else
        echo "✗ Error: Python not found"
        exit 1
    fi

    # Create/activate venv
    if [ ! -d "$VENV_DIR" ]; then
        $SYS_PYTHON -m venv "$VENV_DIR"
    fi
    source "$VENV_DIR/bin/activate"

    # Install swebench if needed
    if ! python -c "import swebench" 2>/dev/null; then
        pip install -q swebench
    fi
    echo "✓ Environment ready"

    echo ""
    echo "Starting SWE-bench evaluation (verbose)..."
    echo "  Problems: {{ problems }}"
    echo "  Provider: {{ provider }}"
    echo "  Model: {{ model }}"
    echo ""

    cd backend && RUST_LOG=debug cargo run --no-default-features --features evals --bin qbit-cli -- \
        --swebench \
        --problems {{ problems }} \
        --eval-provider {{ provider }} \
        --eval-model {{ model }} \
        --output ./swebench.json \
        --results-dir ./swebench \
        --json -v

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
