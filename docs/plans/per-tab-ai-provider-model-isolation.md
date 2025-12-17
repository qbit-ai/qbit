# Per-Tab AI Provider/Model Isolation

## Goal
Enable each tab to have its own isolated AI session with independent provider/model selection. New tabs inherit global defaults, and per-tab settings persist across app restarts.

## Scope
- **Provider + Model only** (not approval thresholds or context settings)
- Inherit global defaults from `settings.toml` for new tabs
- Persist per-tab provider/model with session data

---

## Implementation Plan

### Phase 1: Backend - Multi-Session AI State

**1.1 Update `AiState` to store multiple bridges**

File: `src-tauri/src/ai/commands/mod.rs`

```rust
// Change from:
pub struct AiState {
    pub bridge: Arc<RwLock<Option<AgentBridge>>>,
    pub runtime: Arc<RwLock<Option<Arc<dyn QbitRuntime>>>>,
}

// To:
pub struct AiState {
    pub bridges: Arc<RwLock<HashMap<String, AgentBridge>>>,
    pub runtime: Arc<RwLock<Option<Arc<dyn QbitRuntime>>>>,
}
```

Add helper methods:
- `get_session_bridge(session_id)` - get bridge for specific session
- `with_session_bridge(session_id, closure)` - execute closure with session's bridge

**1.2 Add new Tauri commands**

File: `src-tauri/src/ai/commands/core.rs`

New commands:
- `init_ai_session(session_id, config)` - initialize AI for specific session
- `shutdown_ai_session(session_id)` - cleanup session's AI agent
- `is_ai_session_initialized(session_id)` - check if session has AI

**1.3 Update existing commands to accept `session_id`**

Commands to modify:
- `send_ai_prompt` - add `session_id` parameter
- `clear_ai_conversation` - add `session_id` parameter
- `get_ai_conversation_length` - add `session_id` parameter
- All HITL commands (approve/deny tool calls)
- Context management commands

**1.4 Register new commands**

File: `src-tauri/src/lib.rs`

Add new commands to `generate_handler![]` macro.

---

### Phase 2: Frontend Store - Per-Session AI Config

**2.1 Move `AiConfig` into `Session`**

File: `src/store/index.ts`

```typescript
// Update Session interface:
export interface Session {
  id: string;
  name: string;
  workingDirectory: string;
  createdAt: string;
  mode: SessionMode;
  inputMode?: InputMode;
  customName?: string;
  processName?: string;
  // NEW: per-session AI config
  aiConfig: AiConfig;
}
```

**2.2 Update store actions**

Replace global `setAiConfig` with:
- `setSessionAiConfig(sessionId, config)` - update specific session's AI config
- Update selectors to get AI config from session

**2.3 Deprecate global `aiConfig`**

Keep temporarily for migration, then remove after all call sites updated.

---

### Phase 3: Frontend - Session-Specific Initialization

**3.1 Update AI library functions**

File: `src/lib/ai.ts`

Add session-specific wrappers:
```typescript
export async function initAiSession(sessionId: string, config: ProviderConfig): Promise<void>
export async function shutdownAiSession(sessionId: string): Promise<void>
export async function isAiSessionInitialized(sessionId: string): Promise<boolean>
```

Update existing functions to require `sessionId`:
```typescript
export async function sendAiPrompt(sessionId: string, prompt: string, context?: PromptContext): Promise<string>
```

**3.2 Update App.tsx initialization**

File: `src/App.tsx`

Change flow:
1. Create PTY session → get `session.id`
2. Initialize AI for that session with global defaults
3. Same pattern for `handleNewTab()`

**3.3 Update `useAiEvents` hook**

File: `src/hooks/useAiEvents.ts`

Events should include `session_id` from backend. Route events to correct session.

---

### Phase 4: UI - Per-Session Model Selector

**4.1 Update StatusBar**

File: `src/components/StatusBar/StatusBar.tsx`

- Accept `sessionId` prop
- Read AI config from `sessions[sessionId].aiConfig` (not global)
- Model switch calls `initAiSession(sessionId, newConfig)`

**4.2 Update any other model selection UI**

Ensure all model selection UI operates on current session only.

---

### Phase 5: Session Persistence

**5.1 Include provider/model in session save**

File: `src-tauri/src/session/archive.rs`

`SessionArchiveMetadata` already has `model` and `provider` fields. Ensure they're populated when saving.

**5.2 Restore provider/model on session restore**

File: `src-tauri/src/ai/commands/session.rs`

When restoring a session:
1. Load saved provider/model
2. Fall back to global defaults if missing
3. Re-initialize AI with restored config

---

### Phase 6: Cleanup on Tab Close

**6.1 Shutdown AI when tab closes**

In `handleCloseTab()`:
1. Call `shutdownAiSession(sessionId)` before `ptyDestroy()`
2. Backend removes bridge from `HashMap`
3. Frontend already cleans up session state

---

## Files to Modify

### Backend (Rust)
| File | Changes |
|------|---------|
| `src-tauri/src/ai/commands/mod.rs` | `AiState` with `HashMap<String, AgentBridge>`, helper methods |
| `src-tauri/src/ai/commands/core.rs` | New session commands, update existing to take `session_id` |
| `src-tauri/src/ai/commands/session.rs` | Session restore with provider/model |
| `src-tauri/src/ai/commands/hitl.rs` | Add `session_id` to HITL commands |
| `src-tauri/src/ai/commands/context.rs` | Add `session_id` to context commands |
| `src-tauri/src/lib.rs` | Register new commands |

### Frontend (TypeScript)
| File | Changes |
|------|---------|
| `src/store/index.ts` | Move `AiConfig` into `Session`, update actions |
| `src/lib/ai.ts` | Session-specific API wrappers |
| `src/hooks/useAiEvents.ts` | Route events by `session_id` |
| `src/components/StatusBar/StatusBar.tsx` | Per-session model selector |
| `src/App.tsx` | Per-session initialization flow |
| `src/components/UnifiedInput/AgentInput.tsx` | Pass `sessionId` to `sendAiPrompt` |

---

## Implementation Order

1. **Backend first**: Change `AiState` to support multiple bridges
2. **New commands**: Add `init_ai_session`, `shutdown_ai_session`
3. **Frontend store**: Move `AiConfig` into `Session`
4. **Update callers**: Frontend code to use session-specific APIs
5. **UI updates**: StatusBar and other model selection UI
6. **Session persistence**: Save/restore provider/model
7. **Cleanup**: Remove deprecated global state

---

## Migration Notes

- Existing sessions without provider/model default to global settings
- Temporary backwards compatibility: keep global bridge fallback during migration
- No breaking changes to settings.toml format
