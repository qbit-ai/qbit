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
