# Q-138: File links depend on current directory

File links in agent responses are resolved relative to the current working directory instead of using absolute paths. This causes broken links when the user navigates to a different directory.

## Root Cause

`workingDirectory` used to resolve file paths in agent messages comes from the **live session state** (`state.sessions[sessionId].workingDirectory`), which updates whenever the user runs `cd`. Historical agent messages all receive the current session `workingDirectory`, not the directory that was active when the message was created.

`CommandBlock` already snapshots `workingDirectory` at creation time, so its file links are stable. `AgentMessage` does not.

## TODO

- [x] Add `workingDirectory` field to `AgentMessage` interface (`frontend/store/index.ts`)
- [x] Capture `workingDirectory` when agent messages are finalized in the store
- [x] Use per-message `workingDirectory` in `UnifiedBlock` for `agent_message` blocks
- [ ] Verify tests pass
