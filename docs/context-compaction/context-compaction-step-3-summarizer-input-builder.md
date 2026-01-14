# Step 3: Summarizer Input Builder

**Status:** ✅ Completed

**Goal:** Create a module that reads transcript JSON files and converts them to a human-readable format suitable for the summarizer agent.

**Outcome:** We can call `build_summarizer_input(base_dir, session_id)` to get a formatted conversation transcript ready for summarization.

---

## Implementation Summary

### File Format (from Step 1)

| Aspect | Value |
|--------|-------|
| File Format | Pretty-printed JSON array |
| File Path | `~/.qbit/transcripts/{session_id}/transcript.json` |
| Env Variable | `VT_TRANSCRIPT_DIR` |
| Timestamp | ISO 8601 `DateTime<Utc>` |

### Example Transcript File

```json
[
  {
    "_timestamp": "2026-01-13T10:30:45.123456Z",
    "type": "started",
    "turn_id": "turn-1"
  },
  {
    "_timestamp": "2026-01-13T10:30:45.234567Z",
    "type": "user_message",
    "content": "Read the main.rs file"
  },
  {
    "_timestamp": "2026-01-13T10:30:46.000000Z",
    "type": "tool_request",
    "tool_name": "read_file",
    "args": {"path": "/src/main.rs"},
    "request_id": "req-1",
    "source": {"type": "main"}
  }
]
```

---

## Prerequisites

- Step 1 completed (TranscriptWriter creates JSON files) ✅
- Step 2 completed (Summarizer expects formatted conversation text) ✅

## Files Modified

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-ai/src/transcript.rs` | Modified | Added reader and formatter functions |
| `backend/crates/qbit-ai/src/lib.rs` | Modified | Updated public exports |

---

## API Reference

### Types

```rust
/// A transcript event with its timestamp (public version for consumers).
#[derive(Debug, Clone)]
pub struct TranscriptEvent {
    pub timestamp: DateTime<Utc>,
    pub event: AiEvent,
}
```

### Functions

```rust
/// Read all events from a transcript file.
/// Returns events in chronological order. Returns an error if the file doesn't exist.
pub fn read_transcript(base_dir: &Path, session_id: &str) -> anyhow::Result<Vec<TranscriptEvent>>

/// Format transcript events for the summarizer.
/// Produces a human-readable text format. Excludes streaming events (TextDelta).
pub fn format_for_summarizer(events: &[TranscriptEvent]) -> String

/// Build summarizer input from a session's transcript.
/// Main entry point - reads the transcript file and formats it.
pub fn build_summarizer_input(base_dir: &Path, session_id: &str) -> anyhow::Result<String>

/// Save summarizer input to an artifact file.
/// Returns the path to the saved file (e.g., `summarizer-input-{session_id}-{timestamp}.md`).
pub fn save_summarizer_input(base_dir: &Path, session_id: &str, content: &str) -> anyhow::Result<PathBuf>

/// Save a summary to an artifact file.
/// Returns the path to the saved file (e.g., `summary-{session_id}-{timestamp}.md`).
pub fn save_summary(base_dir: &Path, session_id: &str, summary: &str) -> anyhow::Result<PathBuf>
```

### Public Exports (lib.rs)

```rust
pub use transcript::{
    build_summarizer_input, format_for_summarizer, read_transcript, save_summary,
    save_summarizer_input, transcript_path, TranscriptEvent, TranscriptWriter,
};
```

---

## Formatter Output Format

The `format_for_summarizer()` function produces human-readable output with turn-based structure:

```
[turn 001] USER:
Read the main.rs file

[turn 001] TOOL_REQUEST (tool=read_file, id=req-1):
{
  "path": "/src/main.rs"
}

[turn 001] TOOL_RESULT (tool=read_file, success=true):
fn main() { println!("hello"); }

[turn 001] ASSISTANT (200 in / 100 out tokens):
I found the main function. It prints "hello" to the console.

[turn 002] USER:
Now add error handling

...
```

### Events Included

| Event Type | Format |
|------------|--------|
| `UserMessage` | `[turn XXX] USER:\n{content}` |
| `Completed` | `[turn XXX] ASSISTANT ({in} in / {out} out tokens):\n{response}` |
| `ToolRequest` | `[turn XXX] TOOL_REQUEST (tool={name}, id={id}):\n{args}` |
| `ToolResult` | `[turn XXX] TOOL_RESULT (tool={name}, success={bool}):\n{result}` |
| `ToolApprovalRequest` | `[turn XXX] TOOL_APPROVAL_REQUEST (tool={name}, risk={level}):\n{args}` |
| `ToolAutoApproved` | `[turn XXX] TOOL_AUTO_APPROVED (tool={name}): {reason}` |
| `ToolDenied` | `[turn XXX] TOOL_DENIED (tool={name}): {reason}` |
| `Error` | `[turn XXX] ERROR ({type}): {message}` |
| `SubAgentStarted` | `[turn XXX] SUB_AGENT_STARTED (agent={name}):\n{task}` |
| `SubAgentCompleted` | `[turn XXX] SUB_AGENT_COMPLETED (agent={name}):\n{response}` |
| `SubAgentError` | `[turn XXX] SUB_AGENT_ERROR (agent={name}): {error}` |

### Events Excluded (not useful for summarization)

- `TextDelta` - Streaming chunks (redundant with `Completed`)
- `Reasoning` - Internal thinking
- `Started` - Only used for turn counting
- `ContextPruned`, `ContextWarning`, `ToolResponseTruncated` - Context management
- `LoopWarning`, `LoopBlocked`, `MaxIterationsReached` - Loop protection
- `SubAgentToolRequest`, `SubAgentToolResult` - Too verbose
- `WorkflowStarted`, `WorkflowStepStarted`, `WorkflowStepCompleted`, `WorkflowCompleted`, `WorkflowError` - Workflow lifecycle
- `PlanUpdated` - Planning internals
- `ServerToolStarted`, `WebSearchResult`, `WebFetchResult` - Server tool internals
- `Warning` - Generic warnings

### Truncation

- Tool results > 2000 chars are truncated with `...[truncated, N chars total]`
- Sub-agent responses > 3000 chars are truncated with `...[truncated]`

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit-ai transcript
```

**Result:** 28 tests pass

### Test Coverage

| Module | Tests |
|--------|-------|
| `reader_tests` | 5 tests (file reading, missing files, empty files, timestamps) |
| `formatter_tests` | 11 tests (all event types, truncation, turn tracking) |
| `artifact_tests` | 4 tests (file saving, directory creation) |
| `integration_tests` | 1 test (end-to-end write → read → format) |
| `tests` (existing) | 7 tests (TranscriptWriter) |

### Manual Verification
1. Start qbit, have a conversation with tool calls
2. Find the transcript file in `~/.qbit/transcripts/{session_id}/transcript.json`
3. Call `build_summarizer_input()` on it (via test or debug command)
4. Verify output is readable and contains expected events

---

## Definition of Done

- [x] `read_transcript()` reads JSON array files correctly
- [x] `format_for_summarizer()` produces human-readable output
- [x] Turn numbers are tracked correctly
- [x] TextDelta events are excluded (only final Completed)
- [x] Tool calls and results are included with proper formatting
- [x] HITL events (approval requests, auto-approved, denied) are included
- [x] Very long tool results are truncated
- [x] `save_summarizer_input()` and `save_summary()` work correctly
- [x] All tests pass (28 tests)
- [x] Existing tests still pass
- [x] Clippy passes with no warnings

---

## Notes

- The formatter intentionally excludes streaming events (TextDelta) to avoid duplication
- Sub-agent tool calls are excluded (too verbose) but sub-agent start/complete are included
- Long tool results are truncated to keep the input manageable
- Artifact files use timestamps to allow multiple compactions in a session
- The `TranscriptEvent` struct uses `DateTime<Utc>` for timestamps (ISO 8601 format)
