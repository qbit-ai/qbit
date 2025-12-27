# Dependency-Driven Crate Refactoring Strategy

**Core Principle**: Use crate boundaries as **architectural enforcement** to eliminate circular dependencies and establish proper layering.

---

## Key Insight

The original plan focused on "modularity" and "code organization". But the **real value** of separate crates is:

> **Cargo prevents circular dependencies between crates. Use this as a forcing function to fix architectural problems.**

**Before (monolithic)**:
```rust
// Everything in one crate = circular dependencies allowed
ai/events.rs    ⟷  tools/registry.rs
commands/       ⟷  state.rs
settings/       ⟷  ai/agent_bridge.rs
```

**After (crates)**:
```rust
// Cargo enforces acyclic dependency graph
qbit-core (foundation)
    ↓
qbit-domain-crates (settings, tools, pty, etc.)
    ↓
qbit-application (main crate with commands, AppState)
```

---

## Current Circular Dependencies Analysis

### Cycle 1: Commands ↔ AppState ↔ Subsystem States

**The Problem**:
```rust
// backend/src/commands/settings.rs
use crate::state::AppState;  // Depends on AppState
#[tauri::command]
fn get_settings(state: State<AppState>) -> QbitSettings {
    state.settings_manager.get()
}

// backend/src/state.rs
use crate::settings::SettingsManager;  // AppState depends on subsystems
pub struct AppState {
    pub settings_manager: Arc<SettingsManager>,
    pub indexer_state: Arc<IndexerState>,
    // ...
}
```

**Why It's Circular**:
- Commands need AppState to access subsystems
- AppState needs subsystem types (SettingsManager, IndexerState, etc.)
- If we extract SettingsManager to qbit-settings crate with commands:
  - qbit-settings commands need AppState (from main crate)
  - main crate needs SettingsManager (from qbit-settings)
  - **Circular dependency!** ❌

**Crate-Based Solution**:

```
┌─────────────────────────────────────────────┐
│  qbit (main crate)                          │
│  - AppState aggregator                      │
│  - ALL Tauri commands (thin wrappers)       │
│  - Application composition                  │
└─────────────────┬───────────────────────────┘
                  │ depends on
                  ↓
┌─────────────────────────────────────────────┐
│  Domain Crates (qbit-settings, etc.)        │
│  - SettingsManager (business logic)         │
│  - NO Tauri dependencies                    │
│  - NO knowledge of AppState                 │
└─────────────────────────────────────────────┘
```

**Result**: ✅ No circular dependency - main crate depends on domain crates, never the reverse.

---

### Cycle 2: Events ↔ Tools Types

**The Problem**:
```rust
// backend/src/ai/events.rs
use crate::tools::{PlanSummary, PlanStep};  // Events depend on Tools

pub enum AiEvent {
    PlanUpdated {
        summary: PlanSummary,  // From tools module
        steps: Vec<PlanStep>,
    },
    // ...
}

// backend/src/tools/registry.rs
use crate::ai::events::AiEvent;  // Tools emit events

impl ToolRegistry {
    pub async fn execute(&self) -> Result<String> {
        emit_event(AiEvent::ToolExecutionStarted { ... });
        // ...
    }
}
```

**Why It's Circular**:
- Events need Plan types from Tools
- Tools need to emit AiEvent
- **Cycle**: Events ↔ Tools

**Crate-Based Solution**:

```
┌─────────────────────────────────────────────┐
│  qbit-core (foundation crate)               │
│  - AiEvent, RuntimeEvent (core events)      │
│  - PlanSummary, PlanStep (shared types)     │
│  - QbitRuntime trait                        │
└─────────────────┬───────────────────────────┘
                  │
                  ↓ depends on
┌─────────────────────────────────────────────┐
│  qbit-tools                                 │
│  - ToolRegistry                             │
│  - Emits events from qbit-core              │
│  - Uses Plan types from qbit-core           │
└─────────────────────────────────────────────┘
```

**Result**: ✅ No cycle - both use qbit-core, tools depends on core, events don't depend on tools.

---

### Cycle 3: Runtime ↔ Events ↔ Application

**The Problem**:
```rust
// backend/src/runtime/tauri.rs
use crate::ai::events::AiEvent;  // Runtime needs events

// backend/src/ai/agentic_loop.rs
use crate::runtime::QbitRuntime;  // Agent needs runtime

impl AgentBridge {
    fn new(runtime: Arc<dyn QbitRuntime>) {
        runtime.emit(AiEvent::Started { ... });
    }
}
```

**Not Actually Circular** (but poor layering):
- Runtime trait defined in main crate
- Events defined in main crate
- Both should be in foundation layer

**Crate-Based Solution**:

```
┌─────────────────────────────────────────────┐
│  qbit-core                                  │
│  - QbitRuntime trait (abstract)             │
│  - AiEvent, RuntimeEvent                    │
└─────────────────┬───────────────────────────┘
                  │
                  ↓
┌─────────────────────────────────────────────┐
│  qbit-runtime                               │
│  - TauriRuntime (concrete impl)             │
│  - CliRuntime (concrete impl)               │
│  - Uses events from qbit-core               │
└─────────────────────────────────────────────┘
```

**Result**: ✅ Clean layering - runtime implementations depend on core abstractions.

---

## Dependency-Driven Layering Architecture

### Layer 1: Foundation (Zero Dependencies)

**qbit-core**
- **Purpose**: Shared abstractions and types used by ALL other crates
- **Contains**:
  - Event types (AiEvent, RuntimeEvent, SidecarEvent)
  - Trait definitions (QbitRuntime, Tool)
  - Shared domain types (PlanSummary, PlanStep, ApprovalPattern, RiskLevel)
  - Session types (SessionArchive, SessionMessage)
  - Error types (core errors only)
- **Dependencies**: External only (serde, chrono, async-trait)
- **Used by**: Everything

**Key Rule**: qbit-core has ZERO internal crate dependencies. If we want to add something here, we must ensure it doesn't depend on any other qbit-* crate.

---

### Layer 2: Infrastructure (Depends on Foundation)

**qbit-runtime**
- **Purpose**: Platform-specific runtime implementations
- **Depends on**: qbit-core (for QbitRuntime trait, events)
- **Used by**: qbit (main), domain crates that emit events

**qbit-settings**
- **Purpose**: Configuration management
- **Depends on**: qbit-core (for error types)
- **Used by**: All domain crates, main crate

---

### Layer 3: Domain Logic (Depends on Foundation + Infrastructure)

**qbit-tools**
- **Purpose**: Tool execution system
- **Depends on**: qbit-core (events, Plan types)
- **Used by**: qbit (main), AI agent

**qbit-pty**
- **Purpose**: PTY session management
- **Depends on**: qbit-core (runtime trait), qbit-settings (TerminalSettings)
- **Used by**: qbit (main), qbit-tools (for run_pty_cmd)

**qbit-indexer**
- **Purpose**: Code indexing
- **Depends on**: qbit-core, qbit-settings (IndexLocation)
- **Used by**: qbit (main), qbit-tools (indexer tools)

**qbit-sidecar-core**
- **Purpose**: Session capture and synthesis
- **Depends on**: qbit-core (events), qbit-settings (synthesis config)
- **Used by**: qbit (main)

**qbit-context-manager**
- **Purpose**: LLM token management
- **Depends on**: qbit-core (events)
- **Used by**: qbit (main), AI agent

**qbit-shell-integration**
- **Purpose**: Shell integration installer
- **Depends on**: None (completely standalone)
- **Used by**: qbit (main)

---

### Layer 4: Application (Depends on Everything)

**qbit (main crate)**
- **Purpose**: Application composition and presentation layer
- **Contains**:
  - AppState (aggregates all subsystems)
  - ALL Tauri commands (thin wrappers calling domain crates)
  - AI agent orchestration (AgentBridge, agentic_loop)
  - CLI implementation
  - Main binary entry points
- **Depends on**: All other crates
- **Used by**: Nothing (top of the dependency tree)

---

## Revised Dependency Graph (Enforced by Cargo)

```
                    ┌──────────────┐
                    │  qbit-core   │ ← Layer 1: Foundation (no deps)
                    └──────┬───────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ↓                  ↓                  ↓
┌───────────────┐  ┌──────────────┐  ┌──────────────┐
│ qbit-runtime  │  │qbit-settings │  │qbit-shell-   │ ← Layer 2: Infrastructure
│               │  │              │  │integration   │
└───────┬───────┘  └──────┬───────┘  └──────────────┘
        │                 │
        │    ┌────────────┼────────────┐
        │    │            │            │
        ↓    ↓            ↓            ↓
   ┌─────────────┐  ┌──────────┐  ┌──────────────┐
   │ qbit-tools  │  │qbit-pty  │  │qbit-indexer  │ ← Layer 3: Domain
   │             │  │          │  │              │
   └─────┬───────┘  └────┬─────┘  └──────┬───────┘
         │               │                │
         │               │    ┌───────────┼──────────────┐
         │               │    │           │              │
         ↓               ↓    ↓           ↓              ↓
    ┌────────────────────────────────────────────────────────┐
    │  qbit-sidecar-core     qbit-context-manager           │ ← Layer 3: Advanced
    └─────────────────────────┬──────────────────────────────┘
                              │
                              ↓
                    ┌──────────────────┐
                    │  qbit (main)     │ ← Layer 4: Application
                    │  - AppState      │
                    │  - Commands      │
                    │  - AI Agent      │
                    │  - CLI           │
                    └──────────────────┘
```

**Key Properties**:
- ✅ **Acyclic**: No cycles possible (Cargo enforces this)
- ✅ **Layered**: Clear separation of concerns
- ✅ **Testable**: Each layer can be tested independently
- ✅ **Modular**: Changes in Layer 3 don't affect Layer 1

---

## How This Eliminates Circular Dependencies

### Example 1: Commands and AppState

**Old Problem** (if we extracted commands with crates):
```
qbit-settings (with commands) → needs AppState
qbit (main) → needs SettingsManager → from qbit-settings
CYCLE! ❌
```

**New Solution** (commands stay in main):
```
qbit-settings → NO commands, just SettingsManager
qbit (main) → has commands, uses SettingsManager from qbit-settings
NO CYCLE! ✅
```

**Why it works**: Commands are in the **application layer** (Layer 4), domain logic is in **domain layer** (Layer 3). Application depends on domain, never the reverse.

---

### Example 2: Events and Tools

**Old Problem** (if events stayed with AI module):
```
tools/ → emits AiEvent
ai/events.rs → uses PlanSummary from tools
CYCLE! ❌
```

**New Solution** (events + Plan types in qbit-core):
```
qbit-core → has AiEvent, PlanSummary (foundation)
qbit-tools → uses both from qbit-core
NO CYCLE! ✅
```

**Why it works**: Both tools and AI agent depend on the **foundation layer** (Layer 1). Foundation has no internal dependencies.

---

### Example 3: PTY and Settings

**Old Problem** (discovered in validation):
```
qbit-pty → needs TerminalSettings
qbit-settings → might need PTY info
POTENTIAL CYCLE! ⚠️
```

**New Solution** (proper dependency direction):
```
qbit-settings → just config, no PTY knowledge
qbit-pty → uses TerminalSettings from qbit-settings
NO CYCLE! ✅
```

**Why it works**: Settings is **infrastructure** (Layer 2), PTY is **domain logic** (Layer 3). Domain can depend on infrastructure.

---

## Extraction Order Based on Dependencies

### Phase 0: Preparation (2 weeks)

**Week 1: Move Shared Types to Foundation**
- [ ] Create `qbit-core` skeleton
- [ ] Move `PlanSummary`, `PlanStep` from `tools/planner/` to `qbit-core/src/plan.rs`
- [ ] Move `ApprovalPattern`, `RiskLevel` from `ai/hitl/` to `qbit-core/src/hitl.rs`
- [ ] Update imports in main crate
- [ ] **Validate**: No circular imports

**Week 2: Write Characterization Tests**
- [ ] Characterization tests for events
- [ ] Characterization tests for tool execution
- [ ] Characterization tests for PTY parsing
- [ ] Baseline benchmarks

---

### Phase 1: Foundation Layer (3 weeks)

**Week 1: Extract qbit-core**
- [ ] Move `ai/events.rs` → `qbit-core/src/events/ai.rs`
- [ ] Move `runtime/mod.rs` (trait only) → `qbit-core/src/runtime/trait.rs`
- [ ] Move `session/*` → `qbit-core/src/session/`
- [ ] **Validate**: qbit-core has zero internal crate dependencies
- [ ] **Test**: All event serialization tests pass

**Week 2-3: Extract Infrastructure Crates**
- [ ] Extract qbit-settings (NO commands)
- [ ] Extract qbit-runtime (concrete implementations)
- [ ] **Validate**: Both depend only on qbit-core
- [ ] **Test**: Both build independently

---

### Phase 2: Domain Layer (4 weeks)

**Week 4-5: Extract qbit-tools**
- [ ] Extract qbit-tools (uses Plan types from qbit-core)
- [ ] Update tool executors to emit events from qbit-core
- [ ] **Validate**: qbit-tools depends only on qbit-core
- [ ] **Test**: Tool execution tests pass

**Week 6: Extract qbit-pty**
- [ ] Extract qbit-pty (depends on qbit-core + qbit-settings)
- [ ] **Validate**: No circular dependencies
- [ ] **Test**: PTY parsing tests pass

**Week 7: Extract qbit-indexer**
- [ ] Extract qbit-indexer (depends on qbit-core + qbit-settings)
- [ ] **Validate**: No circular dependencies
- [ ] **Test**: Indexing tests pass

---

### Phase 3: Advanced Domain Layer (3 weeks)

**Week 8-9: Extract qbit-sidecar-core**
- [ ] Extract session I/O, events, synthesis
- [ ] **Validate**: Depends only on lower layers
- [ ] **Test**: Sidecar operations pass

**Week 10: Extract Remaining Crates**
- [ ] Extract qbit-shell-integration (no dependencies!)
- [ ] Extract qbit-context-manager
- [ ] **Validate**: All crates form acyclic graph
- [ ] **Test**: All tests pass

---

### Phase 4: Application Layer (2 weeks)

**Week 11: Main Crate Cleanup**
- [ ] Organize commands in `qbit/src/commands/`
- [ ] Create AppState as pure aggregator
- [ ] Update all imports to use workspace crates
- [ ] **Validate**: Main crate depends on everything, nothing depends on main
- [ ] **Test**: Full integration tests pass

**Week 12: Stabilization**
- [ ] Documentation for each crate
- [ ] Performance benchmarking
- [ ] Bug fixes

---

## How to Use Crates to Resolve Circular Dependencies

### Principle 1: Foundation Types in qbit-core

**If two modules need each other's types, extract the types to qbit-core.**

Example:
```rust
// BEFORE (circular)
ai/events.rs → uses tools::PlanSummary
tools/registry.rs → emits AiEvent

// AFTER (acyclic)
qbit-core → has both AiEvent and PlanSummary
qbit-tools → uses both from qbit-core
main crate → uses both from qbit-core
```

---

### Principle 2: Commands in Application Layer

**Commands aggregate multiple subsystems → they belong in the top layer.**

Example:
```rust
// qbit/src/commands/settings.rs (application layer)
#[tauri::command]
async fn get_settings(state: State<AppState>) -> Result<QbitSettings> {
    // Thin wrapper - just calls domain crate
    state.settings_manager.get().await
}

// qbit-settings/src/manager.rs (domain layer)
impl SettingsManager {
    pub async fn get(&self) -> QbitSettings {
        // Business logic here
    }
}
```

**Why**: Commands need AppState (aggregator). AppState needs domain crates. Commands must be above domain in the layer hierarchy.

---

### Principle 3: Trait Abstractions Break Cycles

**If A needs B and B needs A, introduce a trait in foundation layer.**

Example:
```rust
// BEFORE (circular)
ai/agent.rs → needs PTY to run commands
pty/manager.rs → needs AI to execute agent tools

// AFTER (acyclic via trait)
qbit-core → has CommandExecutor trait
qbit-pty → implements CommandExecutor trait
qbit-ai → uses CommandExecutor trait (not concrete PTY)
main crate → wires up concrete implementations
```

---

### Principle 4: One-Way Data Flow

**Data should flow DOWN the layer hierarchy, never UP.**

```
Layer 4: Application → emits commands to Layer 3
Layer 3: Domain → emits events to Layer 2
Layer 2: Infrastructure → provides services to Layer 3
Layer 1: Foundation → provides types to everyone

✅ Layer 3 can use Layer 1
❌ Layer 1 cannot use Layer 3
```

---

## Validation: How to Ensure No Cycles

### After Each Extraction

```bash
# 1. Visualize dependency graph
cargo tree -p qbit --edges normal --no-indent > deps.txt

# 2. Check for cycles (should be empty)
grep -E "qbit-.*→.*qbit-" deps.txt

# 3. Verify layer separation
cargo tree -p qbit-core | grep "qbit-"  # Should show NO internal deps
cargo tree -p qbit-tools | grep "qbit-" # Should only show qbit-core
```

### Automated Check (CI)

```bash
#!/bin/bash
# scripts/check-no-cycles.sh

# Get all workspace crates
crates=$(cargo metadata --format-version 1 --no-deps | jq -r '.workspace_members[]')

for crate in $crates; do
    # Check qbit-core has no internal deps
    if [[ $crate == *"qbit-core"* ]]; then
        deps=$(cargo tree -p qbit-core | grep -c "qbit-" || echo "0")
        if [[ $deps -gt 0 ]]; then
            echo "❌ qbit-core has internal dependencies!"
            exit 1
        fi
    fi

    # Check for cycles
    cycles=$(cargo tree -p ${crate} --edges normal | grep -E "qbit-.*→.*qbit-.*→.*qbit-" || echo "")
    if [[ -n "$cycles" ]]; then
        echo "❌ Circular dependency detected in ${crate}!"
        echo "$cycles"
        exit 1
    fi
done

echo "✅ No circular dependencies found"
```

---

## Benefits of Dependency-Driven Approach

### 1. Architectural Enforcement

Cargo becomes your architecture police:
```rust
// In qbit-core, this won't compile:
use qbit_tools::ToolRegistry;  // ❌ Error: circular dependency

// Forces you to fix the architecture:
// Move shared types to qbit-core instead
```

### 2. Easier Reasoning

Clear dependency direction:
- "Where should this type go?" → Follow the layer rules
- "Can I use this crate?" → Check the dependency graph
- "Is this circular?" → Cargo will tell you immediately

### 3. Incremental Migration

Because we're focusing on breaking cycles, we can migrate incrementally:
1. Extract foundation types first (breaks most cycles)
2. Extract infrastructure (clean dependencies)
3. Extract domain (already acyclic)
4. Clean up application layer

### 4. Better Testing

Each layer can be tested independently:
```bash
cargo test -p qbit-core        # No external dependencies
cargo test -p qbit-tools       # Only depends on qbit-core
cargo test -p qbit             # Integration tests
```

---

## Comparison: Original Plan vs Dependency-Driven

| Aspect | Original Plan | Dependency-Driven |
|--------|---------------|-------------------|
| **Focus** | Code organization | Breaking cycles |
| **Commands** | Extract with crates | Keep in main (application layer) |
| **Events** | Stay with AI module | Extract to qbit-core (foundation) |
| **Validation** | Manual review | Cargo enforces acyclic graph |
| **Risk** | Circular deps possible | Impossible (Cargo prevents) |
| **Testing** | Hope tests catch issues | Compilation errors force fixes |

---

## Updated Timeline (Dependency-Driven)

| Phase | Focus | Duration |
|-------|-------|----------|
| **Phase 0** | Move shared types to foundation | 2 weeks |
| **Phase 1** | Extract foundation layer (qbit-core) | 3 weeks |
| **Phase 2** | Extract domain layer (tools, pty, indexer) | 4 weeks |
| **Phase 3** | Extract advanced domain (sidecar, context) | 3 weeks |
| **Phase 4** | Clean up application layer | 2 weeks |
| **Total** | | **14 weeks** |

Same timeline, but **higher confidence** because Cargo prevents cycles.

---

## Success Criteria

### After Each Phase

- [ ] `cargo tree` shows no cycles
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes
- [ ] Layer separation validated:
  - qbit-core: 0 internal deps
  - Infrastructure: only qbit-core
  - Domain: only foundation + infrastructure
  - Application: depends on everything

---

## Conclusion

**Original approach**: "Let's organize code into crates for modularity"
**Dependency-driven approach**: "Let's use crates to FORCE proper architecture and eliminate cycles"

**Key difference**: The dependency-driven approach uses Cargo's acyclic graph enforcement as a **design constraint**, not just a side effect.

This approach:
- ✅ Eliminates circular dependencies **by construction**
- ✅ Enforces proper layering **at compile time**
- ✅ Makes architecture violations **impossible** (not just bad practice)
- ✅ Provides **immediate feedback** when design is wrong

**Recommendation**: Use this dependency-driven approach. It leverages Rust's module system as an architectural safety mechanism.
