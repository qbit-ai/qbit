# TDD Execution Plan: Preserve In-Progress Messages + Retry on API Errors

## Objective
Fix agent-turn failure handling so that API/provider errors (especially Anthropic Vertex) do **not** lose in-progress assistant output, while adding safe retry behavior for transient API failures.

## Scope
- Preserve partial assistant output on turn failure (UI + persisted history)
- Eliminate duplicate terminal error messages for one failure
- Add bounded retry/backoff for transient provider/API failures
- Add regression tests first (TDD)

## Out of Scope
- Broad refactors unrelated to failure handling
- New user-facing settings unless required
- Provider-agnostic retry overhaul beyond this path

---

## Phase 0 — Baseline & Repro
1. Reproduce failure path with Anthropic Vertex-like error (429 / RESOURCE_EXHAUSTED).
2. Confirm current behavior:
   - in-progress text disappears from timeline
   - no partial assistant message in history
   - duplicate error messages may appear

---

## Phase 1 — Frontend TDD (message preservation on error)
### Red (tests first)
Add failing tests in `frontend/hooks/useAiEvents.test.ts`:
1. `started -> text_delta -> error` preserves partial assistant content in timeline/history.
2. Error message is added while partial assistant output remains.
3. Streaming state is cleared **after** partial content is finalized.

### Green (minimal implementation)
Update `frontend/hooks/ai-events/core-handlers.ts` (`handleError`):
1. Flush pending deltas for session.
2. Build finalized assistant message from current `streamingBlocks`/streaming text.
3. Persist that assistant message if non-empty.
4. Append system error message.
5. Clear transient streaming state.

### Refactor
- Extract helper for "finalize in-progress turn on error" to reduce duplication with completed path.

---

## Phase 2 — Backend TDD (single terminal error + partial persistence)
### Red (tests first)
Add failing tests in `backend/crates/qbit-ai/src/test_utils.rs` and/or `agent_bridge` tests:
1. Stream-start/provider failure emits exactly one terminal error event.
2. If partial response exists before failure, it is persisted to conversation/session history.

### Green (minimal implementation)
1. Ensure only one layer emits terminal `AiEvent::Error` (remove duplicate emission path).
2. Add failure finalization path to persist partial assistant output/history when available.
3. Keep success path behavior unchanged.

### Refactor
- Introduce typed execution error metadata (e.g., already-emitted / retriable classification hints).

---

## Phase 3 — Retry/Backoff TDD (transient API resilience)
### Red (tests first)
Add failing tests for retry policy in `backend/crates/qbit-ai/src/agentic_loop.rs` tests:
1. Retries on transient errors (429, 5xx, timeout/transport) up to max attempts.
2. Succeeds when a subsequent retry succeeds.
3. Does not retry on non-retriable errors (401/403/invalid request).
4. Emits retry warning/progress event for visibility.

### Green (minimal implementation)
1. Wrap `model.stream(request)` with bounded retry loop.
2. Exponential backoff with jitter (small initial delay, capped max delay).
3. Retry classification function (retriable vs non-retriable).
4. Keep final error messaging user-friendly and single-shot.

### Refactor
- Extract pure helpers:
  - `classify_stream_error(...)`
  - `should_retry(...)`
  - `compute_backoff_delay(...)`
- Inject sleep/clock abstraction for deterministic tests.

---

## Phase 4 — Verification
### Targeted test runs
- `just test-fe` (or focused Vitest for `useAiEvents`)
- `just test-rust` (or focused cargo test for `qbit-ai`)

### Full validation
- `just check` (if time permits before merge)

### Manual QA
1. Trigger simulated 429 during streaming.
2. Verify:
   - partial assistant text remains in timeline
   - partial assistant turn is preserved in history
   - one error card/message only
   - transient failures auto-retry and may recover

---

## Acceptance Criteria
- No in-progress assistant content is lost when API errors occur.
- Exactly one terminal error event/message per failed turn.
- Transient API failures are retried with bounded backoff.
- Non-retriable errors fail fast.
- New tests fail before implementation and pass after.

---

## Implementation Order (strict)
1. Frontend preservation tests + fix
2. Backend single-error/persistence tests + fix
3. Retry classification tests + retry implementation
4. Full regression pass
