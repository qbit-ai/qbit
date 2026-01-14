# Qbit: Conversation compaction without sidecar (implementation plan v8)

## Goal
Replace qbit's current **pruning** (dropping older messages) with **on-demand compaction** that:
- Monitors context growth.
- When a threshold is exceeded, generates a **detailed conversation summary** using a dedicated summarizer model.
- Performs a **hard reset** of the agent conversation and continues with a new system prompt that includes the summary.
- Does **not** use the existing sidecar feature (`state.md` / sidecar session injection).

---

## Confirmed decisions

### Compaction mechanics
- **Hard reset** is the restart mechanism.
  - Clear in-memory conversation history (the `messages: Vec<Message>` in `agentic_loop.rs`).
  - Continue with a new system prompt that ends with the `<summary>...</summary>` block and a short continuation notice.
- **No protected turns**. The summary must be sufficient; we do not keep a verbatim tail.
  - The existing `protected_turns` setting in `ContextSettings` and `ContextManagerConfig` will be **deprecated** (ignored when compaction is enabled).
- **No `TextDelta` capture** for transcript. Only complete agent responses.
- **No truncation fallback**. If compaction fails, we do not fall back to pruning/truncation.
- **User input during compaction:** reject it (UI disabled / blocked).
- Compaction should only begin when we are **between turns** (before starting a new agent loop).
  - No long-running background tasks exist currently, so "no tool calls in flight" is not a concern for now.

### Compaction attempt tracking
- **Explicitly track compaction attempts** per turn (e.g., a counter or flag in session state).
- **Only attempt compaction once per turn.**
- If compaction fails, **do not retry automatically in the same turn**.
  - Require **manual retry** (explicit UI action) *or* wait until the **next successful model call** and re-evaluate compaction.

### Transcript capture
- Transcript is built from **`AiEvent`** (source of truth).
- **Include HITL events** (approval requests/approved/denied) in the transcript.
- Storage is via **`VT_SESSION_DIR`** (existing session archive dir mechanism; defaults to `~/.qbit/sessions`).

### Summarizer agent
- Summarizer is **isolated like `commit_writer`**:
  - Not a sub-agent.
  - Not callable by other agents.
  - No tools.
  - No HITL.
  - Hardcoded system prompt initially.

### Token counting strategy
**Use provider-returned token usage.** This is the simplest and most accurate approach.

Every major LLM provider (Anthropic, OpenAI, Google, Groq, xAI, etc.) returns `usage` data in their API responses:
- Anthropic: `usage.input_tokens`, `usage.output_tokens`
- OpenAI: `usage.prompt_tokens`, `usage.completion_tokens`, `usage.total_tokens`
- Google: `usageMetadata.promptTokenCount`, `usageMetadata.candidatesTokenCount`

**Existing infrastructure:** The `TokenUsage` struct in `qbit-context/src/token_budget.rs` already has `input_tokens` and `output_tokens` fields. The `agentic_loop.rs` already imports `TokenUsage` from `qbit_context::token_budget`.

**Key insight:** We don't need to count tokens *before* a call. We check usage *after* each LLM response and trigger compaction *before the next turn*.

**Flow:**
1. LLM call completes → provider returns `usage.input_tokens` (or equivalent)
2. Store/accumulate the usage in session state (extend existing `TokenUsage` tracking)
3. Before next turn: compare `input_tokens` against `max_context_tokens * compaction_threshold`
4. If over threshold → trigger compaction

**Fallback for missing usage:** Some providers (local Ollama, custom endpoints) may not return usage. Use `char/4` heuristic only in these cases.

### Model context limits
- **Context limits are model-dependent.** Each model has its own `max_context_tokens`.
- These limits are defined in `backend/crates/qbit-context/src/token_budget.rs` via `ModelContextLimits` and `TokenBudgetConfig::for_model()`.
- Currently only Claude models are defined; need to extend with GPT, Gemini, etc.
- Use existing **`compaction_threshold`** setting (default 0.80) as the trigger point.

### Prompt placement
- The summary is always the **last section** of the system prompt:

```md
## Continuation
This is a continuation of a previous session. The conversation so far has been compacted into the summary below.

<summary>
...
</summary>
```

---

## Simple 80% architecture

### What we are simplifying
- **No new crates required initially.** Implement transcript writing + summarizer input building inside existing backend crates.
  - (We can split into `qbit-transcript` / `qbit-summarizer` later if needed.)
- **No normalized transcript schema initially.** Store **raw `AiEvent` JSON** as JSONL.
  - This avoids designing/maintaining a parallel schema.
- **No "central emitter refactor" required upfront.** Hook transcript writing at the most central, already-existing event emission seam(s) (likely `AgentBridge` and/or the Tauri event emission path). If some events are missed, that's acceptable for the first iteration.

### Transcript file
- Append-only JSONL file in `VT_SESSION_DIR`:
  - `transcript-{session_id}.jsonl`
- Each line is the serialized `AiEvent` (including HITL events).

### Summarizer input builder
- Read the JSONL transcript and render a deterministic, human-readable text format.
- Keep formatting simple and stable; example:

```text
[turn 001] USER:
...

[turn 001] TOOL_REQUEST (tool=..., request_id=...):
{...}

[turn 001] TOOL_APPROVAL_REQUEST:
{...}

[turn 001] TOOL_RESULT:
...

[turn 001] ASSISTANT (completed):
...
```

### Summarizer artifacts
Write artifacts to `VT_SESSION_DIR`:
- `summarizer-input-{session_id}-{timestamp}.md`
- `summary-{session_id}-{timestamp}.md`

---

## Compaction flow (end-to-end)

### Trigger evaluation (between turns)
Compaction is evaluated **between turns** (before starting a new agent loop).

Trigger signal sources:
1) **Provider token usage** from prior LLM responses (`input_tokens` from `TokenUsage`).
2) **Heuristic fallback** when usage is unavailable (`char/4`).

Threshold comparison:
- Look up `max_context_tokens` for the **current model** via `TokenBudgetConfig::for_model()`.
- Trigger if `usage > max_context_tokens * compaction_threshold` (default 0.80).

### Compaction criteria
All must be true:
- Between turns.
- UI input is disabled/blocked during compaction.
- **Compaction not already attempted this turn** (check explicit tracking state).

### Steps
1. Check explicit compaction attempt tracker; if already attempted this turn, skip.
2. Mark compaction as attempted for this turn.
3. Disable UI input and show a timeline item: "Compaction occurring…"
4. Call summarizer:
   - Read transcript JSONL.
   - Build summarizer input.
   - Persist summarizer input.
   - Call summarizer model.
   - Persist summary output.
5. Hard reset:
   - Clear the `messages: Vec<Message>` in the agentic loop.
   - Set the next system prompt to:
     - normal base system prompt
     - plus continuation notice
     - plus `<summary>...</summary>` as the **last section**.
6. Reset compaction attempt tracker (new turn starts after reset).
7. Emit `AiEvent::ContextCompacted`.
8. Re-enable UI input.

### Failure handling
- On summarizer failure:
  - Emit `AiEvent::ContextCompactionFailed`.
  - Do **not** truncate or prune.
  - Do **not** retry automatically in the same turn (tracker already marked).
  - Allow manual retry (which resets the tracker) or wait for next turn.
- If context is already **over the limit** and compaction fails:
  - Emit `AiEvent::ContextCompactionFailed` with `context_exceeded: true`.
  - Display a clear message to the user: "Context limit exceeded and compaction failed. Please start a new session."
  - Disable further input (session is effectively dead).
  - Provide a "Start New Session" button in the UI.
  - The transcript remains available in `VT_SESSION_DIR` for manual recovery if needed.

---

## Removing pruning

- Remove pruning path entirely.
- Delete `backend/crates/qbit-context/src/context_pruner.rs`.
- Update `ContextManager` APIs to remove pruning semantics.
- Remove `AiEvent::ContextPruned` and related UI handling.
- Deprecate `protected_turns` and `cooldown_seconds` settings (keep in schema for backwards compat but ignore).

### Pruning references to update (22 total)
Based on codebase grep, these files reference `ContextPruned`/`context_pruned`:
- `backend/crates/qbit-core/src/events.rs` - enum variant and event_type() match
- `backend/crates/qbit-context/src/context_manager.rs` - ContextEvent::ContextPruned, ContextPrunedInfo
- `backend/crates/qbit-context/src/lib.rs` - re-exports ContextPrunedInfo
- `backend/crates/qbit-ai/src/agentic_loop.rs` - emits AiEvent::ContextPruned
- `backend/crates/qbit-cli-output/src/lib.rs` - CLI output handling
- `backend/crates/qbit-sidecar/src/capture.rs` - event capture filter
- `backend/crates/qbit/tests/ai_events_characterization.rs` - serialization tests
- `frontend/hooks/useAiEvents.ts` - event handler (lines 405-418)
- `frontend/lib/ai.ts` - TypeScript type definition (lines 289-295)

---

## Events + UI

### Backend
- Add `AiEvent::ContextCompacted`.
- Add `AiEvent::ContextCompactionFailed`.
- Remove `AiEvent::ContextPruned` (after updating all references).
- Include (where available):
  - tokens/usage before (input_tokens from TokenUsage or heuristic)
  - trigger reason (usage vs heuristic)
  - model used (for context limit lookup)
  - transcript path
  - summarizer input artifact path
  - summary artifact path

### Event schemas

```rust
// In backend/crates/qbit-core/src/events.rs

/// Context was compacted via summarization
ContextCompacted {
    /// Tokens before compaction
    tokens_before: u64,
    /// How threshold was determined
    trigger_reason: String,  // "provider_usage" or "heuristic"
    /// Model used for context limit
    model: String,
    /// Path to transcript JSONL file
    transcript_path: Option<String>,
    /// Path to summary artifact
    summary_path: Option<String>,
    /// Number of compactions performed this session
    compaction_count: u32,
},

/// Context compaction failed
ContextCompactionFailed {
    /// Error message
    error: String,
    /// Whether context has exceeded limits (session is dead)
    context_exceeded: bool,
    /// Tokens at time of failure
    tokens_current: u64,
    /// Max tokens for the model
    max_tokens: u64,
},
```

### Frontend

#### Files to update
| File | Changes |
|------|---------|
| `frontend/lib/ai.ts` | Add TypeScript types for `context_compacted` and `context_compaction_failed`; remove `context_pruned` type |
| `frontend/hooks/useAiEvents.ts` | Replace `context_pruned` handler (lines 405-418) with `context_compacted` and `context_compaction_failed` handlers |
| `frontend/store/index.ts` (or similar) | Update `setContextMetrics` calls; add `compactionCount` to context metrics |

#### Frontend behavior
- Handle `context_compacted` event:
  - Update context metrics via `state.setContextMetrics()`
  - Show notification: "Session compacted. Conversation history has been summarized."
  - Increment compaction counter display (if shown)
- Handle `context_compaction_failed` event:
  - If `context_exceeded: false`: Show warning notification with "Retry" button
  - If `context_exceeded: true`: Show error notification with "Start New Session" button; disable input
- Add a timeline item "Compacting conversation…" while compaction is running (between turns).
- Disable input while compaction is running.
- Add a "Retry compaction" UI action when `context_compaction_failed` occurs (resets the attempt tracker).

---

## Config plan

### Model context limits (existing, needs extension)
Model limits are defined in `backend/crates/qbit-context/src/token_budget.rs`.

**Current state:**
```rust
pub struct ModelContextLimits {
    pub claude_3_5_sonnet: usize,  // 200_000
    pub claude_3_opus: usize,      // 200_000
    pub claude_3_haiku: usize,     // 200_000
    pub claude_4_sonnet: usize,    // 200_000
    pub claude_4_opus: usize,      // 200_000
}
```

**Add these fields:**
```rust
    pub gpt_4o: usize,             // 128_000
    pub gpt_4_turbo: usize,        // 128_000
    pub gpt_4_1: usize,            // 1_047_576
    pub o1: usize,                 // 200_000
    pub o3: usize,                 // 200_000
    pub gemini_pro: usize,         // 1_000_000
    pub gemini_flash: usize,       // 1_000_000
```

Update `TokenBudgetConfig::for_model()` match arms to include:
- GPT-4o, GPT-4-turbo → 128,000
- GPT-4.1 → 1,047,576
- o1, o3 → 200,000
- Gemini models → 1,000,000
- Default fallback → 128,000 (already correct)

### Compaction settings (existing)
Already in `backend/crates/qbit-settings/src/schema.rs`:

```rust
pub struct ContextSettings {
    pub enabled: bool,                   // Keep: enables/disables compaction
    pub compaction_threshold: f64,       // Keep: trigger threshold (default 0.80)
    pub protected_turns: usize,          // Deprecate: ignored when compaction enabled
    pub cooldown_seconds: u64,           // Deprecate: ignored when compaction enabled
}
```

Also mirrored in `backend/crates/qbit-context/src/context_manager.rs`:
```rust
pub struct ContextManagerConfig {
    pub enabled: bool,
    pub compaction_threshold: f64,
    pub protected_turns: usize,
    pub cooldown_seconds: u64,
}
```

No new config section needed for compaction basics.

### Summarizer model
Add to settings schema (similar pattern to other agent-specific models):

```rust
pub struct AiSettings {
    // ... existing fields ...
    
    /// Model to use for the summarizer agent (defaults to main model)
    #[serde(default)]
    pub summarizer_model: Option<String>,
}
```

TOML usage:
```toml
[ai]
summarizer_model = "claude-sonnet-4-20250514"
```

If not specified, uses the session's current model.

---

## Session state additions

Add to session state (in-memory, per-session):
```rust
struct CompactionState {
    /// Whether compaction has been attempted this turn
    attempted_this_turn: bool,
    /// Number of compactions performed this session (for diagnostics)
    compaction_count: u32,
    /// Last known input token count from provider
    last_input_tokens: Option<u64>,
}
```

Reset `attempted_this_turn` to `false`:
- At the start of each new turn.
- After a successful hard reset (new conversation starts).
- On manual retry action from UI.

---

## Work breakdown (files to change)

### Backend

| File | Changes |
|------|---------|
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Remove pruning calls; add compaction trigger check between turns; add `CompactionState` tracking; enforce "once per turn" attempt policy; clear `messages` on hard reset |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Append `AiEvent` JSONL to transcript file; add ability to override/append summary section to system prompt after compaction |
| `backend/crates/qbit-ai/src/system_prompt.rs` | Add helper to append continuation/summary section to system prompt |
| `backend/crates/qbit-core/src/events.rs` | Add `AiEvent::ContextCompacted`; add `AiEvent::ContextCompactionFailed`; remove `AiEvent::ContextPruned` |
| `backend/crates/qbit-context/src/context_manager.rs` | Remove pruning API; update to use compaction trigger logic; remove imports from `context_pruner` |
| `backend/crates/qbit-context/src/context_pruner.rs` | **Delete entirely** |
| `backend/crates/qbit-context/src/lib.rs` | Remove `context_pruner` module export; remove `ContextPruner`, `ContextPrunerConfig`, `PruneResult`, `SemanticScore`, `ContextPrunedInfo` re-exports |
| `backend/crates/qbit-context/src/token_budget.rs` | Extend `ModelContextLimits` with GPT, Gemini, o-series models; update `for_model()` match arms |
| `backend/crates/qbit/src/ai/commands/summarizer.rs` | **New file** - modeled after `commit_writer.rs` |
| `backend/crates/qbit/src/ai/commands/mod.rs` | Export new `summarizer` module |
| `backend/crates/qbit-settings/src/schema.rs` | Add `summarizer_model` to `AiSettings`; add deprecation comments to `protected_turns` and `cooldown_seconds` |
| `backend/crates/qbit-cli-output/src/lib.rs` | Update to handle `ContextCompacted`/`ContextCompactionFailed` instead of `ContextPruned` |
| `backend/crates/qbit-sidecar/src/capture.rs` | Update event filter to handle new compaction events |
| `backend/crates/qbit/tests/ai_events_characterization.rs` | Update serialization tests for new events; remove `ContextPruned` tests |

### Frontend

| File | Changes |
|------|---------|
| `frontend/lib/ai.ts` | Add TypeScript types for `context_compacted` and `context_compaction_failed`; remove `context_pruned` type (lines 289-295) |
| `frontend/hooks/useAiEvents.ts` | Replace `context_pruned` handler (lines 405-418) with `context_compacted` and `context_compaction_failed` handlers; preserve `state.setContextMetrics()` pattern |
| `frontend/store/` | Add `compactionCount` to context metrics; add `retryCompaction` action |
| Timeline component(s) | Render compaction status timeline item ("Compacting conversation…") |
| Input component(s) | Add "Retry compaction" button/action; disable input during compaction; show "Start New Session" when context_exceeded |

---

## Testing / verification

### Unit tests
- Transcript writer:
  - appends valid JSONL lines (raw `AiEvent` JSON)
  - preserves ordering under concurrent emission
- Summarizer input builder:
  - deterministic formatting
  - includes tool calls/results and completed responses
  - includes HITL events
- Trigger logic:
  - uses provider `input_tokens` from `TokenUsage` when available
  - falls back to `char/4`
  - looks up correct model context limit via `TokenBudgetConfig::for_model()`
  - applies `compaction_threshold` correctly
- Compaction state:
  - `attempted_this_turn` prevents second attempt
  - resets correctly on new turn / hard reset / manual retry

### Integration tests
- Simulate long conversation:
  - verify compaction triggers at correct threshold for model
  - verify hard reset occurs (messages cleared)
  - verify next system prompt ends with `<summary>`
  - verify UI receives compaction events
- Edge cases:
  - No token usage → heuristic trigger
  - Summarizer failure → `ContextCompactionFailed`, no auto-retry same turn
  - Manual retry → resets tracker, allows new attempt
  - Concurrent events → safe append

### Test file locations
- Unit tests: `mod tests` blocks within each modified file
- Integration tests: `backend/crates/qbit/tests/` (add `context_compaction.rs`)
- Event serialization: Update existing `ai_events_characterization.rs`

---

## Eval (do last)
This needs to be an eval and should be done **after implementation is complete**:
- Measure summary quality (task continuity, factuality, missing constraints).
- Measure compaction frequency and whether safety margin prevents overflow.
- Validate that "no protected turns" is acceptable in practice.
- Validate model context limits are accurate across providers.
- Decide whether to:
  - introduce normalized transcript schema,
  - split into dedicated crates,
  - or improve event capture centralization.

---

## Implementation order (suggested)

1. **Transcript writer** - Low risk, additive. Start capturing events to JSONL.
2. **Summarizer agent** - Isolated module, can be tested independently.
3. **Summarizer input builder** - Reads transcript, formats for summarizer.
4. **Compaction trigger logic** - Update `ContextManager` to detect threshold using `TokenUsage`.
5. **Hard reset mechanism** - Wire up in `agentic_loop.rs` (clear messages, update system prompt).
6. **Events + UI** - Add new events, update frontend.
7. **Remove pruning** - Delete `context_pruner.rs` and related code.
8. **Eval** - Measure quality and tune thresholds.

---

## Risk assessment

| Risk | Mitigation |
|------|------------|
| Summary loses critical context | Eval phase will measure; can adjust summarizer prompt |
| Compaction too frequent | Tune `compaction_threshold`; default 0.80 provides 20% headroom |
| Compaction too slow (blocks UI) | Show progress indicator; summarizer uses fast model by default |
| Provider doesn't return usage | Fallback to char/4 heuristic |
| Transcript grows unbounded | Existing session retention settings apply |
| Breaking change for existing sessions | Deprecate old settings gracefully; transcript preserves history |
