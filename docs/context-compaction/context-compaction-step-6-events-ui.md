# Step 6: Events + UI

**Goal:** Add new event types for compaction, update frontend to handle them, and provide user feedback during/after compaction.

**Outcome:** After this step, users see a notification when compaction occurs, can retry failed compactions, and understand when a session is unusable.

---

## Prerequisites

- Steps 1-5 completed (full compaction flow working)

## Files to Modify

### Backend
| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-core/src/events.rs` | Modify | Add ContextCompacted and ContextCompactionFailed events |
| `backend/crates/qbit-cli-output/src/lib.rs` | Modify | Handle new events in CLI output |
| `backend/crates/qbit-sidecar/src/capture.rs` | Modify | Update event filter for new events |
| `backend/crates/qbit/tests/ai_events_characterization.rs` | Modify | Add serialization tests |

### Frontend
| File | Action | Description |
|------|--------|-------------|
| `frontend/lib/ai.ts` | Modify | Add TypeScript types for new events |
| `frontend/hooks/useAiEvents.ts` | Modify | Handle new events |
| `frontend/store/` | Modify | Add compaction state and actions |

---

## Task Breakdown

### 6.1 Add new event types to backend

**File:** `backend/crates/qbit-core/src/events.rs`

```rust
// Add these variants to the AiEvent enum:

/// Context was compacted via summarization
ContextCompacted {
    /// Tokens before compaction
    tokens_before: u64,
    /// How threshold was determined: "provider_usage" or "heuristic"
    trigger_reason: String,
    /// Model used for context limit determination
    model: String,
    /// Path to transcript JSON file (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    transcript_path: Option<String>,
    /// Path to summary artifact (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    summary_path: Option<String>,
    /// Number of compactions performed this session
    compaction_count: u32,
},

/// Context compaction failed
ContextCompactionFailed {
    /// Error message describing the failure
    error: String,
    /// Whether context has exceeded limits (session cannot continue)
    context_exceeded: bool,
    /// Tokens at time of failure
    tokens_current: u64,
    /// Max tokens for the model
    max_tokens: u64,
},

// Update the event_type() method:
impl AiEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            // ... existing matches ...
            AiEvent::ContextCompacted { .. } => "context_compacted",
            AiEvent::ContextCompactionFailed { .. } => "context_compaction_failed",
        }
    }
}
```

**Tests:**
```rust
#[test]
fn context_compacted_event_json_format() {
    let event = AiEvent::ContextCompacted {
        tokens_before: 150000,
        trigger_reason: "provider_usage".to_string(),
        model: "claude-3-5-sonnet".to_string(),
        transcript_path: Some("/path/to/transcript.json".to_string()),
        summary_path: Some("/path/to/summary.md".to_string()),
        compaction_count: 1,
    };
    let json = serde_json::to_value(&event).unwrap();

    assert_eq!(json["type"], "context_compacted");
    assert_eq!(json["tokens_before"], 150000);
    assert_eq!(json["trigger_reason"], "provider_usage");
    assert_eq!(json["model"], "claude-3-5-sonnet");
    assert_eq!(json["compaction_count"], 1);
}

#[test]
fn context_compaction_failed_event_json_format() {
    let event = AiEvent::ContextCompactionFailed {
        error: "Summarizer timeout".to_string(),
        context_exceeded: false,
        tokens_current: 180000,
        max_tokens: 200000,
    };
    let json = serde_json::to_value(&event).unwrap();

    assert_eq!(json["type"], "context_compaction_failed");
    assert_eq!(json["error"], "Summarizer timeout");
    assert_eq!(json["context_exceeded"], false);
    assert_eq!(json["tokens_current"], 180000);
    assert_eq!(json["max_tokens"], 200000);
}

#[test]
fn context_compaction_failed_exceeded_event_json_format() {
    let event = AiEvent::ContextCompactionFailed {
        error: "Summarizer failed and context limit exceeded".to_string(),
        context_exceeded: true,
        tokens_current: 210000,
        max_tokens: 200000,
    };
    let json = serde_json::to_value(&event).unwrap();

    assert_eq!(json["type"], "context_compaction_failed");
    assert_eq!(json["context_exceeded"], true);
}
```

### 6.2 Update CLI output handler

**File:** `backend/crates/qbit-cli-output/src/lib.rs`

```rust
// Add handling for new events:

AiEvent::ContextCompacted {
    tokens_before,
    compaction_count,
    ..
} => {
    println!(
        "{}",
        style(format!(
            "✓ Context compacted (was {} tokens, compaction #{})",
            tokens_before, compaction_count
        ))
        .cyan()
    );
}

AiEvent::ContextCompactionFailed {
    error,
    context_exceeded,
    tokens_current,
    max_tokens,
} => {
    if *context_exceeded {
        println!(
            "{}",
            style(format!(
                "✗ Context limit exceeded ({}/{} tokens). Session cannot continue. Error: {}",
                tokens_current, max_tokens, error
            ))
            .red()
            .bold()
        );
    } else {
        println!(
            "{}",
            style(format!(
                "⚠ Compaction failed ({}/{} tokens): {}",
                tokens_current, max_tokens, error
            ))
            .yellow()
        );
    }
}
```

### 6.3 Update sidecar event filter

**File:** `backend/crates/qbit-sidecar/src/capture.rs`

```rust
// In the event filter/match, add new events:

| AiEvent::ContextCompacted { .. }
| AiEvent::ContextCompactionFailed { .. } => {
    // Capture these events for the session log
    true
}
```

### 6.4 Update characterization tests

**File:** `backend/crates/qbit/tests/ai_events_characterization.rs`

Add the new event types to the roundtrip test and add specific tests for serialization format.

### 6.5 Add TypeScript types for frontend

**File:** `frontend/lib/ai.ts`

```typescript
// Add to the AiEvent union type:

| {
    type: "context_compacted";
    tokens_before: number;
    trigger_reason: string;
    model: string;
    transcript_path?: string;
    summary_path?: string;
    compaction_count: number;
  }
| {
    type: "context_compaction_failed";
    error: string;
    context_exceeded: boolean;
    tokens_current: number;
    max_tokens: number;
  }
```

### 6.6 Add compaction state to store

**File:** `frontend/store/index.ts` (or wherever session state is managed)

```typescript
// Add to session state:
interface SessionState {
  // ... existing fields ...
  
  /** Number of compactions performed this session */
  compactionCount: number;
  /** Whether compaction is currently in progress */
  isCompacting: boolean;
  /** Whether session is dead (context exceeded and compaction failed) */
  isSessionDead: boolean;
  /** Last compaction error (for retry UI) */
  compactionError: string | null;
}

// Add actions:
interface StoreActions {
  // ... existing actions ...
  
  /** Mark compaction as in progress */
  setCompacting: (sessionId: string, isCompacting: boolean) => void;
  
  /** Handle successful compaction */
  handleCompactionSuccess: (sessionId: string, compactionCount: number) => void;
  
  /** Handle failed compaction */
  handleCompactionFailed: (
    sessionId: string,
    error: string,
    contextExceeded: boolean
  ) => void;
  
  /** Clear compaction error (for retry) */
  clearCompactionError: (sessionId: string) => void;
}
```

### 6.7 Update useAiEvents hook

**File:** `frontend/hooks/useAiEvents.ts`

```typescript
// Replace the context_pruned handler with new handlers:

case "context_compacted":
  state.handleCompactionSuccess(sessionId, event.compaction_count);
  state.setContextMetrics(sessionId, {
    // Reset metrics after compaction
    utilization: 0,
    usedTokens: 0,
    isWarning: false,
  });
  state.addNotification({
    type: "success",
    title: "Session Compacted",
    message: `Conversation history summarized. This is compaction #${event.compaction_count} for this session.`,
    duration: 5000,
  });
  break;

case "context_compaction_failed":
  state.handleCompactionFailed(
    sessionId,
    event.error,
    event.context_exceeded
  );
  
  if (event.context_exceeded) {
    state.addNotification({
      type: "error",
      title: "Session Limit Exceeded",
      message: "Context limit exceeded and compaction failed. Please start a new session.",
      persistent: true, // Don't auto-dismiss
      action: {
        label: "New Session",
        onClick: () => {
          // Navigate to new session
          window.location.href = "/";
        },
      },
    });
  } else {
    state.addNotification({
      type: "warning",
      title: "Compaction Failed",
      message: `Failed to compact session: ${event.error}. You can retry or continue.`,
      action: {
        label: "Retry",
        onClick: () => {
          state.clearCompactionError(sessionId);
          // Trigger retry via command
          invoke("retry_compaction", { sessionId });
        },
      },
    });
  }
  break;
```

### 6.8 Add retry_compaction command

**File:** `backend/crates/qbit/src/ai/commands/context.rs` (or similar)

```rust
/// Retry compaction for a session.
///
/// This resets the compaction attempt tracker and allows the next
/// turn to attempt compaction again.
#[tauri::command]
pub async fn retry_compaction(
    session_id: String,
    ai_state: State<'_, AiState>,
) -> Result<(), String> {
    let bridge = ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    // Reset the compaction attempt state
    bridge.reset_compaction_attempt().await;
    
    Ok(())
}
```

**Add to AgentBridge:**
```rust
/// Reset the compaction attempt flag (for manual retry).
pub async fn reset_compaction_attempt(&self) {
    // This would need to communicate with the agentic loop
    // For now, store a flag that the loop checks
    *self.retry_compaction.write().await = true;
}
```

### 6.9 Add UI components for compaction state

**File:** Create or update timeline/status components

```typescript
// Component to show compaction status in the timeline
interface CompactionStatusProps {
  isCompacting: boolean;
  compactionCount: number;
  error: string | null;
  isSessionDead: boolean;
  onRetry: () => void;
  onNewSession: () => void;
}

function CompactionStatus({
  isCompacting,
  compactionCount,
  error,
  isSessionDead,
  onRetry,
  onNewSession,
}: CompactionStatusProps) {
  if (isCompacting) {
    return (
      <div className="flex items-center gap-2 text-blue-500">
        <Spinner size="sm" />
        <span>Compacting conversation...</span>
      </div>
    );
  }

  if (isSessionDead) {
    return (
      <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded">
        <strong>Session Limit Exceeded</strong>
        <p>This session cannot continue. Please start a new session.</p>
        <button
          onClick={onNewSession}
          className="mt-2 bg-red-500 text-white px-4 py-2 rounded"
        >
          Start New Session
        </button>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-yellow-100 border border-yellow-400 text-yellow-700 px-4 py-3 rounded">
        <strong>Compaction Failed</strong>
        <p>{error}</p>
        <button
          onClick={onRetry}
          className="mt-2 bg-yellow-500 text-white px-4 py-2 rounded"
        >
          Retry Compaction
        </button>
      </div>
    );
  }

  if (compactionCount > 0) {
    return (
      <div className="text-sm text-gray-500">
        Session compacted {compactionCount} time{compactionCount > 1 ? "s" : ""}
      </div>
    );
  }

  return null;
}
```

### 6.10 Disable input during compaction

**File:** Update input component

```typescript
// In the chat input component:
const isCompacting = useStore((s) => s.sessions[sessionId]?.isCompacting);
const isSessionDead = useStore((s) => s.sessions[sessionId]?.isSessionDead);

// Disable input when compacting or session is dead
const isDisabled = isCompacting || isSessionDead || /* other conditions */;

<input
  disabled={isDisabled}
  placeholder={
    isCompacting
      ? "Compacting conversation..."
      : isSessionDead
      ? "Session limit exceeded. Please start a new session."
      : "Type a message..."
  }
  // ...
/>
```

---

## Verification

### Run Backend Tests
```bash
cd backend
cargo test -p qbit-core events
cargo test -p qbit ai_events_characterization
```

### Run Frontend Tests
```bash
cd frontend
pnpm test
pnpm typecheck
```

### Manual Testing

1. **Test compaction success notification:**
   - Trigger compaction (lower threshold temporarily)
   - Verify notification appears
   - Verify compaction count is shown

2. **Test compaction failure:**
   - Mock summarizer failure
   - Verify warning notification with retry button
   - Click retry and verify it works

3. **Test context exceeded:**
   - Simulate context exceeded state
   - Verify error notification appears
   - Verify input is disabled
   - Verify "New Session" button works

4. **Test input disabled during compaction:**
   - Trigger compaction
   - Verify input is disabled with appropriate placeholder

### Integration Check
```bash
cd backend && cargo test
cd frontend && pnpm build
```

---

## Definition of Done

- [ ] `AiEvent::ContextCompacted` variant added with all fields
- [ ] `AiEvent::ContextCompactionFailed` variant added with all fields
- [ ] `event_type()` returns correct strings for new events
- [ ] CLI output handles new events appropriately
- [ ] Sidecar captures new events
- [ ] TypeScript types match Rust event structure
- [ ] Store has compaction state fields and actions
- [ ] `useAiEvents` handles new events correctly
- [ ] Notifications appear for success/failure/exceeded
- [ ] Retry button works for failed compactions
- [ ] Input disabled during compaction
- [ ] Input disabled and "New Session" shown when exceeded
- [ ] All tests pass
- [ ] TypeScript compiles without errors

---

## Notes

- The `context_pruned` event still exists - it will be removed in Step 7
- Notifications should be dismissible but persist longer for errors
- The "New Session" flow should preserve workspace context
- Compaction count helps users understand how many times their session has been summarized
- The `retry_compaction` command resets the attempt flag but doesn't force immediate compaction
