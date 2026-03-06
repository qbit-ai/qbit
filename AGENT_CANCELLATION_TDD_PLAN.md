# Agent Cancellation Feature — TDD Execution Plan

## Goal
Implement a user-facing **Stop/Cancellation** feature for an in-progress AI agent turn, with complete backend + frontend support, using strict TDD (Red → Green → Refactor).

## Definition of Done
- User can cancel an active turn from the UI.
- Backend cancellation is session-scoped and idempotent.
- A cancelled turn emits a terminal `cancelled` event (not treated as generic error).
- Frontend state is cleaned up correctly on cancel (responding/thinking/tool UI).
- User can immediately submit a new prompt after cancellation.
- Tests cover normal flow, no-op cancel, double-cancel, and race scenarios.

## Scope
### In Scope
- New backend cancel command for session turns.
- Event contract update with `cancelled` event.
- Frontend API wrapper + event handling.
- UnifiedInput Stop behavior and cancellation UX.
- Unit/integration tests across Rust + frontend.

### Out of Scope
- Workflow cancellation redesign (existing `cancel_workflow` remains separate).
- Provider-specific transport cancellation improvements beyond current runtime model.
- Broad refactors unrelated to turn cancellation.

---

## Phase 0 — Baseline & Test Harness Validation
1. Run existing tests to establish baseline:
   - `just test-fe`
   - `just test-rust`
2. Confirm test locations and patterns to extend:
   - Frontend: `frontend/hooks/ai-events/*.test.ts`, `frontend/components/UnifiedInput/*.test.tsx`
   - Rust: module-local tests in updated crates

**Exit criteria:** baseline tests pass and test targets are identified.

---

## Phase 1 — Contract-First TDD (Core Event Schema)
### Red
Add failing tests for new event contract in:
- `backend/crates/qbit-core/src/events.rs`

Tests to add:
- `cancelled_event_json_format`
- `cancelled_event_type_name`

### Green
Implement:
- Add `AiEvent::Cancelled { reason: String }`
- Update `AiEvent::event_type()` mapping to include `"cancelled"`

### Refactor
- Keep payload minimal and stable.
- Ensure serde naming and format consistency with existing event conventions.

**Exit criteria:** event contract tests pass.

---

## Phase 2 — Backend Turn-Cancellation Core (Session-Scoped)
### Red
Add failing Rust tests for cancellation state/lifecycle (new or existing command module tests):
- starting a prompt registers in-flight turn
- cancelling active turn succeeds
- cancelling inactive turn is no-op success
- cancelling twice is idempotent
- completion/error/cancel always clears in-flight tracking
- race: cancel vs completion yields one terminal outcome

Primary files:
- `backend/crates/qbit/src/ai/commands/mod.rs` (AiState extensions)
- `backend/crates/qbit/src/ai/commands/core.rs`

### Green
Implement:
- Session-scoped in-flight tracking in `AiState` (e.g., cancel handle/token per session).
- Wire `send_ai_prompt_session` + `send_ai_prompt_with_attachments` to register/unregister active turn handles.
- Ensure cleanup in all terminal paths (success/error/cancel).

### Refactor
- Centralize registration/cleanup helpers to avoid duplicated logic.
- Keep lock scope short and avoid map-lock during long-running execution.

**Exit criteria:** backend cancellation lifecycle tests pass.

---

## Phase 3 — Backend Command Surface TDD
### Red
Add failing tests for command behavior in `core.rs`/module tests:
- `cancel_ai_prompt_session` returns success when active and emits `Cancelled`
- command is safe when no active turn
- command can be invoked repeatedly without error

### Green
Implement:
- New Tauri command: `cancel_ai_prompt_session(session_id: String)`
- Register command in:
  - `backend/crates/qbit/src/lib.rs` (`generate_handler!`)

### Refactor
- Align logging with existing AI command patterns.
- Ensure command returns predictable, frontend-friendly results.

**Exit criteria:** command tests pass and command is registered.

---

## Phase 4 — Frontend Event Handling TDD
### Red
Add failing tests in frontend event handling:
- `cancelled` event clears `isAgentResponding` and `isAgentThinking`
- pending/active tool UI is cleaned up on cancel
- partial streaming is finalized according to UX decision
- cancel does not create generic error block

Primary files:
- `frontend/hooks/ai-events/registry.ts`
- `frontend/hooks/ai-events/core-handlers.ts`
- tests in `frontend/hooks/ai-events/registry.test.ts` (and/or new core handler test file)

### Green
Implement:
- Add `cancelled` variant to `AiEvent` union in `frontend/lib/ai.ts`
- Register `cancelled` handler in registry
- Add cancellation handler logic in core handlers

### Refactor
- Reuse existing terminal-state cleanup logic (avoid diverging completion/error/cancel code paths).

**Exit criteria:** frontend event handler tests pass.

---

## Phase 5 — Frontend Stop UX TDD
### Red
Add failing component tests in:
- `frontend/components/UnifiedInput/UnifiedInput.callbacks.test.tsx`

Test cases:
- Stop action appears/enabled while agent is busy
- clicking Stop calls cancel API with correct `sessionId`
- repeated clicks are guarded while cancellation is pending
- cancel flow resets submit state to allow next prompt

### Green
Implement:
- Add API wrapper in `frontend/lib/ai.ts`:
  - `cancelPromptSession(sessionId: string)` → invoke `cancel_ai_prompt_session`
- Update `UnifiedInput.tsx`:
  - show Stop behavior when busy in agent/auto mode
  - trigger cancel command
  - avoid showing cancellation as an error toast

### Refactor
- Simplify `isSubmitting` reset logic to include cancel path reliably.

**Exit criteria:** UnifiedInput cancellation tests pass.

---

## Phase 6 — Integration & Regression Verification
1. Run targeted tests for touched modules.
2. Run full verification:
   - `just test-fe`
   - `just test-rust`
3. Manual smoke:
   - send prompt → stream starts → click Stop → UI resets
   - submit new prompt immediately after cancel
   - cancel during tool-approval wait
   - cancel when idle (no error)

**Exit criteria:** all relevant tests pass and manual smoke scenarios succeed.

---

## Suggested Commit Strategy (Optional)
1. `test(core): add cancelled AI event contract tests`
2. `feat(core): add cancelled AI event type`
3. `test(ai): add session turn cancellation lifecycle tests`
4. `feat(ai): add cancel_ai_prompt_session and in-flight turn tracking`
5. `test(frontend): add cancelled event and stop button tests`
6. `feat(frontend): wire stop action and cancelled event handling`
7. `chore: final refactor + test stabilization`

---

## Risk Checklist
- [ ] Race conditions between cancel and natural completion
- [ ] Stale pending approvals after cancel
- [ ] Session map lock contention during long-running execution
- [ ] Frontend dedupe/order issues with new event type
- [ ] Misclassifying user cancel as generic error

## Final Acceptance Checklist
- [ ] Backend command exists and is registered
- [ ] `cancelled` event is part of Rust + TS contracts
- [ ] Frontend can trigger cancel from input area
- [ ] Cancel reliably terminates active turn state
- [ ] New prompt works immediately after cancel
- [ ] Tests added and passing
