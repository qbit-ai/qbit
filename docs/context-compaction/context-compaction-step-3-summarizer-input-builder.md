# Step 3: Summarizer Input Builder

**Goal:** Create a module that reads transcript JSON files and converts them to a human-readable format suitable for the summarizer agent.

**Outcome:** After this step, we can call `build_summarizer_input(session_id)` to get a formatted conversation transcript ready for summarization.

---

## Implementation Notes (Step 1 Changes)

> **Important:** Step 1 implementation changed from the original plan. Update this step's implementation to match:
>
> | Aspect | Original Plan | Actual (Step 1) |
> |--------|---------------|-----------------|
> | File Format | JSONL (one JSON per line) | Pretty-printed JSON array |
> | File Path | `~/.qbit/sessions/transcript-{session_id}.jsonl` | `~/.qbit/transcripts/{session_id}/transcript.json` |
> | Env Variable | `VT_SESSION_DIR` | `VT_TRANSCRIPT_DIR` |
> | Timestamp | Unix epoch `u64` | ISO 8601 `DateTime<Utc>` |
> | Sub-agent Events | In main transcript | Separate files in `subagents/` subdirectory |
>
> The code samples below use the **original plan's format**. When implementing, adapt for JSON array parsing instead of line-by-line JSONL parsing.

---

## Prerequisites

- Step 1 completed (TranscriptWriter creates JSON files)
- Step 2 completed (Summarizer expects formatted conversation text)

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/qbit-ai/src/transcript.rs` | Modify | Add reader and formatter functions |

---

## Task Breakdown

### 3.1 Create failing tests for transcript reader

**File:** `backend/crates/qbit-ai/src/transcript.rs`

Add to existing tests module:

```rust
mod reader_tests {
    use super::*;
    use qbit_core::events::AiEvent;
    use tempfile::TempDir;

    fn create_test_transcript(dir: &Path, session_id: &str, events: &[AiEvent]) -> PathBuf {
        let path = transcript_path(dir, session_id);
        let mut file = std::fs::File::create(&path).unwrap();
        for event in events {
            let entry = serde_json::json!({
                "_timestamp": 1700000000000u64,
                "type": event.event_type(),
                // Flatten the event
            });
            // Actually serialize the full event
            let mut line = serde_json::to_string(&TranscriptEntry {
                _timestamp: 1700000000000,
                event,
            }).unwrap();
            line.push('\n');
            std::io::Write::write_all(&mut file, line.as_bytes()).unwrap();
        }
        path
    }

    #[test]
    fn test_read_transcript_returns_events() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-read";
        
        let events = vec![
            AiEvent::Started { turn_id: "turn-1".to_string() },
            AiEvent::TextDelta { 
                delta: "Hello".to_string(), 
                accumulated: "Hello".to_string() 
            },
            AiEvent::Completed {
                response: "Hello there!".to_string(),
                input_tokens: Some(100),
                output_tokens: Some(50),
                duration_ms: Some(1000),
            },
        ];
        
        create_test_transcript(temp_dir.path(), session_id, &events);
        
        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_read_transcript_handles_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = read_transcript(temp_dir.path(), "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_transcript_handles_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-empty";
        let path = transcript_path(temp_dir.path(), session_id);
        std::fs::File::create(&path).unwrap();
        
        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_transcript_skips_malformed_lines() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-malformed";
        let path = transcript_path(temp_dir.path(), session_id);
        
        let content = r#"{"_timestamp":1700000000000,"type":"started","turn_id":"turn-1"}
not valid json
{"_timestamp":1700000000001,"type":"completed","response":"Done","input_tokens":100,"output_tokens":50,"duration_ms":1000}
"#;
        std::fs::write(&path, content).unwrap();
        
        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        // Should have 2 valid events, skipping the malformed line
        assert_eq!(result.len(), 2);
    }
}

mod formatter_tests {
    use super::*;
    use qbit_core::events::AiEvent;

    #[test]
    fn test_format_empty_events() {
        let events: Vec<TranscriptEvent> = vec![];
        let result = format_for_summarizer(&events);
        assert!(result.is_empty() || result.trim().is_empty());
    }

    #[test]
    fn test_format_simple_conversation() {
        let events = vec![
            TranscriptEvent {
                timestamp: 1700000000000,
                event: AiEvent::Started { turn_id: "turn-1".to_string() },
            },
            TranscriptEvent {
                timestamp: 1700000001000,
                event: AiEvent::Completed {
                    response: "I'll help you with that.".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
        ];
        
        let result = format_for_summarizer(&events);
        
        assert!(result.contains("[turn 001]"));
        assert!(result.contains("ASSISTANT"));
        assert!(result.contains("I'll help you with that."));
    }

    #[test]
    fn test_format_includes_tool_calls() {
        let events = vec![
            TranscriptEvent {
                timestamp: 1700000000000,
                event: AiEvent::Started { turn_id: "turn-1".to_string() },
            },
            TranscriptEvent {
                timestamp: 1700000001000,
                event: AiEvent::ToolRequest {
                    tool_name: "read_file".to_string(),
                    args: serde_json::json!({"path": "/src/main.rs"}),
                    request_id: "req-1".to_string(),
                    source: Default::default(),
                },
            },
            TranscriptEvent {
                timestamp: 1700000002000,
                event: AiEvent::ToolResult {
                    tool_name: "read_file".to_string(),
                    result: serde_json::json!({"content": "fn main() {}"}),
                    success: true,
                    request_id: "req-1".to_string(),
                    source: Default::default(),
                },
            },
        ];
        
        let result = format_for_summarizer(&events);
        
        assert!(result.contains("TOOL_REQUEST"));
        assert!(result.contains("read_file"));
        assert!(result.contains("TOOL_RESULT"));
    }

    #[test]
    fn test_format_includes_hitl_events() {
        let events = vec![
            TranscriptEvent {
                timestamp: 1700000000000,
                event: AiEvent::ToolApprovalRequest {
                    request_id: "req-1".to_string(),
                    tool_name: "write_file".to_string(),
                    args: serde_json::json!({"path": "/test.txt"}),
                    stats: None,
                    risk_level: qbit_core::hitl::RiskLevel::Medium,
                    can_learn: true,
                    suggestion: None,
                    source: Default::default(),
                },
            },
            TranscriptEvent {
                timestamp: 1700000001000,
                event: AiEvent::ToolAutoApproved {
                    request_id: "req-1".to_string(),
                    tool_name: "write_file".to_string(),
                    args: serde_json::json!({}),
                    reason: "User approved".to_string(),
                    source: Default::default(),
                },
            },
        ];
        
        let result = format_for_summarizer(&events);
        
        assert!(result.contains("TOOL_APPROVAL_REQUEST") || result.contains("APPROVAL"));
        assert!(result.contains("write_file"));
    }

    #[test]
    fn test_format_tracks_turn_numbers() {
        let events = vec![
            TranscriptEvent {
                timestamp: 1700000000000,
                event: AiEvent::Started { turn_id: "turn-1".to_string() },
            },
            TranscriptEvent {
                timestamp: 1700000001000,
                event: AiEvent::Completed {
                    response: "First response".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
            TranscriptEvent {
                timestamp: 1700000002000,
                event: AiEvent::Started { turn_id: "turn-2".to_string() },
            },
            TranscriptEvent {
                timestamp: 1700000003000,
                event: AiEvent::Completed {
                    response: "Second response".to_string(),
                    input_tokens: Some(150),
                    output_tokens: Some(75),
                    duration_ms: Some(1200),
                },
            },
        ];
        
        let result = format_for_summarizer(&events);
        
        assert!(result.contains("[turn 001]"));
        assert!(result.contains("[turn 002]"));
    }

    #[test]
    fn test_format_excludes_text_delta() {
        // TextDelta events are streaming chunks - we only want the final Completed response
        let events = vec![
            TranscriptEvent {
                timestamp: 1700000000000,
                event: AiEvent::TextDelta {
                    delta: "Hello".to_string(),
                    accumulated: "Hello".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: 1700000001000,
                event: AiEvent::TextDelta {
                    delta: " world".to_string(),
                    accumulated: "Hello world".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: 1700000002000,
                event: AiEvent::Completed {
                    response: "Hello world".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
        ];
        
        let result = format_for_summarizer(&events);
        
        // Should not have duplicate content from TextDelta
        let hello_count = result.matches("Hello world").count();
        assert_eq!(hello_count, 1, "Should only have final response, not streaming deltas");
    }
}

mod integration_tests {
    use super::*;
    use qbit_core::events::AiEvent;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_build_summarizer_input_end_to_end() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-e2e";
        
        // Create a transcript using the writer
        let writer = TranscriptWriter::new(temp_dir.path(), session_id).unwrap();
        
        writer.append(&AiEvent::Started { turn_id: "turn-1".to_string() }).await.unwrap();
        writer.append(&AiEvent::ToolRequest {
            tool_name: "read_file".to_string(),
            args: serde_json::json!({"path": "/src/main.rs"}),
            request_id: "req-1".to_string(),
            source: Default::default(),
        }).await.unwrap();
        writer.append(&AiEvent::ToolResult {
            tool_name: "read_file".to_string(),
            result: serde_json::json!({"content": "fn main() { println!(\"hello\"); }"}),
            success: true,
            request_id: "req-1".to_string(),
            source: Default::default(),
        }).await.unwrap();
        writer.append(&AiEvent::Completed {
            response: "I found the main function.".to_string(),
            input_tokens: Some(200),
            output_tokens: Some(100),
            duration_ms: Some(2000),
        }).await.unwrap();
        
        // Now read and format
        let input = build_summarizer_input(temp_dir.path(), session_id).unwrap();
        
        assert!(input.contains("[turn 001]"));
        assert!(input.contains("read_file"));
        assert!(input.contains("I found the main function"));
    }
}
```

### 3.2 Implement TranscriptEvent and reader

**File:** `backend/crates/qbit-ai/src/transcript.rs`

Add to the module:

```rust
use std::io::BufRead;

/// A transcript event with its timestamp.
#[derive(Debug, Clone)]
pub struct TranscriptEvent {
    pub timestamp: u64,
    pub event: AiEvent,
}

/// Read all events from a transcript file.
///
/// Returns events in chronological order. Malformed lines are skipped with a warning.
pub fn read_transcript(base_dir: &Path, session_id: &str) -> Result<Vec<TranscriptEvent>> {
    let path = transcript_path(base_dir, session_id);
    
    let file = std::fs::File::open(&path)
        .map_err(|e| anyhow::anyhow!("Failed to open transcript {}: {}", path.display(), e))?;
    
    let reader = std::io::BufReader::new(file);
    let mut events = Vec::new();
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                tracing::warn!("Failed to read line {} of transcript: {}", line_num + 1, e);
                continue;
            }
        };
        
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse the JSON line
        match serde_json::from_str::<serde_json::Value>(&line) {
            Ok(value) => {
                // Extract timestamp
                let timestamp = value
                    .get("_timestamp")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                
                // Parse the event (the rest of the fields)
                match serde_json::from_value::<AiEvent>(value.clone()) {
                    Ok(event) => {
                        events.push(TranscriptEvent { timestamp, event });
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse event at line {}: {}",
                            line_num + 1,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse JSON at line {}: {}",
                    line_num + 1,
                    e
                );
            }
        }
    }
    
    Ok(events)
}
```

### 3.3 Implement formatter

**File:** `backend/crates/qbit-ai/src/transcript.rs`

```rust
/// Format transcript events for the summarizer.
///
/// Produces a human-readable text format that the summarizer can process.
/// Excludes streaming events (TextDelta) in favor of final Completed responses.
pub fn format_for_summarizer(events: &[TranscriptEvent]) -> String {
    let mut output = String::new();
    let mut current_turn: u32 = 0;
    let mut in_turn = false;
    
    for te in events {
        match &te.event {
            AiEvent::Started { turn_id } => {
                current_turn += 1;
                in_turn = true;
                // Don't output anything for Started - the turn header comes with content
            }
            
            AiEvent::Completed { response, input_tokens, output_tokens, .. } => {
                output.push_str(&format!(
                    "[turn {:03}] ASSISTANT (completed, {} in / {} out tokens):\n{}\n\n",
                    current_turn,
                    input_tokens.unwrap_or(0),
                    output_tokens.unwrap_or(0),
                    response
                ));
                in_turn = false;
            }
            
            AiEvent::ToolRequest { tool_name, args, request_id, .. } => {
                let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
                output.push_str(&format!(
                    "[turn {:03}] TOOL_REQUEST (tool={}, request_id={}):\n{}\n\n",
                    current_turn,
                    tool_name,
                    request_id,
                    args_str
                ));
            }
            
            AiEvent::ToolResult { tool_name, result, success, request_id, .. } => {
                let result_str = if let Some(s) = result.as_str() {
                    s.to_string()
                } else {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                };
                // Truncate very long results
                let result_display = if result_str.len() > 2000 {
                    format!("{}...[truncated, {} chars total]", &result_str[..2000], result_str.len())
                } else {
                    result_str
                };
                output.push_str(&format!(
                    "[turn {:03}] TOOL_RESULT (tool={}, success={}):\n{}\n\n",
                    current_turn,
                    tool_name,
                    success,
                    result_display
                ));
            }
            
            AiEvent::ToolApprovalRequest { tool_name, args, risk_level, .. } => {
                let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
                output.push_str(&format!(
                    "[turn {:03}] TOOL_APPROVAL_REQUEST (tool={}, risk={:?}):\n{}\n\n",
                    current_turn,
                    tool_name,
                    risk_level,
                    args_str
                ));
            }
            
            AiEvent::ToolAutoApproved { tool_name, reason, .. } => {
                output.push_str(&format!(
                    "[turn {:03}] TOOL_AUTO_APPROVED (tool={}): {}\n\n",
                    current_turn,
                    tool_name,
                    reason
                ));
            }
            
            AiEvent::ToolDenied { tool_name, reason, .. } => {
                output.push_str(&format!(
                    "[turn {:03}] TOOL_DENIED (tool={}): {}\n\n",
                    current_turn,
                    tool_name,
                    reason
                ));
            }
            
            AiEvent::Error { message, error_type } => {
                output.push_str(&format!(
                    "[turn {:03}] ERROR ({}): {}\n\n",
                    current_turn,
                    error_type,
                    message
                ));
            }
            
            AiEvent::SubAgentStarted { agent_name, task, .. } => {
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_STARTED (agent={}):\n{}\n\n",
                    current_turn,
                    agent_name,
                    task
                ));
            }
            
            AiEvent::SubAgentCompleted { agent_name, response, .. } => {
                // Truncate long sub-agent responses
                let response_display = if response.len() > 3000 {
                    format!("{}...[truncated]", &response[..3000])
                } else {
                    response.clone()
                };
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_COMPLETED (agent={}):\n{}\n\n",
                    current_turn,
                    agent_name,
                    response_display
                ));
            }
            
            AiEvent::SubAgentError { agent_name, error, .. } => {
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_ERROR (agent={}): {}\n\n",
                    current_turn,
                    agent_name,
                    error
                ));
            }
            
            // Skip these events - they're either streaming or not useful for summarization
            AiEvent::TextDelta { .. } => {}
            AiEvent::Reasoning { .. } => {} // Could include if valuable
            AiEvent::ContextPruned { .. } => {}
            AiEvent::ContextWarning { .. } => {}
            AiEvent::ToolResponseTruncated { .. } => {}
            AiEvent::LoopWarning { .. } => {}
            AiEvent::LoopBlocked { .. } => {}
            AiEvent::MaxIterationsReached { .. } => {}
            AiEvent::Warning { .. } => {}
            AiEvent::SubAgentToolRequest { .. } => {} // Too verbose
            AiEvent::SubAgentToolResult { .. } => {} // Too verbose
            AiEvent::WorkflowStarted { .. } => {}
            AiEvent::WorkflowStepStarted { .. } => {}
            AiEvent::WorkflowStepCompleted { .. } => {}
            AiEvent::WorkflowCompleted { .. } => {}
            AiEvent::WorkflowError { .. } => {}
            AiEvent::PlanUpdated { .. } => {} // Could include summary
            AiEvent::ServerToolStarted { .. } => {}
            AiEvent::WebSearchResult { .. } => {}
            AiEvent::WebFetchResult { .. } => {}
        }
    }
    
    output
}

/// Build summarizer input from a session's transcript.
///
/// This is the main entry point - reads the transcript file and formats it.
pub fn build_summarizer_input(base_dir: &Path, session_id: &str) -> Result<String> {
    let events = read_transcript(base_dir, session_id)?;
    Ok(format_for_summarizer(&events))
}
```

### 3.4 Add artifact persistence

**File:** `backend/crates/qbit-ai/src/transcript.rs`

```rust
/// Save summarizer input to an artifact file.
///
/// Returns the path to the saved file.
pub fn save_summarizer_input(
    base_dir: &Path,
    session_id: &str,
    content: &str,
) -> Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let filename = format!("summarizer-input-{}-{}.md", session_id, timestamp);
    let path = base_dir.join(filename);
    
    std::fs::write(&path, content)?;
    
    Ok(path)
}

/// Save a summary to an artifact file.
///
/// Returns the path to the saved file.
pub fn save_summary(
    base_dir: &Path,
    session_id: &str,
    summary: &str,
) -> Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let filename = format!("summary-{}-{}.md", session_id, timestamp);
    let path = base_dir.join(filename);
    
    std::fs::write(&path, summary)?;
    
    Ok(path)
}

#[cfg(test)]
mod artifact_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_summarizer_input() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-save";
        let content = "# Test Content\n\nSome conversation here.";
        
        let path = save_summarizer_input(temp_dir.path(), session_id, content).unwrap();
        
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("summarizer-input-"));
        assert!(path.to_string_lossy().contains(session_id));
        
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, content);
    }

    #[test]
    fn test_save_summary() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-summary";
        let summary = "## Summary\n\nUser asked for help.";
        
        let path = save_summary(temp_dir.path(), session_id, summary).unwrap();
        
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("summary-"));
        
        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, summary);
    }
}
```

---

## Verification

### Run Tests
```bash
cd backend
cargo test -p qbit-ai transcript
```

### Manual Verification
1. Start qbit, have a conversation with tool calls
2. Find the transcript file in `~/.qbit/transcripts/{session_id}/transcript.json`
3. Call `build_summarizer_input()` on it (via test or debug command)
4. Verify output is readable and contains expected events

### Integration Check
```bash
cd backend
cargo test
```

---

## Definition of Done

- [ ] `read_transcript()` reads JSONL files correctly
- [ ] `format_for_summarizer()` produces human-readable output
- [ ] Turn numbers are tracked correctly
- [ ] TextDelta events are excluded (only final Completed)
- [ ] Tool calls and results are included with proper formatting
- [ ] HITL events (approval requests, auto-approved, denied) are included
- [ ] Very long tool results are truncated
- [ ] `save_summarizer_input()` and `save_summary()` work correctly
- [ ] All tests pass
- [ ] Existing tests still pass

---

## Notes

- The formatter intentionally excludes streaming events (TextDelta) to avoid duplication
- Sub-agent tool calls are excluded (too verbose) but sub-agent start/complete are included
- Long tool results are truncated to keep the input manageable
- Artifact files use timestamps to allow multiple compactions in a session
