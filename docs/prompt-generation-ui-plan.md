# Plan: Show Meta-Prompt Generation in Timeline UI

## Goal

When a worker agent is dispatched, the prompt generation step should be visible in the timeline UI. The user should see:
1. That prompt generation is happening (before the worker starts)
2. The system prompt (WORKER_PROMPT_TEMPLATE) sent to the architect LLM
3. The user message (task + context) sent to the architect LLM
4. The generated system prompt that the worker will use

## Current State

- Prompt generation runs inside `execute_sub_agent_inner()` in `executor.rs`
- It makes a `model.completion()` call and uses the response as the worker's system prompt
- No events are emitted — the frontend has no visibility into this step
- The only log evidence is tracing info lines in `backend.log`

## Proposed Approach

### 1. New AiEvent Variant

Add a new event to `AiEvent` in `qbit-core/src/events.rs`:

```rust
/// Emitted when a sub-agent's system prompt is being generated via an LLM call.
PromptGeneration {
    /// The sub-agent this prompt is being generated for
    agent_id: String,
    /// The parent request that triggered this sub-agent
    parent_request_id: String,
    /// The system prompt sent to the architect LLM (the meta-prompt template)
    architect_system_prompt: String,
    /// The user message sent to the architect LLM (task + context)
    architect_user_message: String,
    /// The generated system prompt (None if still in progress or failed)
    generated_prompt: Option<String>,
    /// Whether generation succeeded
    success: bool,
    /// Duration of the generation call in milliseconds
    duration_ms: u64,
}
```

Alternatively, split into two events for streaming-friendly display:
- `PromptGenerationStarted { agent_id, parent_request_id, architect_system_prompt, architect_user_message }`
- `PromptGenerationCompleted { agent_id, parent_request_id, generated_prompt, success, duration_ms }`

The two-event approach is better because:
- The UI can show a "generating..." state immediately
- The system prompt and user message are visible before the LLM responds
- Matches the existing `SubAgentStarted`/`SubAgentCompleted` pattern

### 2. Emit Events in Executor

In `execute_sub_agent_inner()`, wrap the prompt generation call with event emission:

```rust
// Before the completion call
let _ = ctx.event_tx.send(AiEvent::PromptGenerationStarted {
    agent_id: agent_id.to_string(),
    parent_request_id: parent_request_id.to_string(),
    architect_system_prompt: template.clone(),
    architect_user_message: generation_input.clone(),
});

// After the completion call
let _ = ctx.event_tx.send(AiEvent::PromptGenerationCompleted {
    agent_id: agent_id.to_string(),
    parent_request_id: parent_request_id.to_string(),
    generated_prompt: Some(generated.clone()),
    success: true,
    duration_ms: generation_start.elapsed().as_millis() as u64,
});
```

### 3. Frontend Event Handling

In `frontend/hooks/useAiEvents.ts`, handle the new events:
- `PromptGenerationStarted` → create a new streaming block or timeline entry showing the generation is in progress
- `PromptGenerationCompleted` → update the block with the generated prompt

### 4. Timeline UI Component

Create a new component (e.g., `PromptGenerationBlock`) that displays:

```
┌─────────────────────────────────────────┐
│ 🔧 Generating Worker System Prompt      │
│                                         │
│ ▸ Architect System Prompt (click to     │
│   expand — shows WORKER_PROMPT_TEMPLATE)│
│                                         │
│ ▸ Task Input                            │
│   "Task: find all functions with >5     │
│    parameters"                          │
│                                         │
│ ▸ Generated System Prompt               │
│   "You are a Rust code analyst          │
│    specializing in function signature   │
│    analysis..."                         │
│                                         │
│ ⏱ Generated in 1.2s                    │
└─────────────────────────────────────────┘
```

Design considerations:
- The architect system prompt is long (~1500 chars) — show collapsed by default with expand toggle
- The generated prompt is the most interesting part — show it expanded
- The task input should be visible to provide context
- Show duration so the user knows the overhead cost
- This block should appear BEFORE the sub-agent's tool call block in the timeline
- Use a distinct visual style (different icon/color) to differentiate from regular tool calls

### 5. Transcript Integration

The prompt generation events should be written to the transcript (via `should_transcript()` in `transcript.rs`) so they're preserved across sessions and visible in compaction artifacts.

### 6. UnifiedBlock Integration

Add a new `UnifiedBlock` type for prompt generation in the store, similar to how `system_hook` blocks work. This ensures the prompt generation step appears as a distinct entry in the unified timeline.

## Files to Modify

| File | Change |
|------|--------|
| `qbit-core/src/events.rs` | Add `PromptGenerationStarted` and `PromptGenerationCompleted` event variants |
| `qbit-sub-agents/src/executor.rs` | Emit the new events around the `model.completion()` call |
| `qbit-ai/src/transcript.rs` | Add new events to `should_transcript()` |
| `frontend/hooks/useAiEvents.ts` | Handle new event types |
| `frontend/store/index.ts` | Add prompt generation block type to store |
| `frontend/components/PromptGenerationBlock/` | New component for timeline display |
| `frontend/components/UnifiedTimeline/` | Render the new block type |

## Implementation Order

1. Add event variants to `qbit-core` (Rust)
2. Emit events in executor (Rust)
3. Add to transcript filter (Rust)
4. Handle events in `useAiEvents.ts` (Frontend)
5. Add block type to store (Frontend)
6. Build `PromptGenerationBlock` component (Frontend)
7. Wire into `UnifiedTimeline` (Frontend)

## Open Questions

- Should the architect system prompt be truncated in the event payload? It's ~1500 chars which is fine for events but could bloat transcripts if many workers are dispatched.
- Should prompt generation have its own Langfuse span? Currently it runs inside the `sub_agent` span but a dedicated `prompt_generation` span would give better observability.
- Should there be a setting to disable the prompt generation UI display (for users who find it noisy)?
