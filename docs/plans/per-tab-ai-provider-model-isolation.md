# Per-Tab AI Provider/Model Isolation

## Goal
Enable each tab to have its own isolated AI session with independent provider/model selection. New tabs inherit global defaults, and per-tab settings persist across app restarts.

## Scope
- **Provider + Model only** (not approval thresholds or context settings)
- Inherit global defaults from `settings.toml` for new tabs
- Persist per-tab provider/model with session data

---

## Status: COMPLETED

All phases have been implemented.

### Completed

- [x] **Phase 1: Backend - Multi-Session AI State**
  - `AiState` uses `HashMap<String, AgentBridge>` keyed by session ID
  - Helper methods for session-specific bridge access
  - New commands: `init_ai_session`, `shutdown_ai_session`, `is_ai_session_initialized`
  - Session-specific: `send_ai_prompt_session`, `clear_ai_conversation_session`, `get_ai_conversation_length_session`

- [x] **Phase 2: Frontend Store - Per-Session AI Config**
  - Added `aiConfig?: AiConfig` to `Session` interface
  - Added `setSessionAiConfig(sessionId, config)` action
  - Added `useSessionAiConfig(sessionId)` selector

- [x] **Phase 3: Frontend - Session-Specific Initialization**
  - Session-specific wrappers in `frontend/lib/ai.ts`
  - `App.tsx` initializes AI per-session on tab creation
  - `UnifiedInput.tsx` uses `sendPromptSession(sessionId, ...)`

- [x] **Phase 4: UI - Per-Session Model Selector**
  - `StatusBar.tsx` uses `useSessionAiConfig(sessionId)`
  - Model switching calls `initAiSession(sessionId, config)`

- [x] **Phase 5: Session Persistence**
  - Sessions already save provider/model via `QbitSessionManager`
  - Frontend `restoreSession()` now restores provider/model from session data
  - Falls back to global defaults if provider/model missing or API keys unavailable

- [x] **Phase 6: Cleanup on Tab Close**
  - `TabBar.tsx` calls `shutdownAiSession(sessionId)` before `ptyDestroy()`

- [x] **Event Routing with session_id**
  - `RuntimeEvent::Ai` now includes `session_id` field
  - `AgentBridge` stores `event_session_id` for proper routing
  - `TauriRuntime` serializes events with `session_id` using `#[serde(flatten)]`
  - Frontend `AiEvent` type includes `session_id`
  - `useAiEvents` hook routes events using `event.session_id`
  - Events now route to correct session even when user switches tabs during streaming

---

## Known Limitations

### Auto-Switch When Provider Disabled

The settings-updated handler in `StatusBar.tsx` was simplified and no longer auto-switches to an alternative provider when the current provider is disabled in settings. Users must manually select a new provider.

---

## Implementation Details (For Reference)

### Files Modified

#### Backend (Rust)
| File | Changes |
|------|---------|
| `backend/src/ai/commands/mod.rs` | `AiState` with `HashMap<String, AgentBridge>`, helper methods |
| `backend/src/ai/commands/core.rs` | New session commands, `set_event_session_id()` call |
| `backend/src/ai/agent_bridge.rs` | Added `event_session_id` field, `set_event_session_id()`, updated `emit_event()` |
| `backend/src/runtime/mod.rs` | `RuntimeEvent::Ai` now struct variant with `session_id` |
| `backend/src/runtime/tauri.rs` | `AiEventPayload` with `session_id` and flattened event |
| `backend/src/cli/output.rs` | Updated to use new `RuntimeEvent::Ai` struct variant |
| `backend/src/ai/mod.rs` | Export new commands |
| `backend/src/lib.rs` | Register new commands in `generate_handler![]` |

#### Frontend (TypeScript)
| File | Changes |
|------|---------|
| `frontend/store/index.ts` | `aiConfig` in `Session`, `setSessionAiConfig`, `useSessionAiConfig`, `restoreSession()` restores AI config |
| `frontend/lib/ai.ts` | Session-specific API wrappers, `AiEvent` type with `session_id` |
| `frontend/hooks/useAiEvents.ts` | Routes events using `event.session_id` instead of `activeSessionId` |
| `frontend/components/StatusBar/StatusBar.tsx` | Per-session model selector using `initAiSession` |
| `frontend/components/TabBar/TabBar.tsx` | AI shutdown on tab close |
| `frontend/components/UnifiedInput/UnifiedInput.tsx` | Uses `sendPromptSession` |
| `frontend/App.tsx` | Per-session AI initialization flow |
