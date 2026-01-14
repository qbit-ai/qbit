//! Transcript writer for capturing AI events to JSON files.
//!
//! This module provides functionality to persist AI events to disk in a
//! pretty-printed JSON array format, enabling replay, debugging, and analysis
//! of agent sessions.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use qbit_core::events::AiEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// A wrapper struct for transcript entries that includes a timestamp.
///
/// Uses `#[serde(flatten)]` to inline the event fields alongside the timestamp,
/// producing output like: `{"_timestamp": "...", "type": "started", "turn_id": "..."}`
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranscriptEntry {
    /// ISO 8601 timestamp when the event was recorded
    _timestamp: DateTime<Utc>,
    /// The AI event (flattened into the same JSON object)
    #[serde(flatten)]
    event: AiEvent,
}

/// A transcript event with its timestamp (public version for consumers).
///
/// This struct is returned by [`read_transcript()`] and provides access to both
/// the timestamp when the event was recorded and the event itself.
#[derive(Debug, Clone)]
pub struct TranscriptEvent {
    /// The timestamp when the event was recorded
    pub timestamp: DateTime<Utc>,
    /// The AI event
    pub event: AiEvent,
}

impl From<TranscriptEntry> for TranscriptEvent {
    fn from(entry: TranscriptEntry) -> Self {
        Self {
            timestamp: entry._timestamp,
            event: entry.event,
        }
    }
}

/// Thread-safe writer for appending AI events to a JSON transcript file.
///
/// Events are stored as a pretty-printed JSON array with timestamps.
/// The writer uses an async mutex to ensure thread-safe access.
///
/// # Example
///
/// ```ignore
/// use qbit_ai::transcript::TranscriptWriter;
/// use qbit_core::events::AiEvent;
/// use std::path::Path;
///
/// let writer = TranscriptWriter::new(Path::new("/tmp/transcripts"), "session-123").await?;
/// let event = AiEvent::Started { turn_id: "turn-1".to_string() };
/// writer.append(&event).await?;
/// ```
#[derive(Debug)]
pub struct TranscriptWriter {
    /// Path to the transcript file
    path: PathBuf,
    /// In-memory list of entries, protected by mutex for thread safety
    entries: Arc<Mutex<Vec<TranscriptEntry>>>,
}

impl TranscriptWriter {
    /// Creates a new `TranscriptWriter` that writes to `{base_dir}/{session_id}/transcript.json`.
    ///
    /// If the file exists, existing entries are loaded. Otherwise, starts with an empty array.
    /// Parent directories (including the session subdirectory) are created as needed.
    ///
    /// # Arguments
    ///
    /// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
    /// * `session_id` - A unique identifier for the session
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created or the file cannot be read.
    pub async fn new(base_dir: &Path, session_id: &str) -> anyhow::Result<Self> {
        let path = transcript_path(base_dir, session_id);

        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Load existing entries if file exists, otherwise start empty
        let entries = if path.exists() {
            let content = tokio::fs::read_to_string(&path).await?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            path,
            entries: Arc::new(Mutex::new(entries)),
        })
    }

    /// Appends an AI event to the transcript.
    ///
    /// The event is wrapped with a `_timestamp` field and added to the array.
    /// The entire file is rewritten with pretty-printed JSON.
    ///
    /// This method is thread-safe and can be called concurrently from multiple tasks.
    ///
    /// # Arguments
    ///
    /// * `event` - The AI event to append
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the file cannot be written to.
    pub async fn append(&self, event: &AiEvent) -> anyhow::Result<()> {
        let entry = TranscriptEntry {
            _timestamp: Utc::now(),
            event: event.clone(),
        };

        let mut entries = self.entries.lock().await;
        entries.push(entry);

        // Write pretty-printed JSON array to file
        let json = serde_json::to_string_pretty(&*entries)?;
        tokio::fs::write(&self.path, json).await?;

        Ok(())
    }

    /// Returns the path to the transcript file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Constructs the transcript file path for a given base directory and session ID.
///
/// # Arguments
///
/// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
/// * `session_id` - A unique identifier for the session
///
/// # Returns
///
/// A `PathBuf` pointing to `{base_dir}/{session_id}/transcript.json`
pub fn transcript_path(base_dir: &Path, session_id: &str) -> PathBuf {
    base_dir.join(session_id).join("transcript.json")
}

/// Read all events from a transcript file.
///
/// Returns events in chronological order (the order they were written).
///
/// # Arguments
///
/// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
/// * `session_id` - The unique identifier for the session
///
/// # Returns
///
/// A vector of [`TranscriptEvent`]s in chronological order.
///
/// # Errors
///
/// Returns an error if the file doesn't exist or cannot be read.
/// Empty files and files containing an empty JSON array (`[]`) return an empty `Vec`.
///
/// # Example
///
/// ```ignore
/// use qbit_ai::transcript::read_transcript;
/// use std::path::Path;
///
/// let events = read_transcript(Path::new("/tmp/transcripts"), "session-123")?;
/// for event in events {
///     println!("{}: {:?}", event.timestamp, event.event);
/// }
/// ```
pub fn read_transcript(base_dir: &Path, session_id: &str) -> anyhow::Result<Vec<TranscriptEvent>> {
    let path = transcript_path(base_dir, session_id);

    let content = std::fs::read_to_string(&path)?;

    // Handle empty file
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Parse as JSON array of TranscriptEntry
    let entries: Vec<TranscriptEntry> = serde_json::from_str(&content)?;

    // Convert to public TranscriptEvent type
    Ok(entries.into_iter().map(TranscriptEvent::from).collect())
}

/// Format transcript events for the summarizer.
///
/// Produces a human-readable text format that the summarizer can process.
/// Excludes streaming events (TextDelta) in favor of final Completed responses.
/// Also excludes internal events like context management, loop detection, and
/// workflow lifecycle events that are not relevant for summarization.
///
/// # Arguments
///
/// * `events` - A slice of transcript events to format
///
/// # Returns
///
/// A formatted string with turn-based structure suitable for summarization.
///
/// # Example Output
///
/// ```text
/// [turn 001] USER:
/// Please help me fix this bug.
///
/// [turn 001] TOOL_REQUEST (tool=read_file, id=req-1):
/// {
///   "path": "/src/main.rs"
/// }
///
/// [turn 001] TOOL_RESULT (tool=read_file, success=true):
/// fn main() { ... }
///
/// [turn 001] ASSISTANT (100 in / 50 out tokens):
/// I found the issue. Let me fix it.
/// ```
pub fn format_for_summarizer(events: &[TranscriptEvent]) -> String {
    let mut output = String::new();
    let mut current_turn: u32 = 0;

    for te in events {
        match &te.event {
            AiEvent::Started { .. } => {
                current_turn += 1;
                // Don't output anything for Started - the turn header comes with content
            }

            AiEvent::UserMessage { content } => {
                output.push_str(&format!(
                    "[turn {:03}] USER:\n{}\n\n",
                    current_turn, content
                ));
            }

            AiEvent::Completed {
                response,
                input_tokens,
                output_tokens,
                ..
            } => {
                output.push_str(&format!(
                    "[turn {:03}] ASSISTANT ({} in / {} out tokens):\n{}\n\n",
                    current_turn,
                    input_tokens.unwrap_or(0),
                    output_tokens.unwrap_or(0),
                    response
                ));
            }

            AiEvent::ToolRequest {
                tool_name,
                args,
                request_id,
                ..
            } => {
                let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
                output.push_str(&format!(
                    "[turn {:03}] TOOL_REQUEST (tool={}, id={}):\n{}\n\n",
                    current_turn, tool_name, request_id, args_str
                ));
            }

            AiEvent::ToolResult {
                tool_name,
                result,
                success,
                ..
            } => {
                let result_str = if let Some(s) = result.as_str() {
                    s.to_string()
                } else {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                };
                // Truncate very long results
                let result_display = if result_str.len() > 2000 {
                    format!(
                        "{}...\n[truncated, {} chars total]",
                        &result_str[..2000],
                        result_str.len()
                    )
                } else {
                    result_str
                };
                output.push_str(&format!(
                    "[turn {:03}] TOOL_RESULT (tool={}, success={}):\n{}\n\n",
                    current_turn, tool_name, success, result_display
                ));
            }

            AiEvent::ToolApprovalRequest {
                tool_name,
                args,
                risk_level,
                ..
            } => {
                let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
                output.push_str(&format!(
                    "[turn {:03}] TOOL_APPROVAL_REQUEST (tool={}, risk={:?}):\n{}\n\n",
                    current_turn, tool_name, risk_level, args_str
                ));
            }

            AiEvent::ToolAutoApproved {
                tool_name, reason, ..
            } => {
                output.push_str(&format!(
                    "[turn {:03}] TOOL_AUTO_APPROVED (tool={}): {}\n\n",
                    current_turn, tool_name, reason
                ));
            }

            AiEvent::ToolDenied {
                tool_name, reason, ..
            } => {
                output.push_str(&format!(
                    "[turn {:03}] TOOL_DENIED (tool={}): {}\n\n",
                    current_turn, tool_name, reason
                ));
            }

            AiEvent::Error {
                message,
                error_type,
            } => {
                output.push_str(&format!(
                    "[turn {:03}] ERROR ({}): {}\n\n",
                    current_turn, error_type, message
                ));
            }

            AiEvent::SubAgentStarted {
                agent_name, task, ..
            } => {
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_STARTED (agent={}):\n{}\n\n",
                    current_turn, agent_name, task
                ));
            }

            AiEvent::SubAgentCompleted {
                agent_id, response, ..
            } => {
                // Truncate long sub-agent responses
                let response_display = if response.len() > 3000 {
                    format!("{}...\n[truncated]", &response[..3000])
                } else {
                    response.clone()
                };
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_COMPLETED (agent={}):\n{}\n\n",
                    current_turn, agent_id, response_display
                ));
            }

            AiEvent::SubAgentError {
                agent_id, error, ..
            } => {
                output.push_str(&format!(
                    "[turn {:03}] SUB_AGENT_ERROR (agent={}): {}\n\n",
                    current_turn, agent_id, error
                ));
            }

            // Skip these events - streaming or not useful for summarization
            AiEvent::TextDelta { .. } => {}
            AiEvent::Reasoning { .. } => {}
            AiEvent::ContextPruned { .. } => {}
            AiEvent::ContextWarning { .. } => {}
            AiEvent::ToolResponseTruncated { .. } => {}
            AiEvent::LoopWarning { .. } => {}
            AiEvent::LoopBlocked { .. } => {}
            AiEvent::MaxIterationsReached { .. } => {}
            AiEvent::Warning { .. } => {}
            AiEvent::SubAgentToolRequest { .. } => {} // Too verbose
            AiEvent::SubAgentToolResult { .. } => {}  // Too verbose
            AiEvent::WorkflowStarted { .. } => {}
            AiEvent::WorkflowStepStarted { .. } => {}
            AiEvent::WorkflowStepCompleted { .. } => {}
            AiEvent::WorkflowCompleted { .. } => {}
            AiEvent::WorkflowError { .. } => {}
            AiEvent::PlanUpdated { .. } => {}
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
///
/// # Arguments
///
/// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
/// * `session_id` - The unique identifier for the session
///
/// # Returns
///
/// A formatted string suitable for the summarizer agent.
///
/// # Errors
///
/// Returns an error if the transcript file doesn't exist or cannot be read.
pub fn build_summarizer_input(base_dir: &Path, session_id: &str) -> anyhow::Result<String> {
    let events = read_transcript(base_dir, session_id)?;
    Ok(format_for_summarizer(&events))
}

/// Save summarizer input to an artifact file.
///
/// Creates the directory if it doesn't exist and writes the content to a
/// timestamped file for debugging and auditing purposes.
///
/// # Arguments
///
/// * `base_dir` - The base directory for artifacts (e.g., `~/.qbit/artifacts/compaction`)
/// * `session_id` - The unique identifier for the session
/// * `content` - The summarizer input content to save
///
/// # Returns
///
/// The path to the saved file.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn save_summarizer_input(
    base_dir: &Path,
    session_id: &str,
    content: &str,
) -> anyhow::Result<PathBuf> {
    // Ensure the directory exists
    std::fs::create_dir_all(base_dir)?;

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
/// Creates the directory if it doesn't exist and writes the summary to a
/// timestamped file for debugging and auditing purposes.
///
/// # Arguments
///
/// * `base_dir` - The base directory for artifacts (e.g., `~/.qbit/artifacts/summaries`)
/// * `session_id` - The unique identifier for the session
/// * `summary` - The summary content to save
///
/// # Returns
///
/// The path to the saved file.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn save_summary(
    base_dir: &Path,
    session_id: &str,
    summary: &str,
) -> anyhow::Result<PathBuf> {
    // Ensure the directory exists
    std::fs::create_dir_all(base_dir)?;

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
mod tests {
    use super::*;
    use qbit_core::events::AiEvent;
    use serde_json::Value;
    use tempfile::TempDir;

    /// Verifies that the TranscriptWriter creates the session directory on first append.
    #[tokio::test]
    async fn test_transcript_writer_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-001";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .expect("Failed to create TranscriptWriter");

        // Append an event to trigger file creation
        let event = AiEvent::Started {
            turn_id: "turn-1".to_string(),
        };
        writer.append(&event).await.expect("Failed to append event");

        // Verify the file was created
        assert!(writer.path().exists(), "Transcript file should exist");

        // Verify the path is correct
        let expected_path = temp_dir
            .path()
            .join("test-session-001")
            .join("transcript.json");
        assert_eq!(writer.path(), expected_path);
    }

    /// Verifies that events are stored as a valid JSON array.
    #[tokio::test]
    async fn test_transcript_writer_appends_events() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-002";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .expect("Failed to create TranscriptWriter");

        // Append several events
        let events = vec![
            AiEvent::Started {
                turn_id: "turn-1".to_string(),
            },
            AiEvent::TextDelta {
                delta: "Hello".to_string(),
                accumulated: "Hello".to_string(),
            },
            AiEvent::Completed {
                response: "Done".to_string(),
                input_tokens: Some(100),
                output_tokens: Some(50),
                duration_ms: Some(1000),
            },
        ];

        for event in &events {
            writer.append(event).await.expect("Failed to append event");
        }

        // Read the file and parse as JSON array
        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read transcript file");

        let entries: Vec<Value> =
            serde_json::from_str(&content).expect("Should be valid JSON array");
        assert_eq!(entries.len(), 3, "Should have 3 entries");

        // Verify each entry
        assert_eq!(entries[0]["type"], "started");
        assert_eq!(entries[0]["turn_id"], "turn-1");

        assert_eq!(entries[1]["type"], "text_delta");
        assert_eq!(entries[1]["delta"], "Hello");

        assert_eq!(entries[2]["type"], "completed");
        assert_eq!(entries[2]["response"], "Done");
    }

    /// Verifies thread safety by performing concurrent writes.
    #[tokio::test]
    async fn test_transcript_writer_handles_concurrent_writes() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-003";

        let writer = Arc::new(
            TranscriptWriter::new(temp_dir.path(), session_id)
                .await
                .expect("Failed to create TranscriptWriter"),
        );

        // Spawn 10 concurrent write tasks
        let mut handles = Vec::new();
        for i in 0..10 {
            let writer_clone = Arc::clone(&writer);
            let handle = tokio::spawn(async move {
                let event = AiEvent::TextDelta {
                    delta: format!("chunk-{i}"),
                    accumulated: format!("accumulated-{i}"),
                };
                writer_clone
                    .append(&event)
                    .await
                    .expect("Failed to append event");
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task panicked");
        }

        // Read and verify all 10 entries were written
        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read transcript file");

        let entries: Vec<Value> =
            serde_json::from_str(&content).expect("Should be valid JSON array");
        assert_eq!(
            entries.len(),
            10,
            "Should have 10 entries from concurrent writes"
        );

        // Verify each entry is a text_delta event
        for entry in &entries {
            assert_eq!(entry["type"], "text_delta");
        }
    }

    /// Verifies that each entry includes a `_timestamp` field.
    #[tokio::test]
    async fn test_transcript_writer_includes_timestamp() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-004";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .expect("Failed to create TranscriptWriter");

        let event = AiEvent::Started {
            turn_id: "turn-ts".to_string(),
        };
        writer.append(&event).await.expect("Failed to append event");

        // Read and parse the array
        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read transcript file");

        let entries: Vec<Value> =
            serde_json::from_str(&content).expect("Should be valid JSON array");
        assert_eq!(entries.len(), 1);

        let entry = &entries[0];

        // Verify _timestamp field exists and is a valid ISO 8601 string
        assert!(
            entry.get("_timestamp").is_some(),
            "_timestamp field should exist"
        );
        let timestamp_str = entry["_timestamp"]
            .as_str()
            .expect("_timestamp should be a string");

        // Verify it can be parsed as a DateTime
        let parsed: Result<DateTime<Utc>, _> = timestamp_str.parse();
        assert!(
            parsed.is_ok(),
            "_timestamp should be a valid ISO 8601 datetime"
        );

        // Verify the event fields are also present (flattened)
        assert_eq!(entry["type"], "started");
        assert_eq!(entry["turn_id"], "turn-ts");
    }

    /// Verifies the transcript_path helper constructs the correct path.
    #[test]
    fn test_transcript_path_helper() {
        let base_dir = Path::new("/var/log/qbit/transcripts");
        let session_id = "abc-123";

        let path = transcript_path(base_dir, session_id);

        assert_eq!(
            path,
            PathBuf::from("/var/log/qbit/transcripts/abc-123/transcript.json")
        );
    }

    /// Verifies path construction with various session ID formats.
    #[test]
    fn test_transcript_path_with_various_session_ids() {
        let base_dir = Path::new("/tmp/transcripts");

        // UUID-style session ID
        let path1 = transcript_path(base_dir, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(
            path1,
            PathBuf::from("/tmp/transcripts/550e8400-e29b-41d4-a716-446655440000/transcript.json")
        );

        // Simple numeric session ID
        let path2 = transcript_path(base_dir, "12345");
        assert_eq!(
            path2,
            PathBuf::from("/tmp/transcripts/12345/transcript.json")
        );

        // Empty session ID (edge case)
        let path3 = transcript_path(base_dir, "");
        assert_eq!(path3, PathBuf::from("/tmp/transcripts//transcript.json"));
    }

    /// Verifies that pretty-printed JSON is human-readable with proper indentation.
    #[tokio::test]
    async fn test_transcript_writer_produces_pretty_json() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-session-pretty";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .expect("Failed to create TranscriptWriter");

        let event = AiEvent::Started {
            turn_id: "turn-1".to_string(),
        };
        writer.append(&event).await.expect("Failed to append event");

        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read transcript file");

        // Pretty-printed JSON should have multiple lines and indentation
        assert!(content.contains('\n'), "Pretty JSON should have newlines");
        assert!(
            content.contains("  "),
            "Pretty JSON should have indentation"
        );
    }
}

#[cfg(test)]
mod reader_tests {
    use super::*;
    use tempfile::TempDir;

    /// Verifies that read_transcript returns events that were written by TranscriptWriter.
    #[tokio::test]
    async fn test_read_transcript_returns_events() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-read";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .unwrap();
        writer
            .append(&AiEvent::Started {
                turn_id: "turn-1".to_string(),
            })
            .await
            .unwrap();
        writer
            .append(&AiEvent::Completed {
                response: "Done".to_string(),
                input_tokens: Some(100),
                output_tokens: Some(50),
                duration_ms: Some(1000),
            })
            .await
            .unwrap();

        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert_eq!(result.len(), 2);

        // Verify first event is Started
        assert!(matches!(result[0].event, AiEvent::Started { .. }));

        // Verify second event is Completed
        assert!(matches!(result[1].event, AiEvent::Completed { .. }));

        // Verify timestamps are present and in order
        assert!(result[0].timestamp <= result[1].timestamp);
    }

    /// Verifies that read_transcript returns an error for missing files.
    #[test]
    fn test_read_transcript_handles_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let result = read_transcript(temp_dir.path(), "nonexistent");
        assert!(result.is_err());
    }

    /// Verifies that read_transcript returns empty Vec for empty files.
    #[tokio::test]
    async fn test_read_transcript_handles_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-empty";
        let path = transcript_path(temp_dir.path(), session_id);

        // Create parent directory and empty file
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();

        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert!(result.is_empty());
    }

    /// Verifies that read_transcript returns empty Vec for files containing empty JSON array.
    #[tokio::test]
    async fn test_read_transcript_handles_empty_array() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-empty-array";
        let path = transcript_path(temp_dir.path(), session_id);

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "[]").unwrap();

        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert!(result.is_empty());
    }

    /// Verifies that timestamps are correctly parsed from transcript entries.
    #[tokio::test]
    async fn test_read_transcript_preserves_timestamps() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-timestamps";

        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .unwrap();

        let before = Utc::now();
        writer
            .append(&AiEvent::Started {
                turn_id: "turn-1".to_string(),
            })
            .await
            .unwrap();
        let after = Utc::now();

        let result = read_transcript(temp_dir.path(), session_id).unwrap();
        assert_eq!(result.len(), 1);

        // Timestamp should be between before and after
        assert!(result[0].timestamp >= before);
        assert!(result[0].timestamp <= after);
    }
}

#[cfg(test)]
mod formatter_tests {
    use super::*;

    /// Verifies that formatting an empty event list returns an empty string.
    #[test]
    fn test_format_empty_events() {
        let events: Vec<TranscriptEvent> = vec![];
        let result = format_for_summarizer(&events);
        assert!(result.is_empty());
    }

    /// Verifies that a simple conversation formats correctly with turn numbers and token counts.
    #[test]
    fn test_format_simple_conversation() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
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
        assert!(result.contains("100 in / 50 out tokens"));
    }

    /// Verifies that tool requests and results are included in the output.
    #[test]
    fn test_format_includes_tool_calls() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ToolRequest {
                    tool_name: "read_file".to_string(),
                    args: serde_json::json!({"path": "/src/main.rs"}),
                    request_id: "req-1".to_string(),
                    source: Default::default(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
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

    /// Verifies that user messages are included in the output.
    #[test]
    fn test_format_includes_user_message() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::UserMessage {
                    content: "Please help me debug this.".to_string(),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        assert!(result.contains("USER:"));
        assert!(result.contains("Please help me debug this."));
    }

    /// Verifies that TextDelta events are skipped (only Completed response is included).
    #[test]
    fn test_format_excludes_text_delta() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::TextDelta {
                    delta: "Hello".to_string(),
                    accumulated: "Hello".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::TextDelta {
                    delta: " world".to_string(),
                    accumulated: "Hello world".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Completed {
                    response: "Hello world".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        // Should only have final response, not streaming deltas
        let hello_count = result.matches("Hello world").count();
        assert_eq!(
            hello_count, 1,
            "Should only have final response, not streaming deltas"
        );
    }

    /// Verifies that turn numbers increment correctly across multiple turns.
    #[test]
    fn test_format_tracks_turn_numbers() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Completed {
                    response: "First response".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-2".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
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

    /// Verifies that very long tool results are truncated.
    #[test]
    fn test_format_truncates_long_tool_results() {
        // Create a result that's > 2000 chars
        let long_content = "x".repeat(3000);
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ToolResult {
                    tool_name: "read_file".to_string(),
                    result: serde_json::json!(long_content),
                    success: true,
                    request_id: "req-1".to_string(),
                    source: Default::default(),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        assert!(result.contains("truncated"));
        assert!(result.contains("3000 chars total"));
    }

    /// Verifies that error events are included in the output.
    #[test]
    fn test_format_includes_errors() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Error {
                    message: "Connection timeout".to_string(),
                    error_type: "network".to_string(),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        assert!(result.contains("ERROR"));
        assert!(result.contains("network"));
        assert!(result.contains("Connection timeout"));
    }

    /// Verifies that sub-agent events are included.
    #[test]
    fn test_format_includes_sub_agent_events() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::SubAgentStarted {
                    agent_id: "agent-001".to_string(),
                    agent_name: "analyzer".to_string(),
                    task: "Analyze the codebase".to_string(),
                    depth: 1,
                    parent_request_id: "parent-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::SubAgentCompleted {
                    agent_id: "agent-001".to_string(),
                    response: "Analysis complete".to_string(),
                    duration_ms: 5000,
                    parent_request_id: "parent-1".to_string(),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        assert!(result.contains("SUB_AGENT_STARTED"));
        assert!(result.contains("analyzer"));
        assert!(result.contains("Analyze the codebase"));
        assert!(result.contains("SUB_AGENT_COMPLETED"));
        assert!(result.contains("Analysis complete"));
    }

    /// Verifies that tool approval events are included.
    #[test]
    fn test_format_includes_tool_approval_events() {
        use qbit_core::hitl::RiskLevel;

        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ToolApprovalRequest {
                    request_id: "req-1".to_string(),
                    tool_name: "write_file".to_string(),
                    args: serde_json::json!({"path": "/src/lib.rs"}),
                    stats: None,
                    risk_level: RiskLevel::Medium,
                    can_learn: true,
                    suggestion: None,
                    source: Default::default(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ToolAutoApproved {
                    request_id: "req-2".to_string(),
                    tool_name: "read_file".to_string(),
                    args: serde_json::json!({}),
                    reason: "Always allowed".to_string(),
                    source: Default::default(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ToolDenied {
                    request_id: "req-3".to_string(),
                    tool_name: "shell_exec".to_string(),
                    args: serde_json::json!({}),
                    reason: "Dangerous command".to_string(),
                    source: Default::default(),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        assert!(result.contains("TOOL_APPROVAL_REQUEST"));
        assert!(result.contains("Medium"));
        assert!(result.contains("TOOL_AUTO_APPROVED"));
        assert!(result.contains("Always allowed"));
        assert!(result.contains("TOOL_DENIED"));
        assert!(result.contains("Dangerous command"));
    }

    /// Verifies that internal events like context warnings are skipped.
    #[test]
    fn test_format_excludes_internal_events() {
        let events = vec![
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Started {
                    turn_id: "turn-1".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::ContextWarning {
                    utilization: 0.85,
                    total_tokens: 170000,
                    max_tokens: 200000,
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::LoopWarning {
                    tool_name: "read_file".to_string(),
                    current_count: 8,
                    max_count: 10,
                    message: "Approaching limit".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Reasoning {
                    content: "Let me think...".to_string(),
                },
            },
            TranscriptEvent {
                timestamp: Utc::now(),
                event: AiEvent::Completed {
                    response: "Done".to_string(),
                    input_tokens: Some(100),
                    output_tokens: Some(50),
                    duration_ms: Some(1000),
                },
            },
        ];

        let result = format_for_summarizer(&events);

        // These should NOT appear
        assert!(!result.contains("ContextWarning"));
        assert!(!result.contains("utilization"));
        assert!(!result.contains("LoopWarning"));
        assert!(!result.contains("Approaching limit"));
        assert!(!result.contains("Let me think"));

        // But the final response should appear
        assert!(result.contains("Done"));
    }
}

#[cfg(test)]
mod artifact_tests {
    use super::*;
    use tempfile::TempDir;

    /// Verifies that save_summarizer_input creates a file with the expected content.
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

    /// Verifies that save_summary creates a file with the expected content.
    #[test]
    fn test_save_summary() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-summary";
        let summary = "## Summary\n\nUser asked for help.";

        let path = save_summary(temp_dir.path(), session_id, summary).unwrap();

        assert!(path.exists());
        assert!(path.to_string_lossy().contains("summary-"));
        assert!(path.to_string_lossy().contains(session_id));

        let saved = std::fs::read_to_string(&path).unwrap();
        assert_eq!(saved, summary);
    }

    /// Verifies that save_summarizer_input creates nested directories as needed.
    #[test]
    fn test_save_summarizer_input_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("artifacts").join("compaction");
        let session_id = "test-nested";
        let content = "Test content";

        // Directory doesn't exist yet
        assert!(!nested_dir.exists());

        let path = save_summarizer_input(&nested_dir, session_id, content).unwrap();

        // Directory should now exist
        assert!(nested_dir.exists());
        assert!(path.exists());
    }

    /// Verifies that save_summary creates nested directories as needed.
    #[test]
    fn test_save_summary_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("summaries");
        let session_id = "test-dir";
        let summary = "Summary content";

        assert!(!nested_dir.exists());

        let path = save_summary(&nested_dir, session_id, summary).unwrap();

        assert!(nested_dir.exists());
        assert!(path.exists());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_build_summarizer_input_end_to_end() {
        let temp_dir = TempDir::new().unwrap();
        let session_id = "test-e2e";

        // Create a transcript using the writer
        let writer = TranscriptWriter::new(temp_dir.path(), session_id)
            .await
            .unwrap();

        writer
            .append(&AiEvent::Started {
                turn_id: "turn-1".to_string(),
            })
            .await
            .unwrap();
        writer
            .append(&AiEvent::UserMessage {
                content: "Read the main.rs file".to_string(),
            })
            .await
            .unwrap();
        writer
            .append(&AiEvent::ToolRequest {
                tool_name: "read_file".to_string(),
                args: serde_json::json!({"path": "/src/main.rs"}),
                request_id: "req-1".to_string(),
                source: Default::default(),
            })
            .await
            .unwrap();
        writer
            .append(&AiEvent::ToolResult {
                tool_name: "read_file".to_string(),
                result: serde_json::json!({"content": "fn main() { println!(\"hello\"); }"}),
                success: true,
                request_id: "req-1".to_string(),
                source: Default::default(),
            })
            .await
            .unwrap();
        writer
            .append(&AiEvent::Completed {
                response: "I found the main function.".to_string(),
                input_tokens: Some(200),
                output_tokens: Some(100),
                duration_ms: Some(2000),
            })
            .await
            .unwrap();

        // Now read and format
        let input = build_summarizer_input(temp_dir.path(), session_id).unwrap();

        // Verify the formatted output contains expected content
        assert!(input.contains("[turn 001]"), "Should contain turn marker");
        assert!(input.contains("USER:"), "Should contain user message");
        assert!(
            input.contains("Read the main.rs file"),
            "Should contain user's request"
        );
        assert!(input.contains("read_file"), "Should contain tool name");
        assert!(
            input.contains("TOOL_REQUEST"),
            "Should contain tool request"
        );
        assert!(input.contains("TOOL_RESULT"), "Should contain tool result");
        assert!(
            input.contains("ASSISTANT"),
            "Should contain assistant response"
        );
        assert!(
            input.contains("I found the main function"),
            "Should contain assistant's response"
        );
        assert!(
            input.contains("200 in / 100 out tokens"),
            "Should contain token counts"
        );
    }
}
