# Plan: Restore Session Context (state.md, log, patches, artifacts)

## Problem Statement

When a user restores a previous session via SessionBrowser, the **conversation history** is restored to the agent, but the **sidecar context** is lost:
- `state.md` (LLM-managed session state)
- `log.md` (append-only event log)
- `patches/staged/` and `patches/applied/`
- `artifacts/pending/` and `artifacts/applied/`

Currently, `restore_ai_session` starts a **new** sidecar session, which creates fresh empty files instead of preserving the original session's context.

## Current Architecture

### Session Storage Layout
```
~/.qbit/sessions/<session-id>/
  ├── state.md              # LLM-managed state
  ├── log.md                # Event log
  ├── patches/
  │   ├── staged/           # Uncommitted patches
  │   └── applied/          # Applied patches
  └── artifacts/
      ├── pending/          # Pending artifacts
      └── applied/          # Applied artifacts
```

### Current Restore Flow
1. User selects session in SessionBrowser
2. `restoreSession()` in store calls `restoreAiSession(identifier)`
3. Backend `restore_ai_session`:
   - Loads messages from saved session
   - Restores to agent conversation history
   - **Ends any existing sidecar session**
   - **Starts a NEW sidecar session** ← Problem here
4. ContextPanel queries current sidecar session (new, empty)

### ContextPanel Data Flow
```typescript
// ContextPanel.tsx fetches:
const [state, log, staged, applied, artifacts] = await Promise.all([
  getSessionState(sid),      // Uses current sidecar session
  getSessionLog(sid),
  getStagedPatches(sid),
  getAppliedPatches(sid),
  getPendingArtifacts(sid),
]);
```

## Proposed Solution

### Option A: Resume Sidecar Session (Recommended)

Instead of starting a new sidecar session, **resume** the original session.

#### Backend Changes

1. **Add `sidecar_resume_session` command** in `src-tauri/src/sidecar/commands.rs`:
   ```rust
   #[tauri::command]
   pub fn sidecar_resume_session(
       state: State<'_, SidecarState>,
       session_id: String,
   ) -> Result<SessionMeta, String>
   ```
   - Validates session directory exists
   - Sets session as active (updates status to "Active")
   - Updates `updated_at` timestamp
   - Emits `session_started` event

2. **Modify `restore_ai_session`** in `src-tauri/src/ai/commands/session.rs`:
   - Instead of `start_session()`, call `resume_session(original_session_id)`
   - The original session ID should be stored in the saved session metadata

3. **Add session_id to saved session metadata**:
   - When saving a session, include the sidecar session ID
   - This links the AI conversation session to its sidecar context

#### Frontend Changes

1. **Add `resumeSidecarSession()` wrapper** in `src/lib/sidecar.ts`:
   ```typescript
   export async function resumeSidecarSession(sessionId: string): Promise<SessionMeta> {
     return invoke<SessionMeta>("sidecar_resume_session", { sessionId });
   }
   ```

2. **Update `restoreSession()` in store** (optional - backend handles it):
   - After restore, emit event so ContextPanel refreshes

3. **ContextPanel auto-refresh**:
   - Already listens to `session_started` event
   - Will auto-refresh when session is resumed

### Option B: Pass Session ID to ContextPanel

If we want to keep sessions separate, ContextPanel could accept an explicit session ID.

#### Changes

1. **Store restored session ID in Zustand**:
   ```typescript
   // In store
   restoredSidecarSessionId: string | null;
   ```

2. **Update ContextPanel props**:
   ```typescript
   interface ContextPanelProps {
     sessionId?: string;  // Already exists
     // Use restoredSidecarSessionId if set
   }
   ```

3. **ContextPanel queries the restored session**:
   - Use `restoredSidecarSessionId` instead of current sidecar session

**Downside**: More complex, new state to manage, context panel shows read-only historical data.

## Recommended Implementation: Option A

### Step 1: Add sidecar_resume_session Command

**File**: `src-tauri/src/sidecar/commands.rs`

```rust
/// Resume a previous sidecar session by session ID.
/// This makes the session active again, preserving all existing context.
#[tauri::command]
pub fn sidecar_resume_session(
    state: State<'_, SidecarState>,
    app: AppHandle,
    session_id: String,
) -> Result<SessionMeta, String> {
    state.resume_session(&session_id, &app).map_err(|e| e.to_string())
}
```

**File**: `src-tauri/src/sidecar/state.rs`

Add `resume_session` method to `SidecarState`:
```rust
pub fn resume_session(&self, session_id: &str, app: &AppHandle) -> Result<SessionMeta> {
    let mut inner = self.inner.lock().unwrap();

    // Validate session exists
    let session_dir = inner.sessions_dir.join(session_id);
    if !session_dir.exists() {
        return Err(anyhow!("Session {} not found", session_id));
    }

    // Load and update metadata
    let meta_path = session_dir.join("meta.toml");
    let mut meta: SessionMeta = toml::from_str(&std::fs::read_to_string(&meta_path)?)?;
    meta.status = SessionStatus::Active;
    meta.updated_at = Utc::now();
    std::fs::write(&meta_path, toml::to_string_pretty(&meta)?)?;

    // Set as current session
    inner.current_session = Some(session_id.to_string());

    // Emit event
    let _ = app.emit("sidecar-event", SidecarEvent::SessionStarted {
        session_id: session_id.to_string(),
    });

    Ok(meta)
}
```

### Step 2: Modify restore_ai_session

**File**: `src-tauri/src/ai/commands/session.rs`

```rust
#[tauri::command]
pub async fn restore_ai_session(
    state: State<'_, AppState>,
    app: AppHandle,
    identifier: String,
) -> Result<QbitSessionSnapshot, String> {
    // Load the session
    let session = session::load_session(&identifier).await...;

    // Restore messages to agent
    let bridge = state.ai_state.get_bridge()...;
    bridge.restore_session(session.messages.clone()).await;

    // End any existing sidecar session
    let _ = state.sidecar_state.end_session();

    // Resume the original sidecar session if it exists
    if let Some(sidecar_session_id) = &session.sidecar_session_id {
        match state.sidecar_state.resume_session(sidecar_session_id, &app) {
            Ok(_) => tracing::info!("Resumed sidecar session {}", sidecar_session_id),
            Err(e) => {
                tracing::warn!("Could not resume sidecar session: {}", e);
                // Fall back to starting new session
                let initial_request = extract_initial_request(&session.messages);
                state.sidecar_state.start_session(&initial_request)?;
            }
        }
    } else {
        // Legacy session without sidecar ID - start new
        let initial_request = extract_initial_request(&session.messages);
        state.sidecar_state.start_session(&initial_request)?;
    }

    Ok(session)
}
```

### Step 3: Link AI Session to Sidecar Session

When saving an AI session, include the sidecar session ID.

**File**: `src-tauri/src/ai/session.rs` (or wherever sessions are saved)

Ensure the saved session JSON includes:
```json
{
  "sidecar_session_id": "43c1988e-7622-4f41-8cb5-254fb11bb95f",
  "messages": [...],
  ...
}
```

### Step 4: Register New Command

**File**: `src-tauri/src/lib.rs`

Add to `tauri::generate_handler![]`:
```rust
sidecar_resume_session,
```

### Step 5: Frontend Wrapper (Optional)

**File**: `src/lib/sidecar.ts`

```typescript
export async function resumeSidecarSession(sessionId: string): Promise<SessionMeta> {
  return invoke<SessionMeta>("sidecar_resume_session", { sessionId });
}
```

## Testing Plan

1. **Unit Tests**:
   - `resume_session` correctly updates meta.toml
   - `resume_session` emits `session_started` event
   - `restore_ai_session` uses existing sidecar session when available

2. **Integration Tests**:
   - Create session → end session → restore → verify state.md preserved
   - Create session with patches → restore → verify patches visible
   - Create session with artifacts → restore → verify artifacts visible

3. **Manual Testing**:
   - Start new session, do some work
   - Close app or start new session
   - Restore previous session from SessionBrowser
   - Open ContextPanel (Cmd+Shift+C)
   - Verify: state.md, log.md, patches, artifacts are all present

## Migration Notes

- Existing saved sessions won't have `sidecar_session_id`
- Fallback to creating new sidecar session for legacy sessions
- Consider adding migration to backfill sidecar IDs if session directories exist

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/sidecar/state.rs` | Add `resume_session` method |
| `src-tauri/src/sidecar/commands.rs` | Add `sidecar_resume_session` command |
| `src-tauri/src/ai/commands/session.rs` | Modify `restore_ai_session` to resume sidecar |
| `src-tauri/src/ai/session.rs` | Store `sidecar_session_id` in saved sessions |
| `src-tauri/src/lib.rs` | Register new command |
| `src/lib/sidecar.ts` | Add `resumeSidecarSession` wrapper (optional) |

## Alternative Consideration

If resuming sessions causes issues with append-only log or other invariants, consider:
- Making resumed sessions read-only initially
- Appending a "--- Session Resumed ---" marker to log.md
- Creating a new state.md based on the old one
