# Step 1: Transcript Writer

**Goal:** Capture all `AiEvent` emissions to a JSONL file for each session. This is additive and low-risk - existing functionality remains unchanged.

**Outcome:** After this step, every AI session produces a `transcript-{session_id}.jsonl` file in `VT_SESSION_DIR` containing all events.

---

## Implementation Notes (Post-Implementation)

This section documents how the actual implementation differed from the original plan below.

### Changes from Original Plan

| Aspect | Original Plan | Actual Implementation |
|--------|---------------|----------------------|
| **File Format** | JSONL (one JSON object per line) | Pretty-printed JSON array (easier to read) |
| **File Path** | `~/.qbit/sessions/transcript-{session_id}.jsonl` | `~/.qbit/transcripts/{session_id}/transcript.json` |
| **Environment Variable** | `VT_SESSION_DIR` | `VT_TRANSCRIPT_DIR` |
| **Timestamp Format** | Unix epoch milliseconds (`u64`) | ISO 8601 string via `chrono::DateTime<Utc>` |
| **File Operations** | Append-only with `tokio::fs::File` | In-memory `Vec<TranscriptEntry>` with full file rewrite |

### Additional Features Implemented

1. **UserMessage Event**: Added new `AiEvent::UserMessage` variant to capture the initial user prompt in transcripts.

2. **Event Filtering**: Streaming events (`TextDelta`, `Reasoning`) are filtered out since their content is captured in aggregate events (`Completed` contains full response).

3. **Sub-Agent Transcript Separation**:
   - Main transcript: Contains `SubAgentStarted`, `SubAgentCompleted`, `SubAgentError` (boundary events)
   - Sub-agent transcript: Contains `SubAgentToolRequest`, `SubAgentToolResult` (internal events)
   - Sub-agent transcript path: `~/.qbit/transcripts/{session_id}/subagents/{agent_id}-{request_id}/transcript.json`

4. **transcript_base_dir Field**: Added to `AgentBridge`, `AgenticLoopContext`, and `SubAgentExecutorContext` to support creating sub-agent transcript files.

### Additional Files Modified

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-core/src/events.rs` | Modify | Added `UserMessage` variant to `AiEvent` enum |
| `backend/crates/qbit-ai/src/agentic_loop.rs` | Modify | Added `transcript_base_dir` field, event filtering |
| `backend/crates/qbit-sub-agents/src/transcript.rs` | **Create** | Sub-agent transcript writer module |
| `backend/crates/qbit-sub-agents/src/executor.rs` | Modify | Write tool events to sub-agent transcript |
| `backend/crates/qbit-sidecar/src/capture.rs` | Modify | Handle `UserMessage` event |
| `backend/crates/qbit-cli-output/src/lib.rs` | Modify | Handle `UserMessage` in CLI JSON output |
| `backend/crates/qbit/src/ai/commands/core.rs` | Modify | Initialize transcript with base directory |

### Rationale for Changes

- **JSON Array vs JSONL**: User requested pretty-printed JSON for easier visual inspection during development
- **Separate Transcripts Directory**: Cleaner organization with session subdirectories instead of flat file structure
- **Sub-agent Separation**: Main transcript stays focused on high-level agent activity; internal sub-agent tool calls are relegated to separate files to avoid noise
- **ISO 8601 Timestamps**: More human-readable than Unix epoch, works well with the `chrono` crate already in use

---

## Prerequisites

- None (first step)

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-ai/src/transcript.rs` | **Create** | Transcript writer module |
| `backend/crates/qbit-ai/src/lib.rs` | Modify | Export transcript module |
| `backend/crates/qbit-ai/src/agent_bridge.rs` | Modify | Initialize and use transcript writer |

---

## Task Breakdown

### 1.1 Create failing tests for TranscriptWriter

**File:** `backend/crates/qbit-ai/src/transcript.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use qbit_core::events::AiEvent;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transcript_writer_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-123";
        
        let writer = TranscriptWriter::new(temp_dir.path(), session_id).unwrap();
        
        let expected_path = temp_dir.path().join(format!("transcript-{}.jsonl", session_id));
        assert!(expected_path.exists());
    }

    #[tokio::test]
    async fn test_transcript_writer_appends_events() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-456";
        
        let writer = TranscriptWriter::new(temp_dir.path(), session_id).unwrap();
        
        let event = AiEvent::Started { turn_id: "turn-1".to_string() };
        writer.append(&event).await.unwrap();
        
        let event2 = AiEvent::Completed {
            response: "Done".to_string(),
            input_tokens: Some(100),
            output_tokens: Some(50),
            duration_ms: Some(1000),
        };
        writer.append(&event2).await.unwrap();
        
        // Read file and verify
        let content = std::fs::read_to_string(
            temp_dir.path().join(format!("transcript-{}.jsonl", session_id))
        ).unwrap();
        
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        
        // Verify each line is valid JSON
        let parsed1: AiEvent = serde_json::from_str(lines[0]).unwrap();
        let parsed2: AiEvent = serde_json::from_str(lines[1]).unwrap();
        
        assert!(matches!(parsed1, AiEvent::Started { .. }));
        assert!(matches!(parsed2, AiEvent::Completed { .. }));
    }

    #[tokio::test]
    async fn test_transcript_writer_handles_concurrent_writes() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-concurrent";
        
        let writer = Arc::new(TranscriptWriter::new(temp_dir.path(), session_id).unwrap());
        
        let mut handles = vec![];
        for i in 0..10 {
            let writer = Arc::clone(&writer);
            handles.push(tokio::spawn(async move {
                let event = AiEvent::Started { turn_id: format!("turn-{}", i) };
                writer.append(&event).await.unwrap();
            }));
        }
        
        for handle in handles {
            handle.await.unwrap();
        }
        
        let content = std::fs::read_to_string(
            temp_dir.path().join(format!("transcript-{}.jsonl", session_id))
        ).unwrap();
        
        assert_eq!(content.lines().count(), 10);
    }

    #[tokio::test]
    async fn test_transcript_writer_includes_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-timestamp";
        
        let writer = TranscriptWriter::new(temp_dir.path(), session_id).unwrap();
        
        let event = AiEvent::Started { turn_id: "turn-1".to_string() };
        writer.append(&event).await.unwrap();
        
        let content = std::fs::read_to_string(
            temp_dir.path().join(format!("transcript-{}.jsonl", session_id))
        ).unwrap();
        
        let line = content.lines().next().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        
        // Should have a timestamp field added by the writer
        assert!(parsed.get("_timestamp").is_some());
    }

    #[test]
    fn test_transcript_path_helper() {
        let base = std::path::Path::new("/tmp/sessions");
        let session_id = "abc-123";
        
        let path = transcript_path(base, session_id);
        assert_eq!(path, std::path::PathBuf::from("/tmp/sessions/transcript-abc-123.jsonl"));
    }
}
```

### 1.2 Implement TranscriptWriter struct

**File:** `backend/crates/qbit-ai/src/transcript.rs`

```rust
//! Transcript writer for capturing AI events to JSONL files.
//!
//! This module provides append-only logging of all AiEvent emissions
//! for later use in context compaction/summarization.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use qbit_core::events::AiEvent;
use serde::Serialize;

/// Wrapper struct for transcript entries with metadata
#[derive(Serialize)]
struct TranscriptEntry<'a> {
    /// Unix timestamp in milliseconds
    _timestamp: u64,
    /// The actual event (flattened)
    #[serde(flatten)]
    event: &'a AiEvent,
}

/// Helper to construct transcript file path
pub fn transcript_path(base_dir: &Path, session_id: &str) -> PathBuf {
    base_dir.join(format!("transcript-{}.jsonl", session_id))
}

/// Append-only writer for AI event transcripts.
///
/// Thread-safe via internal mutex. Each append is atomic (single write + flush).
pub struct TranscriptWriter {
    path: PathBuf,
    file: Arc<Mutex<tokio::fs::File>>,
}

impl TranscriptWriter {
    /// Create a new transcript writer for the given session.
    ///
    /// Creates the file if it doesn't exist, opens for append if it does.
    pub fn new(base_dir: &Path, session_id: &str) -> Result<Self> {
        let path = transcript_path(base_dir, session_id);
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Open file in append mode (blocking for initial open is fine)
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        let async_file = tokio::fs::File::from_std(file);
        
        Ok(Self {
            path,
            file: Arc::new(Mutex::new(async_file)),
        })
    }

    /// Append an event to the transcript.
    ///
    /// Adds a timestamp and writes as a single JSONL line.
    pub async fn append(&self, event: &AiEvent) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let entry = TranscriptEntry {
            _timestamp: timestamp,
            event,
        };
        
        let mut line = serde_json::to_string(&entry)?;
        line.push('\n');
        
        let mut file = self.file.lock().await;
        file.write_all(line.as_bytes()).await?;
        file.flush().await?;
        
        Ok(())
    }

    /// Get the path to the transcript file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
```

### 1.3 Export transcript module

**File:** `backend/crates/qbit-ai/src/lib.rs`

Add:
```rust
pub mod transcript;
pub use transcript::{TranscriptWriter, transcript_path};
```

### 1.4 Integrate TranscriptWriter into AgentBridge

**File:** `backend/crates/qbit-ai/src/agent_bridge.rs`

Tasks:
1. Add `transcript_writer: Option<Arc<TranscriptWriter>>` field to `AgentBridge`
2. Initialize writer in `AgentBridge::new()` or via setter method
3. In the event emission path (where `event_tx.send()` is called), also call `transcript_writer.append()`
4. Handle errors gracefully (log but don't fail the main operation)

```rust
// Add to AgentBridge struct
transcript_writer: Option<Arc<TranscriptWriter>>,

// Add setter method
pub fn set_transcript_writer(&mut self, writer: TranscriptWriter) {
    self.transcript_writer = Some(Arc::new(writer));
}

// In event emission (find all event_tx.send calls):
if let Some(ref writer) = self.transcript_writer {
    if let Err(e) = writer.append(&event).await {
        tracing::warn!("Failed to write to transcript: {}", e);
    }
}
```

### 1.5 Initialize TranscriptWriter when creating sessions

**File:** `backend/crates/qbit/src/ai/commands/session.rs` (or wherever sessions are initialized)

Tasks:
1. When creating a new AI session, create a TranscriptWriter
2. Use `VT_SESSION_DIR` environment variable for base path
3. Pass writer to AgentBridge via `set_transcript_writer()`

```rust
// In session initialization code
let session_dir = std::env::var("VT_SESSION_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|_| {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".qbit/sessions")
    });

if let Ok(writer) = TranscriptWriter::new(&session_dir, &session_id) {
    bridge.set_transcript_writer(writer);
} else {
    tracing::warn!("Failed to create transcript writer for session {}", session_id);
}
```

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit-ai transcript
cargo test -p qbit-sub-agents transcript
```

### Manual Verification
1. Start qbit and create a new session
2. Send a few messages to the AI
3. Check `~/.qbit/transcripts/{session_id}/transcript.json` exists
4. Verify file contains valid JSON array with events
5. If sub-agents were invoked, check `~/.qbit/transcripts/{session_id}/subagents/` for sub-agent transcripts

### Integration Check
```bash
# Full test suite should still pass
cd backend
cargo test
```

---

## Definition of Done

- [x] `TranscriptWriter` struct implemented with tests passing
- [x] All existing tests still pass
- [x] Transcript files created for new sessions
- [x] Events are appended in real-time during conversation
- [x] Concurrent writes don't corrupt the file
- [x] Errors are logged but don't break main functionality
- [x] **Added:** `UserMessage` event captures initial user prompt
- [x] **Added:** Streaming events filtered from transcripts
- [x] **Added:** Sub-agent internal events written to separate transcript files

---

## Rollback Plan

If issues arise:
1. Remove `set_transcript_writer()` calls from session initialization
2. The feature is purely additive - disabling it has no impact on existing functionality
