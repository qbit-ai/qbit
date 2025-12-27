# Qbit Backend Crate Refactoring Plan

**Created**: 2025-12-26
**Status**: Draft
**Goal**: Logically separate the monolithic backend into smaller, modular, testable crates

---

## Executive Summary

The Qbit backend currently consists of **~42,843 lines of Rust code** across **128 files** organized into 8 major modules. This plan proposes a phased extraction of **9 new local crates** to improve modularity, testability, build times, and code reusability.

### Current State
- Single package with 1 local crate (`rig-anthropic-vertex`)
- All code in `backend/src/` (monolithic)
- Feature flags for GUI/CLI/Evals modes
- In-progress migration from external `vtcode-core` to local implementations

### Target State
- Workspace with 10 local crates
- Clear dependency hierarchy
- Improved compile times (parallel compilation)
- Reusable components for external projects

### Key Metrics

| Metric | Before | After |
|--------|--------|-------|
| Local crates | 1 | 10 |
| Lines in main crate | ~42,843 | ~25,000 |
| Reusable crates | 0 | 5 |
| Average crate size | N/A | ~2,500 lines |

---

## Analysis Summary

### Module Breakdown

| Module | Files | LOC | Complexity | Extractability | Priority |
|--------|-------|-----|------------|----------------|----------|
| **AI** | 45 | ~15,000 | Very High | Medium | Phase 2-3 |
| **PTY** | 4 | ~5,000 | Medium | High | Phase 2 |
| **Indexer** | 4 | ~1,400 | Low | **Very High** | **Phase 1** |
| **Sidecar** | 11 | ~12,000 | High | Medium | Phase 3 |
| **Settings** | 4 | ~1,500 | Low | **Very High** | **Phase 1** |
| **Tools** | 9 | ~3,500 | Medium | **Very High** | **Phase 1** |
| **Session** | 5 | ~2,000 | Low | **Very High** | **Phase 1** |
| **Runtime** | 3 | ~800 | Low | **Very High** | **Phase 1** |
| **Commands** | 6 | ~2,900 | Low | Low | Stay in main |

### Dependency Graph

```
┌──────────────┐
│  qbit-core   │ ← Events, Runtime trait, Session types
└──────┬───────┘
       │
       ├─→ ┌────────────────┐
       │   │ qbit-runtime   │ ← TauriRuntime, CliRuntime
       │   └────────────────┘
       │
       ├─→ ┌────────────────┐
       │   │ qbit-settings  │ ← TOML config management
       │   └────────────────┘
       │
       ├─→ ┌────────────────┐
       │   │ qbit-tools     │ ← Tool system (file ops, shell, diff)
       │   └────────────────┘
       │
       ├─→ ┌────────────────┐
       │   │ qbit-pty       │ ← PTY management, parser, shell detection
       │   └────────────────┘
       │
       └─→ ┌────────────────┐
           │ qbit-indexer   │ ← Code indexing, analysis
           └────────────────┘
                 │
                 ▼
           ┌────────────────┐
           │  qbit (main)   │ ← AI, Sidecar, Commands, CLI
           └────────────────┘
```

---

## Proposed Crate Structure

### Phase 1: Foundation Crates (Week 1-2)

#### 1. `qbit-core` ⭐ **Critical Path**
**Purpose**: Shared types and traits with zero business logic dependencies

```
backend/crates/qbit-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── events/
    │   ├── mod.rs
    │   ├── ai.rs          # AiEvent (30+ variants)
    │   ├── runtime.rs     # RuntimeEvent
    │   └── sidecar.rs     # SidecarEvent
    ├── runtime/
    │   ├── mod.rs
    │   ├── trait.rs       # QbitRuntime trait
    │   └── types.rs       # ApprovalResult, RuntimeError
    ├── session/
    │   ├── mod.rs
    │   ├── archive.rs     # SessionArchive
    │   ├── message.rs     # SessionMessage, MessageRole
    │   └── listing.rs     # SessionListing
    └── error.rs           # Core error types
```

**Dependencies**:
- `serde`, `serde_json` (serialization)
- `thiserror` (error derives)
- `async-trait` (QbitRuntime trait)
- `chrono` (timestamps)

**Used by**: All crates

**Extraction effort**: Medium (2-3 days)
**Risk**: Low (pure types, no logic)

---

#### 2. `qbit-settings`
**Purpose**: TOML-based configuration with environment variable interpolation

```
backend/crates/qbit-settings/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── schema.rs          # QbitSettings struct (817 lines)
    ├── loader.rs          # SettingsManager (407 lines)
    ├── commands.rs        # Tauri commands (feature-gated)
    └── template.toml      # Default template
```

**Dependencies**:
- `qbit-core` (for error types)
- `serde`, `toml`
- `tokio` (RwLock)
- `dirs` (home directory)
- `tauri` (optional, for commands)

**Public API**:
```rust
pub struct SettingsManager { ... }
impl SettingsManager {
    pub async fn new() -> Result<Self>;
    pub async fn get(&self) -> QbitSettings;
    pub async fn update(&self, settings: QbitSettings) -> Result<()>;
    pub fn get_with_env_fallback<T>(...) -> Option<T>;
}
```

**Extraction effort**: Low (1-2 days)
**Risk**: Very Low (already isolated)

---

#### 3. `qbit-runtime`
**Purpose**: Platform-specific runtime implementations

```
backend/crates/qbit-runtime/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── tauri.rs           # TauriRuntime (feature: tauri)
    └── cli.rs             # CliRuntime (feature: cli)
```

**Feature flags**:
```toml
[features]
default = []
tauri = ["dep:tauri"]
cli = []

[[test]]
name = "exclusivity"
required-features = [] # Compile-time check for mutual exclusion
```

**Dependencies**:
- `qbit-core` (QbitRuntime trait)
- `tauri` (optional)
- `tokio` (channels)
- `parking_lot` (RwLock)

**Extraction effort**: Low (1 day)
**Risk**: Low (clean trait boundary)

---

#### 4. `qbit-tools`
**Purpose**: AI tool execution system (drop-in replacement for vtcode-core)

```
backend/crates/qbit-tools/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── registry.rs        # ToolRegistry
    ├── traits.rs          # Tool trait
    ├── error.rs           # ToolError
    ├── definitions.rs     # build_function_declarations
    ├── executors/
    │   ├── mod.rs
    │   ├── file_ops.rs    # read, write, create, edit, delete
    │   ├── directory_ops.rs # list, grep
    │   └── shell.rs       # run_pty_cmd
    ├── udiff/
    │   ├── mod.rs
    │   ├── parser.rs      # Unified diff parsing
    │   ├── applier.rs     # Patch application
    │   └── error.rs       # PatchError
    └── planner/
        └── mod.rs         # Task planning
```

**Interface Contract** (MUST preserve for vtcode-core compatibility):
```rust
pub struct ToolRegistry { ... }
impl ToolRegistry {
    pub async fn new(workspace: PathBuf) -> Result<Self>;
    pub async fn execute_tool(&self, name: &str, args: &str) -> Result<String>;
    pub fn available_tools(&self) -> Vec<String>;
}
```

**Dependencies**:
- `qbit-core` (error types)
- `serde`, `serde_json`
- `async-trait`
- `tokio` (async execution)

**Extraction effort**: Medium (3-4 days)
**Risk**: Medium (interface must match vtcode-core exactly)

---

### Phase 2: Domain Crates (Week 3-4)

#### 5. `qbit-pty`
**Purpose**: PTY session management with terminal parsing

```
backend/crates/qbit-pty/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── manager.rs         # PtyManager, session lifecycle
    ├── parser.rs          # VTE/OSC sequence parser (self-contained)
    └── shell.rs           # Shell detection
```

**Public API**:
```rust
pub struct PtyManager { ... }
impl PtyManager {
    pub fn create_session_with_runtime(...) -> Result<PtySession>;
    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<()>;
    pub fn resize(&self, session_id: &str, rows: u16, cols: u16) -> Result<()>;
    pub fn destroy(&self, session_id: &str) -> Result<()>;
}

pub struct TerminalParser { ... }
pub enum OscEvent { /* 9 variants */ }
```

**Dependencies**:
- `qbit-core` (RuntimeEvent, QbitRuntime trait)
- `portable-pty` (PTY creation)
- `vte` (escape sequence parsing)
- `uuid` (session IDs)
- `parking_lot` (Mutex)

**Extraction effort**: Medium (3-4 days)
**Risk**: Medium (runtime integration)

---

#### 6. `qbit-indexer`
**Purpose**: Code indexing and tree-sitter analysis

```
backend/crates/qbit-indexer/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── state.rs           # IndexerState
    ├── paths.rs           # Index location resolution
    └── commands.rs        # Tauri commands (feature-gated)
```

**Dependencies**:
- `qbit-core` (error types)
- `qbit-settings` (IndexLocation enum)
- `vtcode-indexer` (SimpleIndexer)
- `vtcode-core` (TreeSitterAnalyzer)
- `parking_lot` (RwLock)
- `tauri` (optional, for commands)

**Extraction effort**: Low (2-3 days)
**Risk**: Low (already well-isolated)

---

### Phase 3: Advanced Components (Week 5-6)

#### 7. `qbit-sidecar-core`
**Purpose**: Session file I/O and event definitions (extract reusable parts)

```
backend/crates/qbit-sidecar-core/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── session.rs         # Session file I/O (567 lines, zero deps)
    ├── events.rs          # SessionEvent definitions (1,787 lines)
    └── synthesis.rs       # LLM synthesis abstractions (2,212 lines)
```

**Note**: Full sidecar module (11 files, ~12K LOC) is too interconnected to extract as one unit. This extracts the **reusable components**.

**Dependencies**:
- `qbit-core` (event types)
- `qbit-settings` (synthesis config)
- `serde`, `toml`, `chrono`
- `tokio` (async fs)
- `reqwest` (LLM APIs)
- `gcp_auth` (Vertex AI)

**Extraction effort**: Medium (3-5 days)
**Risk**: Medium (synthesis has LLM dependencies)

---

#### 8. `qbit-shell-integration`
**Purpose**: Shell integration installer (reusable for other terminal apps)

```
backend/crates/qbit-shell-integration/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── installer.rs       # Install/uninstall logic
    ├── scripts/
    │   ├── zsh.sh         # Embedded integration script
    │   ├── bash.sh
    │   └── fish.sh
    └── types.rs           # IntegrationStatus, ShellType
```

**Generic API** (works for any app):
```rust
pub struct ShellIntegration {
    app_name: String,
    config_dir: PathBuf,
}

impl ShellIntegration {
    pub fn new(app_name: &str) -> Self;
    pub fn status(&self, shell: ShellType) -> IntegrationStatus;
    pub fn install(&self, shell: ShellType) -> Result<()>;
    pub fn uninstall(&self, shell: ShellType) -> Result<()>;
}
```

**Dependencies**:
- `dirs` (config directories)
- `std` only

**Extraction effort**: Medium (2-3 days)
**Risk**: Low (well-tested, 900+ lines of tests)

---

#### 9. `qbit-context-manager`
**Purpose**: LLM context window management (token budgeting, pruning)

```
backend/crates/qbit-context-manager/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── manager.rs         # ContextManager orchestrator
    ├── pruner.rs          # Message pruning logic
    ├── budget.rs          # Token budget tracking
    └── truncate.rs        # Tool response truncation
```

**Dependencies**:
- `qbit-core` (events for emission)
- `rig-core` (Message type)
- `serde`, `serde_json`

**Extraction effort**: Low (2-3 days)
**Risk**: Low (self-contained algorithms)

---

### Main Crate: `qbit`

**Remains in `backend/src/`**:
- `ai/` - Agent orchestration (AgentBridge, agentic_loop, workflows)
- `sidecar/` (minus extracted parts) - Capture, processor, commits, artifacts
- `commands/` - Tauri command handlers
- `cli/` - CLI-specific code
- `evals/` - Evaluation framework
- `tavily/` - Web search integration
- `web_fetch.rs` - Web fetcher
- `compat.rs` - Migration compatibility layer
- `lib.rs`, `main.rs`, `bin/qbit-cli.rs` - Entry points

**Estimated size**: ~25,000 lines (reduced from ~42,843)

---

## Workspace Configuration

### Root `Cargo.toml` (Workspace Manifest)

```toml
[workspace]
resolver = "2"
members = [
    "crates/qbit",
    "crates/qbit-core",
    "crates/qbit-runtime",
    "crates/qbit-settings",
    "crates/qbit-tools",
    "crates/qbit-pty",
    "crates/qbit-indexer",
    "crates/qbit-sidecar-core",
    "crates/qbit-shell-integration",
    "crates/qbit-context-manager",
    "crates/rig-anthropic-vertex",
]

[workspace.package]
version = "0.2.0"
edition = "2021"
license = "MIT OR Apache-2.0"
authors = ["Qbit Team"]

[workspace.dependencies]
# Core dependencies (shared versions)
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1.0"
thiserror = "1.0"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
parking_lot = "0.12"
dirs = "5"

# Tauri (optional)
tauri = { version = "2", features = ["devtools"] }

# Local crates
qbit-core = { path = "crates/qbit-core" }
qbit-runtime = { path = "crates/qbit-runtime" }
qbit-settings = { path = "crates/qbit-settings" }
qbit-tools = { path = "crates/qbit-tools" }
qbit-pty = { path = "crates/qbit-pty" }
qbit-indexer = { path = "crates/qbit-indexer" }
qbit-sidecar-core = { path = "crates/qbit-sidecar-core" }
qbit-shell-integration = { path = "crates/qbit-shell-integration" }
qbit-context-manager = { path = "crates/qbit-context-manager" }
rig-anthropic-vertex = { path = "crates/rig-anthropic-vertex" }
```

### Main Crate `crates/qbit/Cargo.toml`

```toml
[package]
name = "qbit"
version.workspace = true
edition.workspace = true

[features]
default = ["tauri", "local-tools"]
tauri = [
    "dep:tauri",
    "qbit-runtime/tauri",
    "qbit-settings/tauri",
    "qbit-indexer/tauri",
    "qbit-sidecar-core/tauri",
]
cli = ["qbit-runtime/cli"]
local-tools = []  # Use qbit-tools instead of vtcode-core
evals = ["cli"]

[dependencies]
# Workspace crates
qbit-core.workspace = true
qbit-runtime.workspace = true
qbit-settings.workspace = true
qbit-tools.workspace = true
qbit-pty.workspace = true
qbit-indexer.workspace = true
qbit-sidecar-core.workspace = true
qbit-shell-integration.workspace = true
qbit-context-manager.workspace = true
rig-anthropic-vertex.workspace = true

# Shared dependencies
tokio.workspace = true
serde.workspace = true
anyhow.workspace = true
# ... etc

# Tauri (optional)
tauri = { workspace = true, optional = true }

# External crates (to be removed)
vtcode-core = { version = "0.5", optional = true }
vtcode-indexer = "0.3"
```

---

## Migration Strategy

### Phase 1: Foundation (Week 1-2)

#### Step 1.1: Convert to Workspace
```bash
# 1. Create workspace structure
mkdir -p backend/crates/qbit
mv backend/src backend/crates/qbit/
mv backend/Cargo.toml backend/crates/qbit/

# 2. Create workspace Cargo.toml at backend/
cat > backend/Cargo.toml <<EOF
[workspace]
resolver = "2"
members = ["crates/qbit", "crates/rig-anthropic-vertex"]
[workspace.dependencies]
# ... shared deps
EOF

# 3. Update paths in qbit/Cargo.toml
# Change rig-anthropic-vertex path: "../rig-anthropic-vertex"

# 4. Test
cd backend && cargo build
```

**Validation**: All tests pass, both GUI and CLI build successfully.

---

#### Step 1.2: Extract `qbit-core`

**Script**: `scripts/extract-qbit-core.sh`
```bash
#!/bin/bash
set -e

echo "Extracting qbit-core crate..."

# 1. Create crate structure
mkdir -p backend/crates/qbit-core/src/{events,runtime,session}

# 2. Copy files
cp backend/crates/qbit/src/ai/events.rs backend/crates/qbit-core/src/events/ai.rs
cp backend/crates/qbit/src/runtime/mod.rs backend/crates/qbit-core/src/runtime/
# ... etc

# 3. Create Cargo.toml
cat > backend/crates/qbit-core/Cargo.toml <<EOF
[package]
name = "qbit-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }
EOF

# 4. Create lib.rs
cat > backend/crates/qbit-core/src/lib.rs <<EOF
pub mod events;
pub mod runtime;
pub mod session;
pub mod error;

pub use events::{AiEvent, RuntimeEvent, SidecarEvent};
pub use runtime::{QbitRuntime, ApprovalResult, RuntimeError};
pub use session::{SessionArchive, SessionMessage};
EOF

# 5. Update imports in main crate
find backend/crates/qbit/src -name "*.rs" -exec sed -i \
  's/crate::ai::events::/qbit_core::events::ai::/g' {} \;
find backend/crates/qbit/src -name "*.rs" -exec sed -i \
  's/crate::runtime::/qbit_core::runtime::/g' {} \;

# 6. Add to workspace members
sed -i '/members = \[/a \    "crates/qbit-core",' backend/Cargo.toml

# 7. Test
cargo build -p qbit-core
cargo test -p qbit-core

echo "✓ qbit-core extracted successfully"
```

**Validation**:
```bash
cargo build --all
cargo test --all
cargo clippy --all
```

---

#### Step 1.3: Extract `qbit-settings`

Similar script-based extraction. Key steps:
1. Move `backend/crates/qbit/src/settings/` → `backend/crates/qbit-settings/src/`
2. Update `Cargo.toml` dependencies
3. Feature-gate Tauri commands
4. Update imports: `crate::settings::` → `qbit_settings::`

**Validation**: Settings load correctly, commands work in GUI mode.

---

#### Step 1.4: Extract `qbit-runtime`

1. Move `runtime/tauri.rs` and `runtime/cli.rs`
2. Keep trait definition in `qbit-core`
3. Feature-gate implementations
4. Test both Tauri and CLI builds

---

#### Step 1.5: Extract `qbit-tools`

**Critical**: Update `compat.rs` to use local crate:
```rust
#[cfg(feature = "local-tools")]
pub mod tools {
    pub use qbit_tools::{ToolRegistry, build_function_declarations};
}

#[cfg(not(feature = "local-tools"))]
pub mod tools {
    pub use vtcode_core::tools::ToolRegistry;
}
```

**Validation**:
```bash
# Test with local-tools (default)
cargo test --features local-tools

# Test with vtcode-core (fallback)
cargo test --no-default-features --features tauri
```

---

### Phase 2: Domain Crates (Week 3-4)

Extract in order:
1. `qbit-pty` (requires `qbit-core` for runtime events)
2. `qbit-indexer` (requires `qbit-settings` for config)

**Validation after each**: Full test suite, both build modes.

---

### Phase 3: Advanced Components (Week 5-6)

Extract in order:
1. `qbit-sidecar-core` (session, events, synthesis)
2. `qbit-shell-integration` (standalone, no deps)
3. `qbit-context-manager` (AI-specific, requires `qbit-core`)

**Final validation**:
```bash
# All crates
cargo build --workspace
cargo test --workspace
cargo clippy --workspace

# GUI mode
cargo build -p qbit --features tauri
just dev

# CLI mode
cargo build -p qbit --features cli --no-default-features --bin qbit-cli
./target/debug/qbit-cli --help

# Evals
cargo run -p qbit --features evals,cli --no-default-features --bin qbit-cli -- --eval
```

---

## Benefits Analysis

### 1. **Build Time Improvements**

**Before** (single crate):
- Full rebuild: ~3-5 minutes
- Incremental: ~30-60 seconds
- Parallel compilation limited by file-level dependencies

**After** (workspace):
- Full rebuild: ~2-3 minutes (parallel crate compilation)
- Incremental: ~10-20 seconds (only changed crate + dependents)
- Example: Changing `qbit-pty` doesn't rebuild AI module

**Estimated savings**: 40-50% on incremental builds

---

### 2. **Modularity & Testability**

| Crate | Test in Isolation | Mock Dependencies | Property Tests |
|-------|-------------------|-------------------|----------------|
| `qbit-core` | ✅ | N/A (no deps) | ✅ Events |
| `qbit-tools` | ✅ | ✅ ToolRegistry | ✅ Diff parsing |
| `qbit-pty` | ✅ | ✅ QbitRuntime | ✅ Parser |
| `qbit-settings` | ✅ | N/A | ✅ TOML parsing |

**New test capabilities**:
- Integration tests per crate in `crates/*/tests/`
- Benchmark tests in `crates/*/benches/`
- Isolated fuzzing for parsers

---

### 3. **Reusability**

Crates ready for external use:

| Crate | Use Case | Publish to crates.io? |
|-------|----------|----------------------|
| `qbit-tools` | AI agent tool systems | Maybe (after stabilization) |
| `qbit-pty` | Terminal applications | Yes (generic PTY manager) |
| `qbit-shell-integration` | Any terminal app | **Yes** (highly reusable) |
| `qbit-settings` | TOML config apps | Maybe |
| `qbit-context-manager` | LLM applications | Yes (useful for AI apps) |

---

### 4. **Code Organization**

**Before**: Flat module structure, unclear boundaries
```
backend/src/
├── ai/ (45 files, mixed responsibilities)
├── commands/ (6 unrelated command groups)
└── ... (everything in one crate)
```

**After**: Clear hierarchy
```
backend/crates/
├── qbit-core/ (shared contracts)
├── qbit-tools/ (tool execution)
├── qbit-pty/ (terminal management)
└── qbit/ (application logic)
```

---

### 5. **Dependency Management**

**Before**: All dependencies in one `Cargo.toml` (100+ lines)

**After**: Minimal dependencies per crate
- `qbit-core`: 5 deps (serde, chrono, thiserror, async-trait, uuid)
- `qbit-pty`: 7 deps (qbit-core + portable-pty, vte, parking_lot, uuid)
- `qbit-tools`: 6 deps (qbit-core + serde, async-trait, tokio)

**Benefit**: Easier to audit, update, and secure individual crates

---

## Risk Mitigation

### Risk 1: Breaking Changes During Migration

**Mitigation**:
- Use feature flags to maintain old behavior
- Keep `compat.rs` layer during transition
- Comprehensive test suite for each extraction
- Git branches for each phase (rollback if needed)

**Rollback plan**: Each phase is independently committable

---

### Risk 2: Performance Regression

**Mitigation**:
- Benchmark before/after for critical paths
- Use `#[inline]` for hot paths across crate boundaries
- Profile with `cargo flamegraph`
- LTO (Link-Time Optimization) for release builds:
  ```toml
  [profile.release]
  lto = "thin"
  codegen-units = 1
  ```

**Monitoring**: Track build times and runtime performance in CI

---

### Risk 3: Circular Dependencies

**Current state**: Zero circular dependencies (verified by analysis)

**Prevention**:
- Strict dependency hierarchy (enforced by Cargo)
- `qbit-core` has no internal deps (foundation layer)
- Regular `cargo tree` audits
- Deny list in `.cargo/config.toml`:
  ```toml
  [lints.rust]
  unused_crate_dependencies = "warn"
  ```

---

### Risk 4: Workspace Overhead

**Concern**: Workspace adds complexity

**Mitigation**:
- Start with 4-5 crates, expand gradually
- Document workspace in CONTRIBUTING.md
- Use `cargo-workspaces` tool for release management
- Shared `workspace.dependencies` reduces duplication

**Alternative**: If overhead is too high, can merge back (Cargo supports this)

---

## Success Metrics

### Technical Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Build time (incremental) | <30s | `cargo build --timings` |
| Test time (workspace) | <2min | `cargo test --workspace` |
| Crate count | 10 | `ls crates/ \| wc -l` |
| Average crate size | <3000 lines | `tokei crates/*/src` |
| Test coverage | >70% | `cargo tarpaulin` |

### Code Quality Metrics

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Clippy warnings | 0 | `cargo clippy --workspace` |
| Rustfmt compliance | 100% | `cargo fmt --check` |
| Documentation | >80% public items | `cargo doc --no-deps` |
| Dependency freshness | <6 months old | `cargo outdated` |

### Developer Experience

- [ ] New contributors can understand architecture from README
- [ ] Each crate has clear purpose and examples
- [ ] CI passes on all feature flag combinations
- [ ] Documentation builds without warnings

---

## Timeline

### Phase 1: Foundation (Weeks 1-2)
- **Week 1**: Workspace setup, extract qbit-core, qbit-settings
- **Week 2**: Extract qbit-runtime, qbit-tools, enable `local-tools` by default

**Milestone**: Remove `vtcode-core` dependency entirely

---

### Phase 2: Domain Crates (Weeks 3-4)
- **Week 3**: Extract qbit-pty, qbit-indexer
- **Week 4**: Testing, documentation, CI updates

**Milestone**: All major subsystems are isolated crates

---

### Phase 3: Advanced (Weeks 5-6)
- **Week 5**: Extract qbit-sidecar-core, qbit-shell-integration
- **Week 6**: Extract qbit-context-manager, final polish

**Milestone**: Publish-ready crates for external use

---

### Phase 4: Stabilization (Week 7)
- Documentation review
- API stabilization
- Performance benchmarking
- Consider publishing to crates.io

---

## Alternative Approaches

### Option A: Big Bang (NOT Recommended)

Extract all crates at once in a single massive PR.

**Pros**: Faster initial completion
**Cons**: High risk, difficult to review, hard to rollback

**Verdict**: ❌ Too risky

---

### Option B: Minimal Extraction (Conservative)

Only extract 3 crates: `qbit-core`, `qbit-tools`, `qbit-settings`

**Pros**: Lower risk, faster
**Cons**: Misses 60% of benefits, still monolithic

**Verdict**: ⚠️ Good fallback if full plan is too ambitious

---

### Option C: Phased Extraction (Recommended)

Current plan - extract in 3 phases over 6-7 weeks.

**Pros**: Balanced risk/reward, incremental validation
**Cons**: Longer timeline

**Verdict**: ✅ **Recommended**

---

## Open Questions

1. **Publishing Strategy**: Should we publish crates to crates.io?
   - **Recommendation**: Start private, publish after 1-2 stable releases

2. **Versioning**: Workspace-level version or independent?
   - **Recommendation**: Workspace version initially, independent after 1.0

3. **Documentation**: Separate docs site or rustdoc?
   - **Recommendation**: Rustdoc + comprehensive examples in each crate

4. **CI Strategy**: Test all feature combinations?
   - **Recommendation**: Matrix strategy testing key combinations

5. **Backward Compatibility**: Maintain old imports?
   - **Recommendation**: Re-export from main crate for 1-2 releases

---

## Next Steps

### Immediate (This Week)
1. ✅ Review this plan with team
2. ⬜ Set up feature branch: `feat/workspace-refactor`
3. ⬜ Run baseline benchmarks (`cargo build --timings`, test times)
4. ⬜ Create extraction scripts for Phase 1

### Week 1
1. ⬜ Convert to workspace structure
2. ⬜ Extract `qbit-core`
3. ⬜ Update all imports, validate tests pass

### Week 2
1. ⬜ Extract `qbit-settings`
2. ⬜ Extract `qbit-runtime`
3. ⬜ Extract `qbit-tools`, enable `local-tools` by default

### Week 3+
Continue with Phase 2 and 3 extractions...

---

## Appendix

### A. File Manifest

Comprehensive list of files to move for each crate extraction:

**qbit-core**:
```
backend/crates/qbit/src/ai/events.rs → qbit-core/src/events/ai.rs
backend/crates/qbit/src/runtime/mod.rs → qbit-core/src/runtime/mod.rs
backend/crates/qbit/src/session/*.rs → qbit-core/src/session/
backend/crates/qbit/src/error.rs → qbit-core/src/error.rs
```

*(Full manifests for other crates available on request)*

---

### B. Command Reference

**Workspace commands**:
```bash
# Build all crates
cargo build --workspace

# Test specific crate
cargo test -p qbit-core

# Check dependencies
cargo tree -p qbit

# Update all deps
cargo update --workspace

# Release workflow
cargo workspaces publish --from-git
```

---

### C. Further Reading

- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [Modular Rust Projects](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html)

---

**Document Version**: 1.0
**Last Updated**: 2025-12-26
**Status**: Ready for Review
