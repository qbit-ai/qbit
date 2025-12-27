# TDD Strategy Summary

**Quick reference** for the crate refactoring test-driven development approach.

---

## 📚 Documentation Index

1. **[tdd-refactoring-strategy.md](./tdd-refactoring-strategy.md)** - Complete TDD strategy
   - Pre-migration test suite
   - Per-crate extraction workflow
   - Continuous validation
   - Safety mechanisms
   - Rollback criteria

2. **[refactor-checklist.md](./refactor-checklist.md)** - Practical checklist
   - Daily workflow
   - Per-crate template
   - Quick commands
   - Validation scripts

3. **[tdd-example-qbit-core.md](./tdd-example-qbit-core.md)** - Complete example
   - Day-by-day walkthrough
   - Actual code examples
   - Troubleshooting guide
   - Metrics and results

4. **[crate-refactoring-plan.md](./crate-refactoring-plan.md)** - Overall plan
   - Architecture overview
   - Crate descriptions
   - Migration timeline

---

## 🎯 Core Principles

1. **Write tests BEFORE extracting code**
   - Define interfaces in tests
   - Capture current behavior
   - Set expectations

2. **Never break existing tests**
   - All green stays green
   - Characterization tests prevent regressions
   - Snapshot tests catch accidental changes

3. **Test at multiple levels**
   - Unit tests (per-crate)
   - Integration tests (cross-crate)
   - Contract tests (boundaries)
   - Property tests (invariants)
   - Golden master tests (exact outputs)

4. **Automate validation**
   - CI runs all tests on every commit
   - Pre-commit hooks prevent bad commits
   - Validation script checks everything

5. **Clear rollback criteria**
   - Coverage <60% → STOP
   - Any test fails → ROLLBACK
   - Performance >20% slower → INVESTIGATE
   - Clippy errors → STOP

---

## 🚀 Quick Start

### Setup (One-Time)

```bash
# Install tools
cargo install cargo-nextest cargo-tarpaulin cargo-public-api cargo-watch

# Add dev dependencies
cd backend
cargo add --dev insta proptest criterion tempfile serial_test

# Create test directories
mkdir -p tests/{characterization,integration,contracts,golden,extraction,snapshots}
mkdir -p benches
mkdir -p scripts

# Create validation script
cat > scripts/validate-refactor-step.sh <<'EOF'
#!/bin/bash
set -e
cargo build --workspace
cargo test --workspace --features local-tools
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo tarpaulin --workspace --fail-under 70
EOF

chmod +x scripts/validate-refactor-step.sh

# Run baseline benchmarks
cargo bench -- --save-baseline pre-refactor
```

### Phase 0: Pre-Migration Tests

```bash
# Write characterization tests
# - Tool execution
# - Event serialization
# - PTY parsing
# - Settings loading

# Run and capture baselines
cargo test
cargo insta accept

# Save coverage baseline
cargo tarpaulin --workspace --out Html
```

### Per-Crate Workflow

For each crate extraction:

**Day 1: Write Tests**
```bash
# 1. Interface tests (define API)
# 2. Characterization tests (capture behavior)
# 3. Contract tests (define boundaries)
# 4. Property tests (invariants)
```

**Day 2: Extract**
```bash
# 1. Create crate structure
mkdir -p backend/crates/<name>/src

# 2. Move code
# 3. Update imports
# 4. Add to workspace
```

**Day 3: Validate**
```bash
# 1. Test compilation
cargo build -p <name>

# 2. Test in isolation
cargo test -p <name>

# 3. Test workspace
cargo test --workspace

# 4. Run validation
./scripts/validate-refactor-step.sh

# 5. Commit
git commit -m "feat(<name>): extract <name> crate"
git tag refactor-step-<N>
```

---

## 📊 Test Coverage Targets

| Phase | Unit | Integration | Contract | Property |
|-------|------|-------------|----------|----------|
| Pre-Migration | ≥60% | ✓ Core paths | N/A | ✓ Parsers |
| During Migration | ≥70% | ✓ All APIs | ✓ All boundaries | ✓ Pure fns |
| Post-Migration | ≥75% | ✓ End-to-end | ✓ Cross-crate | ✓ Invariants |

---

## 🛠️ Tools & Commands

### Testing

```bash
# Fast test runner
cargo nextest run --workspace

# Watch mode
cargo watch -x "nextest run"

# Coverage
cargo tarpaulin --workspace --out Html

# Property tests (extended)
PROPTEST_CASES=10000 cargo test
```

### Validation

```bash
# Full validation
./scripts/validate-refactor-step.sh

# API stability
cargo public-api -p <crate> > tests/snapshots/<crate>-api.txt
cargo public-api diff <crate> main

# Benchmarks
cargo bench --bench <name> -- --baseline pre-refactor

# Dependency check
cargo tree -p <crate> --depth 1
```

### Snapshots

```bash
# Review snapshot changes
cargo insta review

# Accept all snapshots
cargo insta accept

# Reject and fix
cargo insta reject
```

---

## ⚠️ Rollback Criteria

### Hard Stop (Revert Immediately)

1. ❌ Test coverage <60%
2. ❌ Any test failure
3. ❌ Performance regression >20%
4. ❌ Clippy errors
5. ❌ Circular dependency

### Rollback Procedure

```bash
# 1. Backup
git branch backup-$(date +%Y%m%d)

# 2. Revert
git reset --hard <last-good-commit>

# 3. Verify
cargo test --workspace

# 4. Document
echo "Rollback: [reason]" > docs/rollback-$(date +%Y%m%d).md
```

---

## 📝 Test Types Explained

### 1. Characterization Tests

**Purpose**: Capture current behavior before changes

**When**: Before any refactoring

**Example**:
```rust
#[test]
fn test_current_tool_behavior() {
    // Capture exact current behavior
    let result = execute_tool("read_file", args);
    insta::assert_json_snapshot!(result);
}
```

### 2. Interface Tests

**Purpose**: Define the API you want

**When**: Before creating the crate

**Example**:
```rust
#[test]
fn test_desired_interface() {
    use qbit_settings::SettingsManager;

    let mgr = SettingsManager::new().await.unwrap();
    let settings = mgr.get().await;
    // This will fail until crate exists
}
```

### 3. Contract Tests

**Purpose**: Define boundaries between crates

**When**: Before extraction, verify after

**Example**:
```rust
#[test]
fn test_runtime_trait_contract() {
    // Both TauriRuntime and CliRuntime must implement this
    fn _assert<T: QbitRuntime>() {}
}
```

### 4. Property Tests

**Purpose**: Verify invariants hold for all inputs

**When**: For pure functions and parsers

**Example**:
```rust
proptest! {
    #[test]
    fn parser_never_panics(data in any::<Vec<u8>>()) {
        let mut parser = TerminalParser::new();
        let _ = parser.parse(&data); // Must not panic
    }
}
```

### 5. Golden Master Tests

**Purpose**: Exact output comparison

**When**: For serialization formats

**Example**:
```rust
#[test]
fn test_session_archive_format() {
    let session = create_session();
    let json = serde_json::to_string(&session).unwrap();
    insta::assert_snapshot!(json);
}
```

### 6. Integration Tests

**Purpose**: Test cross-module interactions

**When**: Before and after extraction

**Example**:
```rust
#[tokio::test]
async fn test_agent_uses_tools() {
    let agent = create_agent();
    let result = agent.execute("read test.txt").await;
    assert!(result.is_ok());
}
```

---

## 🎓 Best Practices

### Do's ✅

- ✅ Write tests before extracting code
- ✅ Run validation after every commit
- ✅ Capture baselines with snapshots
- ✅ Test all feature combinations
- ✅ Document rollbacks
- ✅ Use property tests for parsers
- ✅ Keep crates focused and small

### Don'ts ❌

- ❌ Skip tests to "save time"
- ❌ Continue with failing tests
- ❌ Forget to update imports
- ❌ Add internal dependencies to foundation crates
- ❌ Ignore coverage drops
- ❌ Skip benchmark comparisons
- ❌ Commit without running validation

---

## 🔄 Daily Workflow

### Morning

```bash
git pull origin feat/workspace-refactor
cargo test --workspace
git status
```

### During Work

```bash
# Watch mode
cargo watch -x "nextest run -p <crate-name>"
```

### End of Day

```bash
./scripts/validate-refactor-step.sh
git add .
git commit -m "wip: <crate> extraction"
git push
```

---

## 📈 Success Metrics

### Per-Crate

- [ ] Tests written before extraction
- [ ] Compiles in isolation
- [ ] Coverage ≥70%
- [ ] API snapshot captured
- [ ] Feature flags work
- [ ] Integration tests pass
- [ ] Benchmarks within 5%
- [ ] Documentation complete

### Overall Project

- [ ] All 9 crates extracted
- [ ] Workspace compiles
- [ ] All tests passing (100%)
- [ ] Coverage ≥75%
- [ ] Zero clippy warnings
- [ ] Build time improved 40-50%
- [ ] Documentation complete
- [ ] CI/CD green

---

## 🎯 Phase 1 Crates (Recommended Order)

1. **qbit-core** (2-3 days)
   - Events, runtime trait, session types
   - Zero internal dependencies
   - Foundation for everything

2. **qbit-settings** (1-2 days)
   - TOML config management
   - Already well-isolated

3. **qbit-runtime** (1 day)
   - TauriRuntime, CliRuntime
   - Implements qbit-core::QbitRuntime

4. **qbit-tools** (3-4 days)
   - Tool execution system
   - Must match vtcode-core API exactly

---

## 🚨 Common Pitfalls

### Pitfall 1: Circular Dependencies

**Problem**: qbit-core imports from qbit

**Solution**: qbit-core should NEVER depend on any internal crate

**Check**:
```bash
cargo tree -p qbit-core --depth 1 | grep qbit-
# Should show nothing except qbit-core itself
```

### Pitfall 2: Missing Feature Flags

**Problem**: Tauri commands compile in CLI mode

**Solution**: Use `#[cfg(feature = "tauri")]`

**Test**:
```bash
cargo build -p <crate> --no-default-features
cargo build -p <crate> --features tauri
```

### Pitfall 3: Lost Tests

**Problem**: Moved code but forgot tests

**Solution**: Always check coverage before/after

**Check**:
```bash
cargo tarpaulin -p <crate> --out Stdout
# Coverage should not drop
```

### Pitfall 4: Snapshot Drift

**Problem**: Output changes but tests still pass

**Solution**: Use insta for snapshots, review changes

**Check**:
```bash
cargo insta review
# Review all changes carefully
```

---

## 📚 Further Reading

- **Rust Testing**: https://doc.rust-lang.org/book/ch11-00-testing.html
- **Proptest**: https://altsysrq.github.io/proptest-book/
- **Insta**: https://insta.rs/
- **Cargo Workspaces**: https://doc.rust-lang.org/cargo/reference/workspaces.html
- **API Guidelines**: https://rust-lang.github.io/api-guidelines/

---

## 🆘 Getting Help

If stuck:

1. Check the example walkthrough: [tdd-example-qbit-core.md](./tdd-example-qbit-core.md)
2. Review the checklist: [refactor-checklist.md](./refactor-checklist.md)
3. Consult the full strategy: [tdd-refactoring-strategy.md](./tdd-refactoring-strategy.md)
4. Check troubleshooting sections in each doc

---

## ✅ Ready to Start?

```bash
# 1. Read the full strategy
cat docs/tdd-refactoring-strategy.md

# 2. Set up tools
./scripts/setup-tdd-tools.sh  # (create this from "Setup" section above)

# 3. Run Phase 0
./scripts/run-phase-0-tests.sh

# 4. Start with qbit-core
# Follow: docs/tdd-example-qbit-core.md

# 5. Track progress
git log --oneline --graph --tags
```

**Good luck!** 🚀

---

**Last Updated**: 2025-12-26
**Status**: Ready for Implementation
