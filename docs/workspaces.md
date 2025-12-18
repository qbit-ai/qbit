# Workspaces

This document explains how workspaces work in Qbit, when they are auto-updated, and the architecture behind workspace synchronization.

## Overview

A **workspace** in Qbit represents the root directory that the AI agent uses as a base for all file operations. When the AI agent reads, writes, or searches files, paths are resolved relative to the workspace, and operations outside the workspace are blocked for security.

## Architecture

Qbit uses two types of AI bridges:

1. **Legacy Bridge** (`AiState.bridge`) - A single shared bridge for backwards compatibility
2. **Session Bridges** (`AiState.bridges`) - A HashMap of session-specific bridges keyed by `session_id`

Each bridge maintains its own workspace path via `Arc<RwLock<PathBuf>>`. This allows different terminal sessions (tabs) to have independent workspaces.

```
Frontend Tab 1 (session: abc123)    Frontend Tab 2 (session: xyz789)
        |                                    |
        v                                    v
   AgentBridge                          AgentBridge
   (workspace: /Code/project-a)         (workspace: /Code/project-b)
        |                                    |
        +-----> ToolRegistry                 +-----> ToolRegistry
                (workspace: /Code/project-a)         (workspace: /Code/project-b)
```

## When Workspaces Auto-Update

Workspaces are automatically synchronized when:

### 1. Directory Changed Events (Primary Mechanism)

When the user changes directories in the terminal (e.g., `cd /new/path`), the following happens:

```
Terminal Shell
    |
    v (OSC 7 escape sequence)
PTY Parser
    |
    v (emit)
"directory_changed" Event { session_id, path }
    |
    v (frontend listener)
useTauriEvents.ts
    |
    +---> updateWorkingDirectory(session_id, path)  // Updates UI store
    |
    +---> updateAiWorkspace(path, session_id)       // Updates backend bridges
              |
              v
        update_ai_workspace command (Tauri)
              |
              +---> Session bridge.set_workspace(path)
              +---> Legacy bridge.set_workspace(path) (fallback)
              +---> Sidecar state reinitialization
```

**Key files:**
- `src/hooks/useTauriEvents.ts:190-205` - Frontend listener
- `src/lib/ai.ts:354-358` - `updateAiWorkspace()` invoke wrapper
- `src-tauri/src/ai/commands/config.rs:190-240` - Backend command

### 2. Initial Session Creation

When a new PTY session is created, the workspace is set to the session's `working_directory`:

```
createSession()
    |
    v
PTY spawn with initial directory
    |
    v
init_ai_session(session_id, workspace, ...)
    |
    v
AgentBridge::new_*(..., workspace, ...)
```

**Key files:**
- `src/App.tsx:145-272` - Session initialization
- `src-tauri/src/ai/commands/core.rs:503-553` - `init_ai_session` command

## Workspace Path Resolution in Tools

When a tool (like `read_file`) receives a path argument, it goes through `resolve_path()`:

```rust
// src-tauri/src/tools/file_ops.rs:19-88
fn resolve_path(path_str: &str, workspace: &Path) -> Result<PathBuf, String> {
    // If path is absolute, use as-is
    // If relative, join with workspace
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    // Canonicalize and verify path is within workspace
    // Returns error if path escapes workspace
}
```

## Common Issues

### Workspace Mismatch (Session ID Not Passed)

**Symptom:** AI agent reports "Path is outside workspace" even though the path should be valid and the workspace indicator shows the correct directory.

**Cause:** The `updateAiWorkspace()` call didn't include the `session_id` parameter, so only the legacy bridge was updated, not the session-specific bridge.

**Fix:** Ensure `updateAiWorkspace()` is called with the `session_id` parameter:
```typescript
await updateAiWorkspace(path, session_id);
```

### Workspace Update Skipped (Wrong Initialization Check)

**Symptom:** Directory changes are detected but workspace sync is skipped.

**Cause:** The code was checking `isAiInitialized()` (legacy bridge) instead of `isAiSessionInitialized(session_id)` (session-specific bridge).

**Fix:** Use the session-specific initialization check:
```typescript
// Wrong: checks legacy bridge
const initialized = await isAiInitialized();

// Correct: checks session-specific bridge
const initialized = await isAiSessionInitialized(session_id);
```

### Stale Workspace After Model Switch

**Symptom:** After switching AI models, the workspace reverts to an old directory.

**Cause:** Model switching creates a new `AgentBridge`, which may not preserve the previous workspace.

**Fix:** Re-initialize the workspace after model switching by re-emitting the `directory_changed` event or explicitly calling `updateAiWorkspace()`.

## Debugging

Enable workspace sync logging:
```bash
RUST_LOG=debug cargo run
# Look for: [cwd-sync] ...
```

Key log messages:
- `[cwd-sync] update_ai_workspace called with: /path, session_id: Some("abc123")`
- `[cwd-sync] Session abc123 workspace successfully updated`
- `[cwd-sync] Legacy bridge workspace also updated`
- `[cwd-sync] No session bridge found for session_id: xyz` (warning)

## Related Files

| File | Purpose |
|------|---------|
| `src-tauri/src/tools/file_ops.rs` | File tools with path resolution |
| `src-tauri/src/tools/directory_ops.rs` | Directory tools with path resolution |
| `src-tauri/src/ai/agent_bridge.rs:477-485` | `set_workspace()` method |
| `src-tauri/src/ai/commands/config.rs:190-240` | `update_ai_workspace` command |
| `src/hooks/useTauriEvents.ts:188-205` | Frontend directory change handler |
| `src/lib/ai.ts:354-358` | Frontend `updateAiWorkspace()` wrapper |
