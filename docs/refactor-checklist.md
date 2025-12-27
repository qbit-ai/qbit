# Crate Refactoring Quick Reference Checklist

**Companion to**: [tdd-refactoring-strategy.md](./tdd-refactoring-strategy.md) and [crate-refactoring-plan.md](./crate-refactoring-plan.md)

---

## Setup (Do Once)

```bash
# Install test tools
cargo install cargo-nextest cargo-tarpaulin cargo-public-api cargo-watch

# Add dev dependencies
cd backend
cargo add --dev insta proptest criterion tempfile serial_test

# Create test directories
mkdir -p tests/{characterization,integration,contracts,golden,extraction,snapshots}
mkdir -p benches

# Run baseline benchmarks
cargo bench -- --save-baseline pre-refactor

# Generate initial coverage
cargo tarpaulin --workspace --out Html
```

---

## Pre-Migration (Phase 0)

### Characterization Tests

- [ ] Tool execution behavior (vtcode-core vs local)
- [ ] Event serialization (all 30+ AiEvent variants)
- [ ] PTY parser (OSC sequences)
- [ ] Settings loading (env interpolation)
- [ ] Session archive format

```bash
# Create characterization tests
cat > backend/tests/characterization/baseline.rs

# Run and save snapshots
cargo test --test characterization
cargo insta accept
```

### Golden Master Tests

- [ ] Session archive JSON format
- [ ] Tool response formats (success/error)
- [ ] Event stream format

```bash
# Capture golden outputs
cargo test --test golden
cargo insta review
```

### Integration Tests

- [ ] AI → Tools
- [ ] Tools → PTY
- [ ] PTY → Runtime
- [ ] Settings → All modules

```bash
cargo test --test integration
```

### Benchmarks

- [ ] Tool execution (read/write/edit)
- [ ] Event serialization
- [ ] PTY parsing
- [ ] Session archive I/O

```bash
cargo bench -- --save-baseline pre-refactor
```

### Baseline Metrics

```bash
# Coverage
cargo tarpaulin --workspace --out Stdout | grep "Coverage"
# Target: ≥60%

# Build time
cargo clean
cargo build --workspace --timings
# Note: incremental build time

# Test time
cargo nextest run --workspace
# Note: total execution time
```

---

## Per-Crate Extraction Template

**For each crate** (qbit-core, qbit-settings, qbit-runtime, etc.):

### 1. Pre-Extraction (Day 1)

#### Write Interface Tests First

```rust
// backend/tests/extraction/<crate_name>.rs

#[test]
fn test_<crate>_public_api() {
    use <crate_name>::MainType;

    // Define the API you want
    let instance = MainType::new();
    instance.method();
}
```

- [ ] Define public API in tests
- [ ] Write contract tests (interactions with other crates)
- [ ] Document expected dependencies

#### Run Tests (Should Fail)

```bash
cargo test test_<crate>_public_api
# Expected: Error - crate doesn't exist yet
```

### 2. Extraction (Day 2)

#### Create Crate Structure

```bash
# Copy template
mkdir -p backend/crates/<crate-name>/src
cd backend/crates/<crate-name>

# Create Cargo.toml
cat > Cargo.toml <<EOF
[package]
name = "<crate-name>"
version.workspace = true
edition.workspace = true

[dependencies]
# Add minimal dependencies
EOF

# Create lib.rs
cat > src/lib.rs <<EOF
pub mod module;

pub use module::PublicType;
EOF
```

- [ ] Create directory structure
- [ ] Write `Cargo.toml`
- [ ] Write `lib.rs` with public exports
- [ ] Move source files from main crate
- [ ] Add to workspace members in root `Cargo.toml`

#### Update Imports

```bash
# Find all imports to update
rg "crate::<old_path>::" --files-with-matches

# Update imports
rg "crate::<old_path>::" --files-with-matches | \
  xargs sed -i 's/crate::<old_path>::/<crate_name>::/g'
```

- [ ] Update all imports in main crate
- [ ] Update re-exports if needed
- [ ] Update feature flags

### 3. Testing (Day 2-3)

#### Compilation Tests

```bash
# Test isolated compilation
cd backend/crates/<crate-name>
cargo clean
cargo build

# Test with all feature combinations
cargo build --no-default-features
cargo build --all-features
```

- [ ] Builds in isolation
- [ ] Builds with no features
- [ ] Builds with all features
- [ ] No circular dependencies (`cargo tree`)

#### Isolated Tests

```bash
# Run crate's own tests
cargo test

# With features
cargo test --no-default-features
cargo test --features <feature>
```

- [ ] All crate tests pass
- [ ] Tests pass with all feature combinations
- [ ] No warnings (`cargo build`)

#### Integration Tests

```bash
# Test in workspace context
cd ../../../../
cargo test --workspace
```

- [ ] All workspace tests still pass
- [ ] No regressions
- [ ] Integration tests work

#### Coverage

```bash
cargo tarpaulin -p <crate-name> --out Stdout
```

- [ ] Coverage ≥70%
- [ ] All public APIs tested
- [ ] Edge cases covered

#### API Snapshot

```bash
cargo public-api -p <crate-name> > tests/snapshots/<crate>-api.txt
```

- [ ] API snapshot created
- [ ] Reviewed for correctness
- [ ] No unintended exports

### 4. Validation (Day 3)

#### Run Full Validation Script

```bash
./scripts/validate-refactor-step.sh
```

Checklist (automated):
- [ ] Workspace builds (`cargo build --workspace`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Formatting correct (`cargo fmt --check`)
- [ ] Zero clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Feature matrix passes
- [ ] Integration tests pass
- [ ] Benchmarks within 5% of baseline
- [ ] Coverage ≥70%

#### Manual Checks

- [ ] Documentation updated
- [ ] Examples work
- [ ] README updated (if applicable)
- [ ] CHANGELOG entry added

### 5. Commit & Tag

```bash
# Commit
git add .
git commit -m "feat(<crate>): extract <crate-name> crate

- Moved <module> to standalone crate
- Coverage: X%
- Zero breaking changes
- All tests passing"

# Tag
git tag refactor-step-<N>
git push origin refactor-step-<N>
```

- [ ] Commit message follows convention
- [ ] Tag created
- [ ] Pushed to remote

---

## Validation Script

**File**: `scripts/validate-refactor-step.sh`

```bash
#!/bin/bash
set -e

echo "=== Validation Suite ==="

# 1. Build
echo "→ Building workspace..."
cargo build --workspace

# 2. Tests
echo "→ Running tests..."
cargo nextest run --workspace --features local-tools

# 3. Format
echo "→ Checking format..."
cargo fmt --check

# 4. Clippy
echo "→ Running clippy..."
cargo clippy --workspace -- -D warnings

# 5. Feature matrix
echo "→ Testing feature combinations..."
cargo build -p qbit --no-default-features --features cli
cargo build -p qbit --no-default-features --features tauri
cargo build -p qbit --features cli,local-tools

# 6. Integration
echo "→ Integration tests..."
cargo test --test compat_layer --features local-tools

# 7. Coverage
echo "→ Coverage check..."
cargo tarpaulin --workspace --out Stdout --fail-under 70

# 8. Benchmarks
echo "→ Performance check..."
cargo bench -- --baseline pre-refactor

echo "✓ Validation complete"
```

Make executable:
```bash
chmod +x scripts/validate-refactor-step.sh
```

---

## Rollback Criteria

### Hard Stop (Revert Immediately)

❌ Stop and rollback if:

1. **Test coverage drops below 60%**
   ```bash
   cargo tarpaulin --workspace --fail-under 60 || echo "ROLLBACK"
   ```

2. **Any test fails**
   ```bash
   cargo test --workspace || echo "ROLLBACK"
   ```

3. **Performance regression >20%**
   ```bash
   cargo bench -- --baseline pre-refactor
   # Check output for regressions
   ```

4. **Clippy errors**
   ```bash
   cargo clippy --workspace -- -D warnings || echo "ROLLBACK"
   ```

5. **Circular dependency**
   ```bash
   cargo tree -p qbit-core --edges normal | grep "qbit-core" | wc -l
   # Should be 1 (only root)
   ```

### Warning Signs (Investigate)

⚠️ Investigate if:

- Clippy warnings increase by >5
- Build time increases by >30%
- Snapshot test fails
- Test flakiness >1%

### Rollback Procedure

```bash
# 1. Create backup
git branch backup-$(date +%Y%m%d)

# 2. Revert
git reset --hard <last-good-commit>

# 3. Verify
cargo test --workspace

# 4. Document
cat > docs/rollback-$(date +%Y%m%d).md <<EOF
# Rollback $(date)

## Reason
[Why?]

## Failed Tests
[Which?]

## Next Steps
[What to try differently?]
EOF
```

---

## Phase 1: Foundation Crates

### qbit-core

- [ ] Extract events (AiEvent, RuntimeEvent, SidecarEvent)
- [ ] Extract runtime trait (QbitRuntime)
- [ ] Extract session types (SessionArchive, SessionMessage)
- [ ] Extract error types
- [ ] Verify: Zero internal dependencies
- [ ] Coverage: ≥75%
- [ ] Estimated: 2-3 days

### qbit-settings

- [ ] Extract schema (QbitSettings struct)
- [ ] Extract loader (SettingsManager)
- [ ] Feature-gate commands (`#[cfg(feature = "tauri")]`)
- [ ] Include template.toml
- [ ] Verify: Works in isolation
- [ ] Coverage: ≥75%
- [ ] Estimated: 1-2 days

### qbit-runtime

- [ ] Extract TauriRuntime (feature: tauri)
- [ ] Extract CliRuntime (feature: cli)
- [ ] Verify: Mutual exclusion (tauri XOR cli)
- [ ] Contract tests with qbit-core
- [ ] Coverage: ≥70%
- [ ] Estimated: 1 day

### qbit-tools

- [ ] Extract ToolRegistry
- [ ] Extract tool definitions
- [ ] Extract tool executors
- [ ] Extract udiff module
- [ ] Extract planner
- [ ] Verify: API matches vtcode-core exactly
- [ ] Contract tests with qbit-core
- [ ] Property tests for diff parsing
- [ ] Coverage: ≥75%
- [ ] Estimated: 3-4 days

---

## Phase 2: Domain Crates

### qbit-pty

- [ ] Extract PtyManager
- [ ] Extract TerminalParser
- [ ] Extract shell detection
- [ ] Property tests for parser (fuzz testing)
- [ ] Verify: Works with both runtimes
- [ ] Coverage: ≥75%
- [ ] Estimated: 3-4 days

### qbit-indexer

- [ ] Extract IndexerState
- [ ] Extract path resolution
- [ ] Feature-gate commands
- [ ] Verify: Works with qbit-settings
- [ ] Coverage: ≥70%
- [ ] Estimated: 2-3 days

---

## Phase 3: Advanced Crates

### qbit-sidecar-core

- [ ] Extract session file I/O
- [ ] Extract event definitions
- [ ] Extract synthesis (LLM client)
- [ ] Verify: Minimal dependencies
- [ ] Coverage: ≥70%
- [ ] Estimated: 3-5 days

### qbit-shell-integration

- [ ] Extract installer
- [ ] Extract shell scripts (embedded)
- [ ] Extract types
- [ ] Verify: Zero dependencies (std only)
- [ ] Coverage: ≥80% (already has 900+ lines of tests)
- [ ] Estimated: 2-3 days

### qbit-context-manager

- [ ] Extract ContextManager
- [ ] Extract pruner
- [ ] Extract budget tracking
- [ ] Extract truncation
- [ ] Property tests for pruning logic
- [ ] Coverage: ≥75%
- [ ] Estimated: 2-3 days

---

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/refactor-validation.yml

name: Refactor Validation

on:
  push:
    branches: [feat/workspace-refactor]
  pull_request:

jobs:
  test-matrix:
    strategy:
      matrix:
        features: ["", "cli", "tauri", "cli,local-tools"]
    steps:
      - run: cargo test --workspace --features ${{ matrix.features }}

  coverage:
    steps:
      - run: cargo tarpaulin --workspace --fail-under 70

  benchmarks:
    steps:
      - run: cargo bench -- --baseline main
```

### Pre-commit Hook

```bash
# .git/hooks/pre-commit
#!/bin/bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --lib --workspace
```

Make executable:
```bash
chmod +x .git/hooks/pre-commit
```

---

## Daily Workflow

### Morning (Start of Work)

```bash
# 1. Pull latest
git pull origin feat/workspace-refactor

# 2. Run tests
cargo test --workspace

# 3. Check status
git status
```

### During Work

```bash
# Watch mode (auto-run tests)
cargo watch -x "nextest run -p <crate-name>"

# Or use just
just test-watch
```

### End of Day

```bash
# 1. Run full validation
./scripts/validate-refactor-step.sh

# 2. Commit progress
git add .
git commit -m "wip: <crate-name> extraction"

# 3. Push
git push origin feat/workspace-refactor
```

---

## Metrics Dashboard

Track these metrics for each crate:

| Crate | Coverage | Tests | Warnings | Deps | Status |
|-------|----------|-------|----------|------|--------|
| qbit-core | X% | ✓/✗ | N | M | 🟢/🟡/🔴 |
| qbit-settings | X% | ✓/✗ | N | M | 🟢/🟡/🔴 |
| ... | ... | ... | ... | ... | ... |

Update after each extraction:

```bash
# Generate metrics
cat > metrics-$(date +%Y%m%d).md <<EOF
# Metrics - $(date)

| Crate | Coverage | Tests | Warnings | Deps |
|-------|----------|-------|----------|------|
$(for crate in qbit-core qbit-settings qbit-runtime; do
  coverage=$(cargo tarpaulin -p $crate --out Stdout | grep -oP '\d+\.\d+(?=%)')
  tests=$(cargo test -p $crate 2>&1 | grep -oP '\d+(?= passed)')
  warnings=$(cargo build -p $crate 2>&1 | grep -c "warning")
  deps=$(cargo tree -p $crate --depth 1 | wc -l)
  echo "| $crate | ${coverage}% | ${tests} ✓ | $warnings | $deps |"
done)
EOF
```

---

## Success Criteria

### Overall Project

- [ ] All 9 crates extracted
- [ ] Workspace compiles
- [ ] All tests passing (100%)
- [ ] Coverage ≥75%
- [ ] Zero clippy warnings
- [ ] Build time improved by 40-50%
- [ ] Documentation complete
- [ ] CI/CD green

### Per-Crate

- [ ] Tests written before extraction
- [ ] Compiles in isolation
- [ ] Coverage ≥70%
- [ ] API snapshot captured
- [ ] Feature flags work
- [ ] Integration tests pass
- [ ] Benchmarks within 5%
- [ ] Documentation complete

---

## Quick Commands

```bash
# Full validation
./scripts/validate-refactor-step.sh

# Watch mode
cargo watch -x "nextest run"

# Coverage
cargo tarpaulin --workspace --out Html

# Benchmarks
cargo bench -- --baseline pre-refactor

# API snapshot
cargo public-api -p <crate> > tests/snapshots/<crate>-api.txt

# Dependency tree
cargo tree -p <crate> --depth 1

# Feature test
cargo test -p <crate> --no-default-features
cargo test -p <crate> --all-features

# Clean rebuild
cargo clean && cargo build --workspace --timings
```

---

**Last Updated**: 2025-12-26
**Status**: Ready to Use
