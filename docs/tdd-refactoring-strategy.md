# Test-Driven Development Strategy for Crate Refactoring

**Created**: 2025-12-26
**Status**: Ready for Implementation
**Companion to**: [crate-refactoring-plan.md](./crate-refactoring-plan.md)

---

## Executive Summary

This document defines a comprehensive **Test-Driven Development (TDD)** strategy for refactoring Qbit's monolithic backend into 9 modular crates. The strategy emphasizes **safety**, **continuous validation**, and **incremental progress** with clear rollback criteria.

### Key Principles

1. **Write tests BEFORE extracting code** - Characterize behavior first
2. **Never break existing tests** - All green stays green
3. **Test at multiple levels** - Unit, integration, contract, golden
4. **Automate validation** - CI checks every step
5. **Clear rollback criteria** - Know when to stop and revert

### Test Coverage Targets

| Phase | Unit Tests | Integration Tests | Contract Tests | Property Tests |
|-------|-----------|------------------|----------------|----------------|
| **Pre-Migration** | ≥60% | ✓ Core paths | N/A | ✓ Parsers |
| **During Migration** | ≥70% | ✓ All APIs | ✓ All boundaries | ✓ All pure fns |
| **Post-Migration** | ≥75% | ✓ End-to-end | ✓ Cross-crate | ✓ Invariants |

---

## Phase 0: Pre-Migration Test Suite

**Goal**: Establish a comprehensive baseline test suite BEFORE any refactoring begins.

### 0.1. Characterization Tests

Capture the **actual behavior** of existing code, including quirks and edge cases.

#### What to Test

1. **Tool Execution Behavior** (`backend/src/tools/`)
   ```rust
   // File: backend/tests/characterization/tools.rs

   #[tokio::test]
   async fn test_tool_registry_current_behavior() {
       // Test vtcode-core's exact behavior
       let workspace = TempDir::new().unwrap();
       let mut registry = ToolRegistry::new(workspace.path().to_path_buf()).await;

       // Characterize success response format
       let result = registry.execute_tool("read_file", json!({
           "path": "test.txt"
       })).await.unwrap();

       assert!(result.contains_key("content"));
       assert!(!result.contains_key("error"));

       // Characterize error response format
       let err_result = registry.execute_tool("read_file", json!({
           "path": "nonexistent.txt"
       })).await.unwrap();

       assert!(err_result.contains_key("error"));
   }

   #[tokio::test]
   async fn test_available_tools_output() {
       // Snapshot the exact list of tools
       let registry = ToolRegistry::new(PathBuf::from("/tmp")).await;
       let tools = registry.available_tools().await;

       // Document exact tool list
       insta::assert_yaml_snapshot!(tools);
   }
   ```

2. **Event Serialization** (`backend/src/ai/events.rs`)
   ```rust
   // File: backend/tests/characterization/events.rs

   use insta::assert_json_snapshot;

   #[test]
   fn test_ai_event_serialization_golden() {
       // Golden master: capture exact JSON format
       let events = vec![
           AiEvent::Started { turn_id: "abc123".into() },
           AiEvent::TextDelta { delta: "Hello".into(), accumulated: "Hello".into() },
           AiEvent::ToolApprovalRequest {
               request_id: "req1".into(),
               tool_name: "read_file".into(),
               args: json!({"path": "test.txt"}),
               stats: None,
               risk_level: RiskLevel::Low,
               can_learn: true,
               suggestion: None,
               source: ToolSource::Main,
           },
           AiEvent::Completed {
               response: "Done".into(),
               input_tokens: Some(100),
               output_tokens: Some(50),
               duration_ms: Some(1500),
           },
       ];

       for event in events {
           assert_json_snapshot!(event);
       }
   }
   ```

3. **PTY Parser Behavior** (`backend/src/pty/parser.rs`)
   ```rust
   // File: backend/tests/characterization/pty_parser.rs

   use proptest::prelude::*;

   proptest! {
       #[test]
       fn test_osc_parser_never_panics(data in "\\PC*") {
           let mut parser = TerminalParser::new();
           // Should not panic on any input
           let _ = parser.parse(data.as_bytes());
       }
   }

   #[test]
   fn test_osc_133_sequences() {
       let mut parser = TerminalParser::new();

       // Test each OSC 133 sequence
       let sequences = vec![
           ("\x1b]133;A\x07", OscEvent::PromptStart),
           ("\x1b]133;B\x07", OscEvent::PromptEnd),
           ("\x1b]133;C\x07", OscEvent::CommandStart { command: None }),
           ("\x1b]133;D;0\x07", OscEvent::CommandEnd { exit_code: 0 }),
       ];

       for (input, expected) in sequences {
           let events = parser.parse(input.as_bytes());
           assert_eq!(events, vec![expected]);
       }
   }
   ```

4. **Settings Loading** (`backend/src/settings/`)
   ```rust
   // File: backend/tests/characterization/settings.rs

   #[tokio::test]
   async fn test_settings_env_interpolation() {
       std::env::set_var("TEST_VAR", "test_value");

       let toml = r#"
           test_field = "${TEST_VAR}"
       "#;

       let settings: QbitSettings = toml::from_str(toml).unwrap();
       assert_eq!(settings.test_field, "test_value");
   }
   ```

#### Tools

- **[insta]** - Snapshot testing for exact output comparison
- **[proptest]** - Property-based testing for edge cases
- **[tempfile]** - Isolated test environments

```bash
# Add dependencies
cd backend
cargo add --dev insta proptest tempfile
```

#### Run Characterization Tests

```bash
# Create baseline snapshots
just test-rust
cargo insta review

# Accept all snapshots as baseline
cargo insta accept
```

---

### 0.2. Integration Test Suite

Test **cross-module interactions** before extracting code.

```rust
// File: backend/tests/integration/agent_to_tools.rs

#[tokio::test]
async fn test_agent_executes_tool_via_registry() {
    // Create a minimal agent + tool setup
    let temp = TempDir::new().unwrap();
    let workspace = temp.path().to_path_buf();

    // Write a test file
    std::fs::write(workspace.join("test.txt"), "content").unwrap();

    // Create tool registry
    let mut registry = ToolRegistry::new(workspace.clone()).await;

    // Simulate agent tool execution
    let result = registry.execute_tool("read_file", json!({
        "path": "test.txt"
    })).await;

    assert!(result.is_ok());
    let value = result.unwrap();
    assert!(value.get("content").unwrap().as_str().unwrap().contains("content"));
}
```

```rust
// File: backend/tests/integration/pty_to_runtime.rs

#[tokio::test]
async fn test_pty_manager_with_tauri_runtime() {
    // Test that PTY manager works with TauriRuntime
    // (This will fail if we break the runtime abstraction)

    use qbit_lib::runtime::TauriRuntime;
    use qbit_lib::pty::PtyManager;

    let runtime = TauriRuntime::new_mock(); // Mock for testing
    let manager = PtyManager::new(runtime);

    let session = manager.create_session_with_runtime(
        "/tmp",
        None,
    ).await.unwrap();

    assert!(!session.id.is_empty());
}
```

---

### 0.3. Golden Master Tests

Capture **exact outputs** for critical serialization formats.

```rust
// File: backend/tests/golden/session_archive.rs

use std::fs;

#[tokio::test]
#[serial]
async fn test_session_archive_format_golden() {
    let temp = TempDir::new().unwrap();
    std::env::set_var("VT_SESSION_DIR", temp.path());

    let metadata = SessionArchiveMetadata::new(
        "golden-test",
        "/tmp/workspace".into(),
        "claude-3-5-sonnet",
        "vertex-ai",
        "default",
        "standard",
    );

    let archive = SessionArchive::new(metadata).await.unwrap();

    let messages = vec![
        SessionMessage::with_tool_call_id(
            MessageRole::User,
            "Test prompt",
            None,
        ),
        SessionMessage::with_tool_call_id(
            MessageRole::Assistant,
            "Test response",
            None,
        ),
    ];

    let path = archive.finalize(
        vec!["Transcript line 1".into()],
        1,
        vec![],
        messages,
    ).unwrap();

    // Read the generated JSON
    let content = fs::read_to_string(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Golden snapshot
    insta::assert_json_snapshot!(json, @r###"
    {
      "session_id": "...",
      "workspace": "/tmp/workspace",
      "model": "claude-3-5-sonnet",
      "provider": "vertex-ai",
      "turn_count": 1,
      "messages": [...]
    }
    "###);
}
```

---

### 0.4. Performance Benchmarks

Establish **baseline performance metrics**.

```rust
// File: backend/benches/tool_execution.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qbit_lib::compat::tools::ToolRegistry;

fn bench_tool_execution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let temp = tempfile::TempDir::new().unwrap();

    // Write test file
    std::fs::write(temp.path().join("bench.txt"), "x".repeat(1000)).unwrap();

    let mut registry = rt.block_on(async {
        ToolRegistry::new(temp.path().to_path_buf()).await
    });

    c.bench_function("read_file_1kb", |b| {
        b.to_async(&rt).iter(|| async {
            registry.execute_tool("read_file", json!({
                "path": "bench.txt"
            })).await.unwrap();
        });
    });
}

criterion_group!(benches, bench_tool_execution);
criterion_main!(benches);
```

```toml
# Add to backend/Cargo.toml
[[bench]]
name = "tool_execution"
harness = false

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
```

**Run benchmarks**:
```bash
cd backend
cargo bench --bench tool_execution

# Save baseline
cargo bench --bench tool_execution -- --save-baseline pre-refactor
```

---

### 0.5. Contract Tests

Define **API contracts** between modules.

```rust
// File: backend/tests/contracts/runtime_trait.rs

/// Contract test: QbitRuntime trait must support these operations
#[tokio::test]
async fn test_runtime_trait_contract() {
    // Both TauriRuntime and CliRuntime must pass this test

    trait RuntimeContract {
        async fn emit_event(&self, event: RuntimeEvent) -> Result<()>;
        async fn request_approval(&self, request: ApprovalRequest) -> Result<ApprovalResult>;
    }

    // Test with mock runtime
    struct MockRuntime;

    impl QbitRuntime for MockRuntime {
        async fn emit_event(&self, _event: RuntimeEvent) -> Result<()> {
            Ok(())
        }

        async fn request_approval(&self, _request: ApprovalRequest) -> Result<ApprovalResult> {
            Ok(ApprovalResult::Approved)
        }
    }

    // Verify contract
    let runtime = MockRuntime;
    runtime.emit_event(RuntimeEvent::ToolExecuted {
        tool_name: "test".into(),
    }).await.unwrap();
}
```

---

### Pre-Migration Test Checklist

- [ ] All characterization tests passing (100%)
- [ ] Integration tests cover AI → Tools → PTY → Runtime
- [ ] Golden snapshots captured for:
  - [ ] AiEvent serialization (all 30+ variants)
  - [ ] SessionArchive JSON format
  - [ ] Tool response formats
  - [ ] OSC event parsing
- [ ] Performance benchmarks saved as baseline
- [ ] Contract tests define all cross-module interfaces
- [ ] Property tests cover:
  - [ ] PTY parser (fuzz testing)
  - [ ] Shell command parsing
  - [ ] Path validation
  - [ ] Diff parsing
- [ ] Test coverage ≥60% (`cargo tarpaulin`)

---

## Phase 1: Per-Crate TDD Extraction Strategy

For **each crate extraction**, follow this TDD workflow.

### Step 1: Write Interface Tests First

Define the **public API** in tests before creating the crate.

```rust
// File: backend/tests/crates/qbit_settings_interface.rs
// Written BEFORE extracting qbit-settings

#[tokio::test]
async fn test_settings_manager_interface() {
    // This test will fail until we implement qbit-settings

    use qbit_settings::SettingsManager;

    let manager = SettingsManager::new().await.unwrap();

    // Test get()
    let settings = manager.get().await;
    assert!(settings.ai.model.contains("claude"));

    // Test update()
    let mut new_settings = settings.clone();
    new_settings.ai.model = "claude-3-opus".to_string();
    manager.update(new_settings.clone()).await.unwrap();

    let updated = manager.get().await;
    assert_eq!(updated.ai.model, "claude-3-opus");
}
```

### Step 2: Extract Code

Move code to the new crate, making tests pass.

```bash
# Create crate structure
mkdir -p backend/crates/qbit-settings/src

# Move files
cp backend/src/settings/*.rs backend/crates/qbit-settings/src/

# Create Cargo.toml
cat > backend/crates/qbit-settings/Cargo.toml <<EOF
[package]
name = "qbit-settings"
version.workspace = true
edition.workspace = true

[dependencies]
serde.workspace = true
toml = "0.8"
tokio.workspace = true
dirs.workspace = true
EOF
```

### Step 3: Write Boundary Tests

Test the **interface between crates**.

```rust
// File: backend/crates/qbit-settings/tests/integration.rs

#[tokio::test]
async fn test_settings_isolated_in_crate() {
    // This test runs in the qbit-settings crate
    // It verifies the crate works in isolation

    use qbit_settings::SettingsManager;
    use tempfile::TempDir;

    let temp = TempDir::new().unwrap();
    std::env::set_var("QBIT_CONFIG_DIR", temp.path());

    let manager = SettingsManager::new().await.unwrap();
    let settings = manager.get().await;

    // Should have default values
    assert!(!settings.ai.model.is_empty());
}
```

### Step 4: Write Feature Flag Tests

Ensure feature flags work correctly.

```rust
// File: backend/crates/qbit-settings/tests/features.rs

#[cfg(feature = "tauri")]
#[test]
fn test_tauri_commands_available() {
    // Tauri commands should only be available with feature flag
    use qbit_settings::commands::{get_settings, update_settings};

    // If this compiles, the feature flag works
}

#[cfg(not(feature = "tauri"))]
#[test]
fn test_tauri_commands_not_available() {
    // This should fail to compile if tauri is enabled
    // (Run with: cargo test --no-default-features)
}
```

### Step 5: Compilation Test

**Critical**: Verify the crate compiles in isolation.

```bash
# Test compilation WITHOUT workspace dependencies
cd backend/crates/qbit-settings

# Clean build (no workspace cache)
cargo clean
cargo build

# Test with all feature combinations
cargo build --no-default-features
cargo build --features tauri
cargo build --all-features

# Run isolated tests
cargo test
```

### Step 6: Cross-Crate Integration Tests

Test the **interaction** between the new crate and existing code.

```rust
// File: backend/tests/integration/main_crate_uses_settings.rs

#[tokio::test]
async fn test_main_crate_imports_settings() {
    // Test that main crate can use qbit-settings

    use qbit_settings::SettingsManager;
    use qbit_lib::ai::AgentBridge; // Main crate

    let settings = SettingsManager::new().await.unwrap().get().await;

    // AgentBridge should be able to use settings
    let _bridge = AgentBridge::new(
        settings.ai.model.clone(),
        settings.workspace.clone(),
    );
}
```

---

## Phase 2: Continuous Validation

### After Each Extraction Step

Run this validation suite:

```bash
#!/bin/bash
# File: scripts/validate-refactor-step.sh

set -e

echo "=== Step Validation ==="

# 1. Build all crates
echo "Building workspace..."
cargo build --workspace

# 2. Run all tests
echo "Running tests..."
cargo test --workspace --features local-tools

# 3. Check formatting
echo "Checking format..."
cargo fmt --check

# 4. Clippy (zero warnings)
echo "Running clippy..."
cargo clippy --workspace -- -D warnings

# 5. Feature flag matrix
echo "Testing feature combinations..."
cargo test -p qbit --no-default-features --features cli
cargo test -p qbit --no-default-features --features tauri
cargo test -p qbit --features cli,local-tools

# 6. Integration tests
echo "Running integration tests..."
cargo test --test compat_layer --features local-tools

# 7. Benchmark regression check
echo "Checking performance..."
cargo bench --bench tool_execution -- --baseline pre-refactor

# 8. Coverage check
echo "Checking coverage..."
cargo tarpaulin --workspace --out Stdout --fail-under 70

echo "✓ All validations passed"
```

### Regression Test Suite

Run after **every commit**:

```rust
// File: backend/tests/regression/mod.rs

mod ai_event_serialization {
    use insta::assert_json_snapshot;

    #[test]
    fn test_no_event_format_changes() {
        // Regression: Ensure AiEvent format doesn't change
        let event = AiEvent::Started { turn_id: "test".into() };
        assert_json_snapshot!(event);
    }
}

mod tool_execution {
    #[tokio::test]
    async fn test_tool_execution_still_works() {
        // Regression: Basic tool execution
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.txt"), "content").unwrap();

        let mut registry = ToolRegistry::new(temp.path().to_path_buf()).await;
        let result = registry.execute_tool("read_file", json!({
            "path": "test.txt"
        })).await;

        assert!(result.is_ok());
    }
}
```

---

## Phase 3: Safety Mechanisms

### Snapshot Tests for API Surfaces

Detect **accidental API changes**.

```rust
// File: backend/tests/snapshots/public_api.rs

use insta::assert_snapshot;

#[test]
fn test_qbit_core_public_api() {
    // Use cargo-public-api to snapshot the public API
    let output = std::process::Command::new("cargo")
        .args(["public-api", "-p", "qbit-core"])
        .output()
        .unwrap();

    let api = String::from_utf8(output.stdout).unwrap();
    assert_snapshot!(api);
}
```

```bash
# Install cargo-public-api
cargo install cargo-public-api

# Generate API snapshots
cargo public-api -p qbit-core > tests/snapshots/qbit-core-api.txt
cargo public-api -p qbit-settings > tests/snapshots/qbit-settings-api.txt
```

### Contract Tests Between Crates

Define **explicit contracts** at crate boundaries.

```rust
// File: backend/tests/contracts/qbit_core_to_runtime.rs

/// Contract: qbit-core defines QbitRuntime trait
/// Contract: qbit-runtime implements QbitRuntime trait
/// Contract: Both TauriRuntime and CliRuntime must be compatible

#[test]
fn test_runtime_trait_exists_in_core() {
    use qbit_core::runtime::QbitRuntime;

    // Trait must have these methods
    fn _assert_trait<T: QbitRuntime>() {}
}

#[test]
#[cfg(feature = "tauri")]
fn test_tauri_runtime_implements_contract() {
    use qbit_core::runtime::QbitRuntime;
    use qbit_runtime::TauriRuntime;

    fn _assert_impl<T: QbitRuntime>(_: &T) {}

    // This compiles only if TauriRuntime implements QbitRuntime correctly
}
```

### Property-Based Tests for Invariants

Use **proptest** to verify invariants hold.

```rust
// File: backend/crates/qbit-pty/tests/properties.rs

use proptest::prelude::*;

proptest! {
    #[test]
    fn test_parser_never_panics_on_random_input(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        let mut parser = TerminalParser::new();
        let _ = parser.parse(&data); // Must not panic
    }

    #[test]
    fn test_session_id_always_valid_uuid(seed in any::<u64>()) {
        let session_id = generate_session_id(seed);
        assert!(uuid::Uuid::parse_str(&session_id).is_ok());
    }

    #[test]
    fn test_path_validation_invariants(path in "\\PC{1,100}") {
        // If validation passes, path must be absolute
        if let Ok(validated) = validate_workspace_path(&path) {
            assert!(validated.is_absolute());
        }
    }
}
```

### Compatibility Tests for Old Behavior

Ensure **backward compatibility** during migration.

```rust
// File: backend/tests/compatibility/local_vs_vtcode.rs

/// Test that local-tools and vtcode-core behave identically

#[tokio::test]
async fn test_tool_registry_compatibility() {
    let temp = TempDir::new().unwrap();
    std::fs::write(temp.path().join("test.txt"), "hello").unwrap();

    #[cfg(feature = "local-tools")]
    let mut registry = {
        use qbit_lib::compat::tools::ToolRegistry;
        ToolRegistry::new(temp.path().to_path_buf()).await
    };

    #[cfg(not(feature = "local-tools"))]
    let mut registry = {
        use vtcode_core::tools::ToolRegistry;
        ToolRegistry::new(temp.path().to_path_buf()).await
    };

    // Both should produce identical results
    let result = registry.execute_tool("read_file", json!({
        "path": "test.txt"
    })).await.unwrap();

    assert_eq!(result.get("content").unwrap().as_str().unwrap(), "hello");
}
```

---

## Phase 4: Rollback Criteria

### When to STOP and ROLLBACK

❌ **Hard Stop Criteria** (revert immediately):

1. **Test coverage drops below 60%**
   ```bash
   cargo tarpaulin --workspace --out Stdout
   # If coverage < 60%, STOP
   ```

2. **Any test failure in main branch**
   ```bash
   cargo test --workspace
   # Exit code != 0 → ROLLBACK
   ```

3. **Performance regression >20%**
   ```bash
   cargo bench --bench tool_execution -- --baseline pre-refactor
   # If >20% slower, investigate or revert
   ```

4. **Compilation failure in feature matrix**
   ```bash
   # All these MUST succeed:
   cargo build --no-default-features --features cli
   cargo build --no-default-features --features tauri
   cargo build --features cli,local-tools,evals
   ```

5. **Circular dependency detected**
   ```bash
   cargo tree -p qbit-core --edges normal
   # If qbit-core appears in its own tree (other than root), STOP
   ```

⚠️ **Warning Criteria** (investigate before proceeding):

1. **Clippy warnings increase by >5**
2. **Build time increases by >30%**
3. **Any snapshot test failure** (indicates API change)
4. **Integration test flakiness** (>1% failure rate)

### Rollback Procedure

```bash
# 1. Identify the last good commit
git log --oneline --graph

# 2. Create a backup branch
git branch backup-before-rollback

# 3. Revert to last known good state
git reset --hard <last-good-commit>

# 4. Verify tests pass
just test

# 5. Document what went wrong
cat > docs/rollback-$(date +%Y%m%d).md <<EOF
# Rollback: $(date)

## Reason
[Why we rolled back]

## Failed Tests
[Which tests failed]

## Lessons Learned
[What to do differently]
EOF
```

---

## Phase 5: Test Automation

### CI Pipeline Integration

```yaml
# File: .github/workflows/refactor-validation.yml

name: Refactor Validation

on:
  push:
    branches: [feat/workspace-refactor]
  pull_request:

jobs:
  test-matrix:
    name: Test Feature Combinations
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features:
          - ""
          - "cli"
          - "tauri"
          - "cli,local-tools"
          - "tauri,local-tools"
          - "cli,local-tools,evals"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Test ${{ matrix.features }}
        run: |
          if [ -z "${{ matrix.features }}" ]; then
            cargo test --workspace
          else
            cargo test --workspace --no-default-features --features "${{ matrix.features }}"
          fi

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Run coverage
        run: cargo tarpaulin --workspace --out Xml --fail-under 70
      - name: Upload to codecov
        uses: codecov/codecov-action@v3

  benchmarks:
    name: Performance Regression Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run benchmarks
        run: |
          cargo bench --bench tool_execution -- --baseline main
          # Fail if >20% slower
          cargo bench --bench tool_execution -- --baseline-lenient 1.2

  api-stability:
    name: API Stability Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install cargo-public-api
        run: cargo install cargo-public-api
      - name: Check API changes
        run: |
          cargo public-api -p qbit-core > api-current.txt
          diff tests/snapshots/qbit-core-api.txt api-current.txt || {
            echo "⚠️ Public API changed!"
            exit 1
          }

  integration:
    name: Integration Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run integration tests
        run: cargo test --test compat_layer --features local-tools

  clippy:
    name: Clippy (zero warnings)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Run clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
```

### Pre-Commit Hooks

```bash
# File: .git/hooks/pre-commit

#!/bin/bash
set -e

echo "Running pre-commit checks..."

# 1. Format check
cargo fmt --check || {
    echo "❌ Format check failed. Run: cargo fmt"
    exit 1
}

# 2. Clippy
cargo clippy --workspace -- -D warnings || {
    echo "❌ Clippy failed"
    exit 1
}

# 3. Quick test (unit tests only)
cargo test --lib --workspace || {
    echo "❌ Unit tests failed"
    exit 1
}

echo "✓ Pre-commit checks passed"
```

### Nightly Regression Suite

```bash
# File: scripts/nightly-regression.sh

#!/bin/bash
# Run comprehensive regression tests nightly

set -e

# 1. Full test suite
cargo test --workspace --all-features

# 2. Integration tests with both implementations
cargo test --test compat_layer
cargo test --test compat_layer --features local-tools

# 3. Benchmarks
cargo bench --bench tool_execution
cargo bench --bench pty_parsing
cargo bench --bench event_serialization

# 4. Coverage report
cargo tarpaulin --workspace --out Html --output-dir coverage/

# 5. Property tests (longer runs)
PROPTEST_CASES=10000 cargo test --workspace

# 6. Memory leak check (valgrind)
cargo build --workspace
valgrind --leak-check=full ./target/debug/qbit-cli --help

# 7. Generate report
cat > regression-report-$(date +%Y%m%d).md <<EOF
# Regression Report - $(date)

## Test Results
- Total tests: $(cargo test --workspace 2>&1 | grep -c "test result:")
- Coverage: $(cargo tarpaulin --workspace --out Stdout | grep -oP '\d+\.\d+(?=%)')

## Benchmarks
$(cargo bench 2>&1)

## Status: ✓ PASSED
EOF
```

---

## Concrete Test Examples

### Example 1: qbit-core Extraction

```rust
// File: backend/tests/extraction/qbit_core.rs
// Written BEFORE extracting qbit-core

#[test]
fn test_qbit_core_has_no_dependencies() {
    // qbit-core must have zero internal dependencies
    let metadata = cargo_metadata::MetadataCommand::new()
        .exec()
        .unwrap();

    let qbit_core = metadata.packages.iter()
        .find(|p| p.name == "qbit-core")
        .expect("qbit-core package not found");

    // Should only depend on external crates (serde, chrono, etc.)
    let internal_deps: Vec<_> = qbit_core.dependencies.iter()
        .filter(|d| d.path.is_some()) // Local dependencies
        .collect();

    assert_eq!(internal_deps.len(), 0,
        "qbit-core should have zero internal dependencies");
}

#[test]
fn test_ai_event_enum_complete() {
    // Ensure all AiEvent variants are tested
    use qbit_core::events::AiEvent;

    // This list must match the actual enum
    let expected_variants = vec![
        "Started", "TextDelta", "ToolRequest", "ToolApprovalRequest",
        "ToolAutoApproved", "ToolDenied", "ToolResult", "Reasoning",
        "Completed", "Error", "SubAgentStarted", "SubAgentCompleted",
        // ... all 30+ variants
    ];

    // Serialize one of each and check discriminant
    // (This will fail to compile if we add a variant without updating the test)
}
```

### Example 2: qbit-settings Feature Flags

```rust
// File: backend/crates/qbit-settings/tests/feature_flags.rs

#[cfg(feature = "tauri")]
mod tauri_enabled {
    use qbit_settings::commands;

    #[test]
    fn test_tauri_commands_exist() {
        // Commands should be available
        let _ = commands::get_settings;
        let _ = commands::update_settings;
    }
}

#[cfg(not(feature = "tauri"))]
mod tauri_disabled {
    #[test]
    fn test_tauri_commands_not_compiled() {
        // This test ensures commands module doesn't exist without feature
        // If this fails to compile, the feature flag isn't working

        // Uncomment to verify it fails:
        // use qbit_settings::commands; // Should not compile
    }
}

// Compilation test: run with multiple feature combinations
// cargo test --no-default-features
// cargo test --features tauri
```

### Example 3: qbit-pty Parser Property Tests

```rust
// File: backend/crates/qbit-pty/tests/parser_properties.rs

use proptest::prelude::*;
use qbit_pty::parser::{TerminalParser, OscEvent};

proptest! {
    /// Property: Parser never panics on any input
    #[test]
    fn parser_never_panics(data in prop::collection::vec(any::<u8>(), 0..10000)) {
        let mut parser = TerminalParser::new();
        let _ = parser.parse(&data);
    }

    /// Property: Valid OSC sequences are always parsed correctly
    #[test]
    fn valid_osc_sequences_parse(
        exit_code in 0i32..255,
        command in "\\PC{0,100}"
    ) {
        let mut parser = TerminalParser::new();

        // OSC 133;D;<exit_code>
        let sequence = format!("\x1b]133;D;{}\x07", exit_code);
        let events = parser.parse(sequence.as_bytes());

        // Should parse exactly one CommandEnd event
        assert_eq!(events.len(), 1);
        if let OscEvent::CommandEnd { exit_code: parsed } = events[0] {
            assert_eq!(parsed, exit_code);
        } else {
            panic!("Expected CommandEnd event");
        }
    }

    /// Property: Parser is idempotent (parsing twice gives same result)
    #[test]
    fn parser_is_idempotent(data in prop::collection::vec(any::<u8>(), 0..1000)) {
        let mut parser1 = TerminalParser::new();
        let mut parser2 = TerminalParser::new();

        let result1 = parser1.parse(&data);
        let result2 = parser2.parse(&data);

        assert_eq!(result1, result2);
    }
}
```

### Example 4: Cross-Crate Contract Test

```rust
// File: backend/tests/contracts/tools_to_runtime.rs

/// Contract: Tools use Runtime to emit events
/// Both qbit-tools and qbit-runtime must honor this contract

use qbit_core::runtime::QbitRuntime;
use qbit_tools::ToolRegistry;

#[tokio::test]
async fn test_tools_emit_via_runtime() {
    use std::sync::{Arc, Mutex};

    // Mock runtime that records events
    #[derive(Clone)]
    struct RecordingRuntime {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl QbitRuntime for RecordingRuntime {
        async fn emit_event(&self, event: RuntimeEvent) -> anyhow::Result<()> {
            self.events.lock().unwrap().push(format!("{:?}", event));
            Ok(())
        }

        async fn request_approval(&self, _: ApprovalRequest) -> anyhow::Result<ApprovalResult> {
            Ok(ApprovalResult::Approved)
        }
    }

    let runtime = RecordingRuntime {
        events: Arc::new(Mutex::new(vec![])),
    };

    // Tool execution should emit events through runtime
    let temp = tempfile::TempDir::new().unwrap();
    let mut registry = ToolRegistry::new_with_runtime(
        temp.path().to_path_buf(),
        runtime.clone(),
    ).await;

    registry.execute_tool("read_file", json!({"path": "test.txt"})).await.ok();

    // Verify events were emitted
    let events = runtime.events.lock().unwrap();
    assert!(!events.is_empty(), "Tools should emit events via runtime");
}
```

---

## Tools and Frameworks

### Testing Infrastructure

| Tool | Purpose | Installation |
|------|---------|--------------|
| **[cargo-nextest]** | Faster test runner | `cargo install cargo-nextest` |
| **[cargo-tarpaulin]** | Code coverage | `cargo install cargo-tarpaulin` |
| **[cargo-watch]** | Auto-run tests | `cargo install cargo-watch` |
| **[cargo-public-api]** | API snapshots | `cargo install cargo-public-api` |
| **[insta]** | Snapshot testing | `cargo add --dev insta` |
| **[proptest]** | Property-based testing | `cargo add --dev proptest` |
| **[criterion]** | Benchmarking | `cargo add --dev criterion` |
| **[tempfile]** | Test isolation | `cargo add --dev tempfile` |
| **[serial_test]** | Serial test execution | `cargo add --dev serial_test` |

[cargo-nextest]: https://nexte.st/
[cargo-tarpaulin]: https://github.com/xd009642/tarpaulin
[cargo-watch]: https://github.com/watchexec/cargo-watch
[cargo-public-api]: https://github.com/Enselic/cargo-public-api
[insta]: https://insta.rs/
[proptest]: https://altsysrq.github.io/proptest-book/
[criterion]: https://github.com/bheisler/criterion.rs

### Test Commands

```bash
# Fast test runner
cargo nextest run --workspace

# Watch mode (auto-run on file changes)
cargo watch -x "nextest run"

# Coverage report
cargo tarpaulin --workspace --out Html

# Benchmark with comparison
cargo bench --bench tool_execution -- --baseline main

# Snapshot review
cargo insta review

# Property tests (extended)
PROPTEST_CASES=10000 cargo test
```

### Quality Gates (CI/CD)

```yaml
# Required checks before merging:
quality-gates:
  - name: All tests pass
    command: cargo nextest run --workspace --all-features
    required: true

  - name: Coverage ≥70%
    command: cargo tarpaulin --workspace --fail-under 70
    required: true

  - name: Zero clippy warnings
    command: cargo clippy --workspace -- -D warnings
    required: true

  - name: API stability
    command: cargo public-api diff main
    required: false  # Warn only

  - name: Performance regression <20%
    command: cargo bench -- --baseline main
    required: false  # Warn only

  - name: Feature matrix
    command: |
      cargo build --no-default-features --features cli &&
      cargo build --no-default-features --features tauri &&
      cargo build --features cli,local-tools,evals
    required: true
```

---

## Step-by-Step TDD Workflow

### Per-Crate Extraction Checklist

Use this checklist for **each crate** (e.g., qbit-settings, qbit-pty):

#### Before Extraction

- [ ] Write interface tests (define public API)
- [ ] Write contract tests (define interactions with other crates)
- [ ] Write characterization tests (capture current behavior)
- [ ] Document expected dependencies (`Cargo.toml` draft)
- [ ] Create extraction script (`scripts/extract-<crate-name>.sh`)

#### During Extraction

- [ ] Create crate structure (`mkdir -p backend/crates/<name>/src`)
- [ ] Move files to new crate
- [ ] Create `Cargo.toml` with minimal dependencies
- [ ] Create `lib.rs` with public exports
- [ ] Update imports in main crate (`crate::` → `<crate_name>::`)
- [ ] Add crate to workspace members

#### After Extraction

- [ ] **Compilation test**: `cargo build -p <crate-name>`
- [ ] **Isolated test**: `cargo test -p <crate-name>`
- [ ] **Feature flag test**: `cargo build -p <crate-name> --no-default-features`
- [ ] **Integration test**: `cargo test --workspace`
- [ ] **Coverage check**: `cargo tarpaulin -p <crate-name>`
- [ ] **API snapshot**: `cargo public-api -p <crate-name> > snapshots/<crate>-api.txt`
- [ ] **Benchmark**: `cargo bench --bench <crate>_bench`
- [ ] **Clippy**: `cargo clippy -p <crate-name> -- -D warnings`

#### Validation

- [ ] All workspace tests pass: `cargo test --workspace`
- [ ] Feature matrix passes (see CI pipeline)
- [ ] Coverage ≥70%
- [ ] No new clippy warnings
- [ ] Benchmarks within 5% of baseline
- [ ] API snapshot reviewed and approved
- [ ] Documentation updated

#### Final Steps

- [ ] Commit with message: `feat(<crate>): extract <crate-name> crate`
- [ ] Update refactoring plan progress
- [ ] Tag commit: `git tag refactor-step-<N>`

---

## Success Metrics

### Test Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Test coverage** | ≥75% | `cargo tarpaulin` |
| **Test pass rate** | 100% | CI dashboard |
| **Flaky test rate** | <1% | CI analytics |
| **Test execution time** | <2 min | `cargo nextest` |
| **Property test cases** | ≥1000/test | `PROPTEST_CASES` |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Clippy warnings** | 0 | `cargo clippy` |
| **Public API stability** | No breaking changes | `cargo public-api diff` |
| **Dependency count (per crate)** | <10 | `cargo tree` |
| **Build time (incremental)** | <30s | `cargo build --timings` |

### Safety Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Rollback count** | 0 | Git history |
| **Test regressions** | 0 | CI logs |
| **Performance regressions** | 0 (>20% slower) | Benchmarks |
| **Breaking changes** | 0 | API diff |

---

## Example Workflow: Extracting qbit-settings

**Day 1: Write Tests**

```bash
# 1. Create test file
cat > backend/tests/extraction/qbit_settings.rs <<EOF
// Interface test - defines the API we want

#[tokio::test]
async fn test_settings_manager_interface() {
    use qbit_settings::SettingsManager;

    let mgr = SettingsManager::new().await.unwrap();
    let settings = mgr.get().await;
    assert!(!settings.ai.model.is_empty());
}
EOF

# 2. Run test (should fail - crate doesn't exist yet)
cargo test test_settings_manager_interface
# ❌ Error: qbit_settings not found

# 3. Write characterization test
cat > backend/tests/characterization/settings_current.rs <<EOF
#[tokio::test]
async fn test_current_settings_behavior() {
    use qbit_lib::settings::SettingsManager;

    let mgr = SettingsManager::new().await.unwrap();
    let settings = mgr.get().await;

    // Document current behavior
    insta::assert_yaml_snapshot!(settings);
}
EOF

cargo test test_current_settings_behavior
cargo insta accept
```

**Day 2: Extract Crate**

```bash
# 1. Create crate structure
./scripts/extract-qbit-settings.sh

# 2. Test compilation
cd backend/crates/qbit-settings
cargo build
# ✓ Compiles

# 3. Run isolated tests
cargo test
# ✓ All pass

# 4. Update main crate imports
cd ../../
rg "crate::settings::" --files-with-matches | xargs sed -i 's/crate::settings::/qbit_settings::/g'

# 5. Test workspace
cargo test --workspace
# ✓ All pass
```

**Day 3: Validation & Polish**

```bash
# 1. Coverage check
cargo tarpaulin -p qbit-settings
# Coverage: 78% ✓

# 2. API snapshot
cargo public-api -p qbit-settings > tests/snapshots/qbit-settings-api.txt

# 3. Feature flag test
cargo test -p qbit-settings --no-default-features
cargo test -p qbit-settings --features tauri

# 4. Benchmark
cargo bench --bench settings_bench

# 5. Full validation
./scripts/validate-refactor-step.sh
# ✓ All checks passed

# 6. Commit
git add .
git commit -m "feat(settings): extract qbit-settings crate

- Moved settings module to standalone crate
- Coverage: 78%
- Zero breaking changes
- All tests passing"

git tag refactor-step-1
```

---

## Summary

This TDD strategy provides:

1. **Pre-migration baseline** - Comprehensive characterization tests
2. **Per-crate workflow** - Step-by-step extraction with tests first
3. **Continuous validation** - Automated checks after every step
4. **Safety mechanisms** - Snapshots, contracts, property tests
5. **Clear rollback criteria** - Know when to stop
6. **Automation** - CI/CD integration with quality gates

### Key Takeaways

- ✅ **Write tests BEFORE extracting code**
- ✅ **Test at multiple levels** (unit, integration, contract, property)
- ✅ **Automate everything** (CI runs all tests on every commit)
- ✅ **Have clear rollback criteria** (don't continue with failing tests)
- ✅ **Use snapshots for stability** (detect accidental changes)
- ✅ **Property tests for robustness** (find edge cases automatically)

### Next Steps

1. Review this plan with team
2. Set up test infrastructure:
   ```bash
   cargo install cargo-nextest cargo-tarpaulin cargo-public-api
   cargo add --dev insta proptest criterion
   ```
3. Run Phase 0 (pre-migration tests)
4. Begin Phase 1 with qbit-core extraction
5. Follow the per-crate checklist for each extraction

---

**Document Version**: 1.0
**Last Updated**: 2025-12-26
**Status**: Ready for Implementation
