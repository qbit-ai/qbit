# Step 7: Remove Pruning

**Goal:** Remove the legacy pruning system now that compaction is in place. This is cleanup - removing dead code and deprecated features.

**Outcome:** After this step, the codebase no longer has pruning logic. Context management is done exclusively via compaction.

---

## Prerequisites

- Steps 1-6 completed (compaction fully working)
- Verified compaction works in production-like scenarios

## Files to Delete

| File | Description |
|------|-------------|
| `backend/crates/qbit-context/src/context_pruner.rs` | Entire file - pruning logic |

## Files to Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-context/src/lib.rs` | Modify | Remove context_pruner exports |
| `backend/crates/qbit-context/src/context_manager.rs` | Modify | Remove pruning imports and methods |
| `backend/crates/qbit-core/src/events.rs` | Modify | Remove ContextPruned event |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Modify | Remove pruning calls |
| `backend/crates/qbit-cli-output/src/lib.rs` | Modify | Remove ContextPruned handler |
| `backend/crates/qbit-sidecar/src/capture.rs` | Modify | Remove ContextPruned from filter |
| `backend/crates/qbit/tests/ai_events_characterization.rs` | Modify | Remove ContextPruned tests |
| `backend/crates/qbit-settings/src/schema.rs` | Modify | Deprecate protected_turns, cooldown_seconds |
| `frontend/lib/ai.ts` | Modify | Remove context_pruned type |
| `frontend/hooks/useAiEvents.ts` | Modify | Remove context_pruned handler |

---

## Task Breakdown

### 7.1 Delete context_pruner.rs

```bash
rm backend/crates/qbit-context/src/context_pruner.rs
```

### 7.2 Update qbit-context/src/lib.rs

**File:** `backend/crates/qbit-context/src/lib.rs`

Before:
```rust
pub mod context_manager;
pub mod context_pruner;
pub mod token_budget;
pub mod token_trunc;

// Re-export main types
pub use context_manager::{
    ContextEnforcementResult, ContextEvent, ContextManager, ContextManagerConfig,
    ContextPrunedInfo, ContextSummary, ContextTrimConfig, ContextWarningInfo,
};
pub use context_pruner::{ContextPruner, ContextPrunerConfig, PruneResult, SemanticScore};
// ...
```

After:
```rust
pub mod context_manager;
pub mod token_budget;
pub mod token_trunc;

// Re-export main types
pub use context_manager::{
    CompactionCheck, CompactionState, ContextEnforcementResult, ContextEvent,
    ContextManager, ContextManagerConfig, ContextSummary, ContextTrimConfig,
    ContextWarningInfo,
};
// Note: ContextPrunedInfo, ContextPruner, ContextPrunerConfig, PruneResult, SemanticScore removed
// ...
```

### 7.3 Update context_manager.rs

**File:** `backend/crates/qbit-context/src/context_manager.rs`

Remove:
- `use crate::context_pruner::...` imports
- `pruner: Arc<RwLock<ContextPruner>>` field
- All methods that use ContextPruner
- `enforce_context_window` method (or rewrite to use compaction)
- `ContextEvent::ContextPruned` variant
- `ContextPrunedInfo` struct
- Tests related to pruning

Keep:
- Token budget management
- Compaction trigger logic (added in Step 4)
- `CompactionState`, `CompactionCheck`

```rust
// Remove these from struct ContextManager:
// - pruner: Arc<RwLock<ContextPruner>>
// - last_prune_time: Arc<RwLock<Option<u64>>>
// - prune_cooldown_seconds: u64

// Remove these methods:
// - enforce_context_window() - or replace with compaction-based version
// - preview_prune()
// - score_message()

// Remove ContextEvent::ContextPruned variant from the enum

// Remove ContextPrunedInfo struct
```

### 7.4 Update events.rs

**File:** `backend/crates/qbit-core/src/events.rs`

Remove:
```rust
// Remove this variant:
/// Context was pruned due to token limits
ContextPruned {
    messages_removed: usize,
    tokens_freed: usize,
    utilization_before: f64,
    utilization_after: f64,
},

// Remove from event_type() match:
AiEvent::ContextPruned { .. } => "context_pruned",
```

### 7.5 Update agentic_loop.rs

**File:** `backend/crates/qbit-ai/src/agentic_loop.rs`

Remove:
- Any calls to `enforce_context_window()`
- Emission of `AiEvent::ContextPruned`
- Related logging/handling

The compaction logic from Step 5 replaces this.

### 7.6 Update CLI output

**File:** `backend/crates/qbit-cli-output/src/lib.rs`

Remove:
```rust
AiEvent::ContextPruned {
    messages_removed,
    tokens_freed,
    utilization_before,
    utilization_after,
} => {
    // ... remove this handler
}
```

### 7.7 Update sidecar capture

**File:** `backend/crates/qbit-sidecar/src/capture.rs`

Remove:
```rust
| AiEvent::ContextPruned { .. }
```

### 7.8 Update characterization tests

**File:** `backend/crates/qbit/tests/ai_events_characterization.rs`

Remove:
- `test_context_pruned_serialization` test
- ContextPruned from the roundtrip test's event list

### 7.9 Deprecate settings (don't remove for backwards compat)

**File:** `backend/crates/qbit-settings/src/schema.rs`

Update comments:
```rust
pub struct ContextSettings {
    /// Enable context window management
    #[serde(default = "default_context_enabled")]
    pub enabled: bool,

    /// Context utilization threshold (0.0-1.0) at which compaction is triggered
    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: f64,

    /// DEPRECATED: No longer used. Compaction replaces pruning.
    /// Kept for backwards compatibility with existing config files.
    #[serde(default = "default_protected_turns")]
    #[deprecated(note = "Pruning has been replaced by compaction")]
    pub protected_turns: usize,

    /// DEPRECATED: No longer used. Compaction replaces pruning.
    /// Kept for backwards compatibility with existing config files.
    #[serde(default = "default_cooldown_seconds")]
    #[deprecated(note = "Pruning has been replaced by compaction")]
    pub cooldown_seconds: u64,
}
```

### 7.10 Update frontend types

**File:** `frontend/lib/ai.ts`

Remove:
```typescript
| {
    type: "context_pruned";
    messages_removed: number;
    tokens_freed: number;
    utilization_before: number;
    utilization_after: number;
  }
```

### 7.11 Update frontend event handler

**File:** `frontend/hooks/useAiEvents.ts`

Remove:
```typescript
case "context_pruned":
  state.setContextMetrics(sessionId, {
    utilization: event.utilization_after,
    isWarning: event.utilization_after >= 0.75,
    lastPruned: new Date().toISOString(),
    messagesRemoved: event.messages_removed,
    tokensFreed: event.tokens_freed,
  });
  state.addNotification({
    type: "info",
    title: "Context Pruned",
    message: `Removed ${event.messages_removed} old messages to free up ${event.tokens_freed.toLocaleString()} tokens.`,
  });
  break;
```

### 7.12 Update store

Remove any pruning-specific state:
```typescript
// Remove from session state:
// - lastPruned
// - messagesRemoved  
// - tokensFreed (if pruning-specific)
```

---

## Verification

### Compile Check (most important)
```bash
cd backend
cargo build
cargo test
```

### Frontend Check
```bash
cd frontend
pnpm typecheck
pnpm build
```

### Search for Remaining References
```bash
# In backend
grep -r "ContextPruned" backend/crates/
grep -r "context_pruned" backend/crates/
grep -r "ContextPruner" backend/crates/
grep -r "enforce_context_window" backend/crates/

# In frontend  
grep -r "context_pruned" frontend/
grep -r "pruned" frontend/lib/ai.ts
```

All should return no matches (except comments documenting the removal).

### Run Full Test Suite
```bash
cd backend && cargo test
cd frontend && pnpm test
```

---

## Definition of Done

- [ ] `context_pruner.rs` deleted
- [ ] No compilation errors after deletion
- [ ] `lib.rs` no longer exports pruning types
- [ ] `context_manager.rs` cleaned of pruning logic
- [ ] `AiEvent::ContextPruned` removed from events.rs
- [ ] CLI output handler removed
- [ ] Sidecar filter updated
- [ ] Characterization tests updated
- [ ] Settings deprecated (with comments)
- [ ] Frontend TypeScript types updated
- [ ] Frontend event handler removed
- [ ] No grep matches for pruning-related terms
- [ ] All backend tests pass
- [ ] All frontend tests pass
- [ ] Application builds and runs correctly

---

## Rollback Plan

If issues are discovered after removing pruning:

1. **Git revert** the deletion commits
2. Pruning and compaction can coexist temporarily
3. Add feature flag to switch between them

However, since Steps 1-6 ensure compaction is fully working before this step, rollback should not be necessary.

---

## Notes

- This is a **breaking change** for any external code that depends on ContextPruned
- The settings are kept (but deprecated) for backwards compatibility
- Users with old config files won't see errors, just warnings about deprecated fields
- After this step, the only context management mechanism is compaction
- The removal simplifies the codebase significantly
