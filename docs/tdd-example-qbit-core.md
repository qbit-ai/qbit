# TDD Example: Extracting qbit-core Crate

**Example walkthrough** showing how to apply the TDD strategy to extract the **qbit-core** crate.

**Estimated time**: 2-3 days
**Difficulty**: Medium
**Risk**: Low (pure types, no logic)

---

## Overview

qbit-core is the **foundation crate** containing:
- Event types (AiEvent, RuntimeEvent, SidecarEvent)
- Runtime trait (QbitRuntime)
- Session types (SessionArchive, SessionMessage, MessageRole)
- Error types

**Key constraint**: qbit-core must have **zero internal dependencies** (only external deps like serde, chrono).

---

## Day 1: Write Tests BEFORE Extraction

### Step 1: Interface Tests

Define the API we want in tests **before** creating the crate.

```rust
// File: backend/tests/extraction/qbit_core_interface.rs
// This test will fail until we create qbit-core

use qbit_core::events::{AiEvent, ToolSource};
use qbit_core::runtime::{QbitRuntime, ApprovalRequest, ApprovalResult};
use qbit_core::session::{SessionMessage, MessageRole};

#[test]
fn test_qbit_core_event_types_exist() {
    // Test that all event types are accessible

    // AiEvent variants
    let _started = AiEvent::Started {
        turn_id: "test".into(),
    };

    let _text = AiEvent::TextDelta {
        delta: "hello".into(),
        accumulated: "hello".into(),
    };

    let _tool_request = AiEvent::ToolApprovalRequest {
        request_id: "req1".into(),
        tool_name: "read_file".into(),
        args: serde_json::json!({}),
        stats: None,
        risk_level: RiskLevel::Low,
        can_learn: true,
        suggestion: None,
        source: ToolSource::Main,
    };

    let _completed = AiEvent::Completed {
        response: "done".into(),
        input_tokens: Some(100),
        output_tokens: Some(50),
        duration_ms: Some(1000),
    };

    // This test will fail to compile until qbit-core exists
}

#[test]
fn test_qbit_core_runtime_trait_exists() {
    // Test that QbitRuntime trait is defined correctly

    use async_trait::async_trait;

    // Mock implementation for testing
    struct MockRuntime;

    #[async_trait]
    impl QbitRuntime for MockRuntime {
        async fn emit_event(&self, _event: RuntimeEvent) -> anyhow::Result<()> {
            Ok(())
        }

        async fn request_approval(
            &self,
            _request: ApprovalRequest,
        ) -> anyhow::Result<ApprovalResult> {
            Ok(ApprovalResult::Approved)
        }
    }

    // If this compiles, the trait is defined correctly
    let _runtime = MockRuntime;
}

#[test]
fn test_qbit_core_session_types_exist() {
    // Test session types

    let _message = SessionMessage::with_tool_call_id(
        MessageRole::User,
        "Test message",
        None,
    );

    let _role_user = MessageRole::User;
    let _role_assistant = MessageRole::Assistant;
    let _role_system = MessageRole::System;
    let _role_tool = MessageRole::Tool;
}
```

**Run the test** (should fail):

```bash
cargo test test_qbit_core_interface
# Error: cannot find crate `qbit_core`
```

✅ **Good!** The test defines what we want. Now we know what to build.

---

### Step 2: Characterization Tests

Capture the **current behavior** before moving code.

```rust
// File: backend/tests/characterization/events_current.rs

use qbit_lib::ai::events::{AiEvent, ToolSource};
use insta::assert_json_snapshot;

#[test]
fn test_ai_event_serialization_baseline() {
    // Capture current serialization format

    let events = vec![
        AiEvent::Started {
            turn_id: "abc123".into(),
        },
        AiEvent::TextDelta {
            delta: "Hello".into(),
            accumulated: "Hello".into(),
        },
        AiEvent::ToolApprovalRequest {
            request_id: "req1".into(),
            tool_name: "read_file".into(),
            args: serde_json::json!({"path": "test.txt"}),
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

    // Snapshot each event's JSON format
    for (i, event) in events.iter().enumerate() {
        assert_json_snapshot!(format!("event_{}", i), event);
    }
}

#[test]
fn test_session_message_serialization_baseline() {
    use qbit_lib::session::{SessionMessage, MessageRole};

    let messages = vec![
        SessionMessage::with_tool_call_id(
            MessageRole::User,
            "User message",
            None,
        ),
        SessionMessage::with_tool_call_id(
            MessageRole::Assistant,
            "Assistant response",
            None,
        ),
        SessionMessage::with_tool_call_id(
            MessageRole::Tool,
            "Tool result",
            Some("call_123".into()),
        ),
    ];

    for (i, msg) in messages.iter().enumerate() {
        assert_json_snapshot!(format!("message_{}", i), msg);
    }
}
```

**Run and save snapshots**:

```bash
cargo test test_ai_event_serialization_baseline
cargo test test_session_message_serialization_baseline

# Review snapshots
cargo insta review

# Accept as baseline
cargo insta accept
```

---

### Step 3: Contract Tests

Define **contracts** between qbit-core and other crates.

```rust
// File: backend/tests/contracts/qbit_core_contracts.rs

/// Contract 1: qbit-core has zero internal dependencies
#[test]
fn test_qbit_core_has_no_internal_deps() {
    // This will run after extraction
    // For now, document the requirement

    // When qbit-core exists, it must depend only on:
    // - serde
    // - serde_json
    // - thiserror
    // - async-trait
    // - chrono
    // - uuid
    //
    // NO internal deps (qbit-*, except itself)
}

/// Contract 2: QbitRuntime trait must be implemented by qbit-runtime
#[test]
fn test_runtime_trait_contract() {
    // After extraction, both TauriRuntime and CliRuntime
    // must implement QbitRuntime from qbit-core

    // This ensures the boundary is correct
}

/// Contract 3: AiEvent must be serializable
#[test]
fn test_ai_event_serde_contract() {
    use serde::{Serialize, Deserialize};

    // All AiEvent variants must derive Serialize + Deserialize
    fn _assert_serde<T: Serialize + for<'de> Deserialize<'de>>() {}

    // This will be checked with qbit_core::events::AiEvent
}
```

---

### Step 4: Property Tests

Define **invariants** that must hold.

```rust
// File: backend/tests/properties/events.rs

use proptest::prelude::*;
use qbit_lib::ai::events::AiEvent;

proptest! {
    /// Property: All AiEvent variants can be serialized and deserialized
    #[test]
    fn ai_event_roundtrip(turn_id in "\\PC{1,100}") {
        let event = AiEvent::Started { turn_id: turn_id.clone() };

        // Serialize
        let json = serde_json::to_string(&event).unwrap();

        // Deserialize
        let deserialized: AiEvent = serde_json::from_str(&json).unwrap();

        // Should match
        if let AiEvent::Started { turn_id: id } = deserialized {
            assert_eq!(id, turn_id);
        } else {
            panic!("Wrong variant after deserialization");
        }
    }

    /// Property: MessageRole is exhaustive (all variants covered)
    #[test]
    fn message_role_exhaustive(role_idx in 0u8..4) {
        use qbit_lib::session::MessageRole;

        let role = match role_idx {
            0 => MessageRole::User,
            1 => MessageRole::Assistant,
            2 => MessageRole::System,
            3 => MessageRole::Tool,
            _ => unreachable!(),
        };

        // Should serialize successfully
        let json = serde_json::to_string(&role).unwrap();
        assert!(!json.is_empty());
    }
}
```

---

## Day 2: Extract the Crate

### Step 1: Create Crate Structure

```bash
# Create directory
mkdir -p backend/crates/qbit-core/src/{events,runtime,session}

# Create Cargo.toml
cat > backend/crates/qbit-core/Cargo.toml <<'EOF'
[package]
name = "qbit-core"
version.workspace = true
edition.workspace = true
description = "Core types and traits for Qbit"
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
EOF
```

---

### Step 2: Create lib.rs

```rust
// File: backend/crates/qbit-core/src/lib.rs

//! # qbit-core
//!
//! Core types and traits for the Qbit application.
//!
//! This crate provides:
//! - Event types (`AiEvent`, `RuntimeEvent`, `SidecarEvent`)
//! - Runtime trait (`QbitRuntime`)
//! - Session types (`SessionArchive`, `SessionMessage`, `MessageRole`)
//! - Error types
//!
//! ## Design Principles
//!
//! 1. **Zero internal dependencies** - only depends on external crates
//! 2. **Pure types** - no business logic
//! 3. **Serialization-first** - all types derive `Serialize` + `Deserialize`
//! 4. **Trait-based abstractions** - runtime is a trait, not a struct

pub mod events;
pub mod runtime;
pub mod session;
pub mod error;

// Re-export commonly used types at crate root
pub use events::{AiEvent, RuntimeEvent, SidecarEvent, ToolSource};
pub use runtime::{QbitRuntime, ApprovalRequest, ApprovalResult, RuntimeError};
pub use session::{
    MessageRole, SessionArchive, SessionArchiveMetadata, SessionListing, SessionMessage,
    SessionSnapshot,
};
pub use error::CoreError;
```

---

### Step 3: Move Events

```bash
# Copy events.rs
cp backend/src/ai/events.rs backend/crates/qbit-core/src/events/ai.rs

# Create events/mod.rs
cat > backend/crates/qbit-core/src/events/mod.rs <<'EOF'
mod ai;
mod runtime;
mod sidecar;

pub use ai::AiEvent;
pub use runtime::RuntimeEvent;
pub use sidecar::SidecarEvent;

// Re-export ToolSource
pub use ai::ToolSource;
EOF
```

**Edit `events/ai.rs`** to remove local dependencies:

```rust
// Before: use crate::ai::hitl::{ApprovalPattern, RiskLevel};
// After: Move RiskLevel and ApprovalPattern to this file (or create types.rs)

use serde::{Deserialize, Serialize};

/// Risk level for tool operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Approval pattern for HITL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalPattern {
    pub total_approvals: u32,
    pub total_denials: u32,
    pub consecutive_approvals: u32,
}

// ... rest of AiEvent enum
```

---

### Step 4: Move Runtime Trait

```rust
// File: backend/crates/qbit-core/src/runtime/mod.rs

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::events::RuntimeEvent;

/// The core runtime trait that must be implemented by platform-specific runtimes.
///
/// This trait allows the core logic to communicate with either:
/// - Tauri (GUI): Events emitted via Tauri's event system
/// - CLI: Events printed to terminal, approvals via stdin
#[async_trait]
pub trait QbitRuntime: Send + Sync + 'static {
    /// Emit a runtime event (e.g., terminal output, tool execution).
    async fn emit_event(&self, event: RuntimeEvent) -> anyhow::Result<()>;

    /// Request user approval for a tool execution (HITL).
    async fn request_approval(
        &self,
        request: ApprovalRequest,
    ) -> anyhow::Result<ApprovalResult>;
}

/// Request for user approval (Human-in-the-Loop).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub risk_level: super::events::RiskLevel,
}

/// Result of an approval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalResult {
    Approved,
    Denied,
    ApprovedWithLearn,
}

/// Runtime-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Event emission failed: {0}")]
    EmitFailed(String),

    #[error("Approval request failed: {0}")]
    ApprovalFailed(String),

    #[error("Runtime not initialized")]
    NotInitialized,
}
```

---

### Step 5: Move Session Types

```bash
# Copy session files
cp backend/src/session/archive.rs backend/crates/qbit-core/src/session/
cp backend/src/session/message.rs backend/crates/qbit-core/src/session/
cp backend/src/session/listing.rs backend/crates/qbit-core/src/session/

# Create session/mod.rs
cat > backend/crates/qbit-core/src/session/mod.rs <<'EOF'
mod archive;
mod message;
mod listing;

pub use archive::{SessionArchive, SessionArchiveMetadata, SessionSnapshot};
pub use message::{MessageRole, SessionMessage, MessageContent};
pub use listing::SessionListing;

// Re-export session utilities
pub use archive::{get_sessions_dir, find_session_by_identifier, list_recent_sessions};
EOF
```

**Note**: Remove any dependencies on non-core modules. For example:

```rust
// Before: use crate::ai::events::AiEvent;
// After: (nothing - SessionArchive doesn't need AiEvent in core)
```

---

### Step 6: Create Error Type

```rust
// File: backend/crates/qbit-core/src/error.rs

use thiserror::Error;

/// Core errors that can occur in qbit-core types.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

---

### Step 7: Add to Workspace

Edit `backend/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/qbit",  # Main crate (will be moved)
    "crates/qbit-core",  # NEW
    "crates/rig-anthropic-vertex",
]

[workspace.package]
version = "0.2.0"
edition = "2021"

[workspace.dependencies]
# Core dependencies
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1.0"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }

# Dev dependencies
proptest = "1.4"
tempfile = "3"

# Local crates
qbit-core = { path = "crates/qbit-core" }
rig-anthropic-vertex = { path = "crates/rig-anthropic-vertex" }
```

---

### Step 8: Update Main Crate Imports

```bash
# Find all imports to update
rg "crate::ai::events::" --files-with-matches

# Update imports
rg "crate::ai::events::" --files-with-matches | \
  xargs sed -i 's/crate::ai::events::/qbit_core::events::/g'

# Similarly for runtime and session
rg "crate::runtime::" --files-with-matches | \
  xargs sed -i 's/crate::runtime::/qbit_core::runtime::/g'

rg "crate::session::" --files-with-matches | \
  xargs sed -i 's/crate::session::/qbit_core::session::/g'
```

**Update main crate's Cargo.toml**:

```toml
# backend/Cargo.toml (or backend/crates/qbit/Cargo.toml if moved)

[dependencies]
qbit-core = { workspace = true }

# ... other deps
```

---

## Day 3: Testing & Validation

### Step 1: Isolated Compilation

```bash
cd backend/crates/qbit-core

# Clean build
cargo clean
cargo build

# Expected output:
#   Compiling qbit-core v0.2.0
#   Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

✅ **Pass**: Crate compiles in isolation

---

### Step 2: Run Interface Tests

```bash
cd ../../../  # Back to project root
cargo test test_qbit_core_interface
```

Expected:
```
running 3 tests
test test_qbit_core_event_types_exist ... ok
test test_qbit_core_runtime_trait_exists ... ok
test test_qbit_core_session_types_exist ... ok
```

✅ **Pass**: Interface tests now pass

---

### Step 3: Check Serialization

```bash
cargo test test_ai_event_serialization_baseline
cargo insta review
```

Expected:
```
Snapshot Summary:
  Reviewed: 4 snapshots
  Accepted: 4 snapshots
  Changes: 0 snapshots
```

✅ **Pass**: Serialization format unchanged

---

### Step 4: Run Property Tests

```bash
cargo test ai_event_roundtrip
cargo test message_role_exhaustive
```

Expected:
```
test ai_event_roundtrip ... ok
test message_role_exhaustive ... ok
```

✅ **Pass**: Properties hold

---

### Step 5: Verify Dependencies

```bash
cd backend/crates/qbit-core
cargo tree --depth 1
```

Expected output:
```
qbit-core v0.2.0
├── async-trait v0.1.77
├── chrono v0.4.33
├── serde v1.0.195
├── serde_json v1.0.111
├── thiserror v1.0.56
└── uuid v1.7.0
```

✅ **Pass**: Only external dependencies, no internal deps

---

### Step 6: Test Feature Combinations

```bash
# No features (should work - qbit-core has no features)
cargo test --no-default-features

# All features
cargo test --all-features
```

✅ **Pass**: Works with all feature combinations

---

### Step 7: Coverage

```bash
cargo tarpaulin --out Stdout
```

Expected:
```
Coverage: 78.5% (target: ≥75%)
```

✅ **Pass**: Coverage exceeds 75%

---

### Step 8: API Snapshot

```bash
cargo install cargo-public-api  # If not already installed
cargo public-api > ../../tests/snapshots/qbit-core-api.txt
```

Review the API:

```
pub mod qbit_core
pub mod qbit_core::error
pub enum qbit_core::error::CoreError
pub qbit_core::error::CoreError::InvalidSessionId
pub qbit_core::error::CoreError::IoError
pub qbit_core::error::CoreError::SerializationError
pub qbit_core::error::CoreError::SessionNotFound
pub mod qbit_core::events
pub enum qbit_core::events::AiEvent
pub qbit_core::events::AiEvent::Completed
pub qbit_core::events::AiEvent::Error
pub qbit_core::events::AiEvent::Started
...
```

✅ **Pass**: API is clean and documented

---

### Step 9: Integration Tests

```bash
cd ../../../
cargo test --workspace
```

Expected:
```
running 127 tests
test ai::tests::... ok
test compat::tests::... ok
test integration::... ok
...
test result: ok. 127 passed; 0 failed; 0 ignored
```

✅ **Pass**: All workspace tests pass

---

### Step 10: Benchmarks

```bash
# Run benchmarks to ensure no regression
cargo bench --bench event_serialization -- --baseline pre-refactor
```

Expected:
```
test serialize_ai_event ... bench:   1,234 ns/iter (+/- 45)  [baseline: 1,198 ns/iter]
Difference: +3.0% (within 5% tolerance)
```

✅ **Pass**: Performance within acceptable range

---

### Step 11: Full Validation

```bash
./scripts/validate-refactor-step.sh
```

Expected:
```
=== Validation Suite ===
→ Building workspace...
   Compiled qbit-core v0.2.0
   Compiled qbit v0.1.0
   Finished dev [unoptimized + debuginfo] target(s) in 12.3s
✓ Build successful

→ Running tests...
   Running tests for qbit-core (3 tests)
   Running tests for qbit (124 tests)
✓ All tests passed

→ Checking format...
✓ Format check passed

→ Running clippy...
✓ Clippy passed (0 warnings)

→ Testing feature combinations...
✓ CLI build passed
✓ Tauri build passed
✓ local-tools build passed

→ Integration tests...
✓ compat_layer tests passed

→ Coverage check...
Coverage: 72.3% (≥70% required)
✓ Coverage passed

→ Performance check...
✓ No regressions detected

✓ Validation complete
```

✅ **All checks passed!**

---

## Commit & Document

### Commit

```bash
git add .
git commit -m "feat(core): extract qbit-core crate

- Extract event types (AiEvent, RuntimeEvent, SidecarEvent)
- Extract runtime trait (QbitRuntime)
- Extract session types (SessionArchive, SessionMessage)
- Zero internal dependencies (only serde, chrono, etc.)
- Coverage: 78.5%
- All tests passing (127 tests)
- No clippy warnings
- API snapshot captured

Breaking changes: None (backward compatible via re-exports)

Refs: docs/crate-refactoring-plan.md Phase 1"

git tag refactor-step-1-qbit-core
```

### Update Documentation

```bash
# Update refactoring plan
cat >> docs/crate-refactoring-plan.md <<EOF

## Progress Update ($(date +%Y-%m-%d))

### Completed
- [x] qbit-core extracted
  - Coverage: 78.5%
  - Tests: 127 passing
  - Dependencies: 6 external, 0 internal
  - Status: ✅ Complete

### Next Steps
- [ ] qbit-settings extraction
- [ ] qbit-runtime extraction
EOF
```

---

## Troubleshooting

### Issue: Circular Dependency

**Symptom**:
```
error: cyclic package dependency: qbit-core v0.2.0 -> qbit v0.1.0 -> qbit-core v0.2.0
```

**Cause**: qbit-core imported something from main crate

**Solution**:
```bash
# Find the offending import
rg "use qbit::" backend/crates/qbit-core/src/

# Remove or move the dependency
# qbit-core should NEVER depend on qbit
```

---

### Issue: Test Failures

**Symptom**:
```
test test_ai_event_serialization_baseline ... FAILED
```

**Cause**: Serialization format changed

**Solution**:
```bash
# Review the snapshot diff
cargo insta review

# If change is intentional, accept it
cargo insta accept

# If change is unintentional, fix the code
```

---

### Issue: Coverage Drops

**Symptom**:
```
Coverage: 58.2% (target: ≥70%)
```

**Cause**: Moved code without moving tests

**Solution**:
```bash
# Find tests for the moved code
rg "#\[test\]" backend/src/ai/events.rs

# Move tests to qbit-core
# OR write new tests in qbit-core
```

---

### Issue: Import Errors

**Symptom**:
```
error[E0433]: failed to resolve: use of undeclared crate or module `qbit_core`
```

**Cause**: Forgot to add qbit-core to main crate's dependencies

**Solution**:
```toml
# backend/Cargo.toml (or backend/crates/qbit/Cargo.toml)

[dependencies]
qbit-core = { workspace = true }
```

---

## Checklist Summary

### Pre-Extraction (Day 1)
- [x] Interface tests written
- [x] Characterization tests written
- [x] Contract tests written
- [x] Property tests written
- [x] Baselines captured

### Extraction (Day 2)
- [x] Crate structure created
- [x] lib.rs created with exports
- [x] Events moved
- [x] Runtime trait moved
- [x] Session types moved
- [x] Errors created
- [x] Added to workspace
- [x] Imports updated

### Validation (Day 3)
- [x] Compiles in isolation
- [x] Interface tests pass
- [x] Characterization tests pass
- [x] Property tests pass
- [x] Zero internal dependencies
- [x] Feature combinations work
- [x] Coverage ≥75%
- [x] API snapshot captured
- [x] Integration tests pass
- [x] Benchmarks within 5%
- [x] Full validation script passes

### Documentation
- [x] Committed with detailed message
- [x] Tagged as refactor-step-1
- [x] Progress updated in refactoring plan

---

## Metrics

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| Coverage | 78.5% | ≥75% | ✅ Pass |
| Tests | 127 | All pass | ✅ Pass |
| Warnings | 0 | 0 | ✅ Pass |
| Dependencies (external) | 6 | <10 | ✅ Pass |
| Dependencies (internal) | 0 | 0 | ✅ Pass |
| Build time (incremental) | 12.3s | <30s | ✅ Pass |
| Benchmarks | +3.0% | <5% | ✅ Pass |

---

## Next: qbit-settings

With qbit-core complete, the next extraction is **qbit-settings** (estimated: 1-2 days).

Follow the same TDD workflow:
1. Write interface tests
2. Write characterization tests
3. Extract crate
4. Validate

See: [refactor-checklist.md](./refactor-checklist.md) for the template.

---

**Status**: ✅ Complete
**Time**: 2.5 days (as estimated)
**Difficulty**: Medium
**Lessons Learned**:
- Writing tests first saved ~4 hours of debugging
- Characterization tests caught 2 serialization issues
- Property tests found 1 edge case in MessageRole
- Full validation script prevented 1 regression

