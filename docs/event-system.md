# Event System

Qbit uses Tauri's event system for communication between the Rust backend and React frontend.

## Terminal Events

| Event | Payload | Description |
|-------|---------|-------------|
| `terminal_output` | `{session_id, data}` | Raw PTY output |
| `command_block` | `CommandBlock` | Parsed command with output |

## AI Events

All AI events are emitted as `ai-event` with a type discriminator.

### Lifecycle Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `started` | `turn_id` | Agent turn started |
| `completed` | `response`, `tokens_used` | Turn finished |
| `error` | `message`, `error_type` | Error occurred |

### Streaming Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `text_delta` | `delta`, `accumulated` | Streaming text chunk |
| `reasoning` | `content` | Extended thinking content |

### Tool Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `tool_approval_request` | `request_id`, `tool_name`, `args`, `risk_level` | Requires user approval |
| `tool_auto_approved` | `request_id`, `reason` | Auto-approved by pattern |
| `tool_result` | `request_id`, `success`, `result` | Tool execution completed |

### Workflow Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `workflow_*` | `workflow_id`, `step_*` | Workflow lifecycle events |

### Context Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `context_*` | utilization metrics | Context window management |
| `compaction_started` | `tokens_before`, `messages_before` | Context compaction initiated |
| `compaction_completed` | `tokens_before`, `messages_before/after`, `summary_length` | Compaction succeeded |
| `compaction_failed` | `tokens_before`, `messages_before`, `error` | Compaction failed |

### Loop Detection Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `loop_*` | detection stats | Loop protection events |

### System Events

| Event Type | Key Fields | Description |
|------------|------------|-------------|
| `system_hooks_injected` | `hooks` | System hooks injected into conversation |

**UI note**: `system_hooks_injected` is persisted into the unified timeline as a `UnifiedBlock` of type `system_hook`.

## Frontend Event Handling

Events are subscribed to in `frontend/hooks/useAiEvents.ts`:

```typescript
import { listen } from "@tauri-apps/api/event";

// Subscribe to AI events
const unlisten = await listen<AiEvent>("ai-event", (event) => {
  switch (event.payload.type) {
    case "text_delta":
      // Handle streaming text
      break;
    case "tool_approval_request":
      // Show approval dialog
      break;
    // ...
  }
});
```

## Backend Event Emission

Events are emitted from Rust using Tauri's `app.emit()`:

```rust
use qbit_core::events::AiEvent;

app.emit("ai-event", AiEvent::TextDelta {
    delta: "Hello".to_string(),
    accumulated: "Hello".to_string(),
})?;
```

## Adding New Events

1. Add variant to `AiEvent` enum in `backend/crates/qbit-core/src/events.rs`
2. Emit via `app.emit("ai-event", event)` in the backend
3. Handle in `frontend/hooks/useAiEvents.ts`
