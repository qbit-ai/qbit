//! Sub-agent transcript writer for capturing internal sub-agent events.
//!
//! This module provides functionality to persist sub-agent internal events
//! (tool requests and results) to separate transcript files, keeping them
//! separate from the main agent transcript.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use qbit_core::events::AiEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// A wrapper struct for sub-agent transcript entries that includes a timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubAgentTranscriptEntry {
    /// ISO 8601 timestamp when the event was recorded
    _timestamp: DateTime<Utc>,
    /// The AI event (flattened into the same JSON object)
    #[serde(flatten)]
    event: AiEvent,
}

/// Thread-safe writer for sub-agent transcript files.
///
/// Events are stored as a pretty-printed JSON array with timestamps.
#[derive(Debug)]
pub struct SubAgentTranscriptWriter {
    /// Path to the transcript file
    path: PathBuf,
    /// In-memory list of entries, protected by mutex for thread safety
    entries: Arc<Mutex<Vec<SubAgentTranscriptEntry>>>,
}

impl SubAgentTranscriptWriter {
    /// Creates a new `SubAgentTranscriptWriter` for a specific sub-agent execution.
    ///
    /// Path format: `{base_dir}/{session_id}/subagents/{agent_id}-{request_id}/transcript.json`
    ///
    /// # Arguments
    ///
    /// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
    /// * `session_id` - The main session ID
    /// * `agent_id` - The sub-agent identifier
    /// * `parent_request_id` - The request ID that triggered this sub-agent
    pub async fn new(
        base_dir: &Path,
        session_id: &str,
        agent_id: &str,
        parent_request_id: &str,
    ) -> anyhow::Result<Self> {
        let path = sub_agent_transcript_path(base_dir, session_id, agent_id, parent_request_id);

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

    /// Appends an AI event to the sub-agent transcript.
    pub async fn append(&self, event: &AiEvent) -> anyhow::Result<()> {
        let entry = SubAgentTranscriptEntry {
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

/// Constructs the transcript file path for a sub-agent execution.
///
/// # Arguments
///
/// * `base_dir` - The base directory for transcripts (e.g., `~/.qbit/transcripts`)
/// * `session_id` - The main session ID
/// * `agent_id` - The sub-agent identifier
/// * `parent_request_id` - The request ID that triggered this sub-agent
///
/// # Returns
///
/// A `PathBuf` pointing to `{base_dir}/{session_id}/subagents/{agent_id}-{request_id}/transcript.json`
pub fn sub_agent_transcript_path(
    base_dir: &Path,
    session_id: &str,
    agent_id: &str,
    parent_request_id: &str,
) -> PathBuf {
    base_dir
        .join(session_id)
        .join("subagents")
        .join(format!("{}-{}", agent_id, parent_request_id))
        .join("transcript.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sub_agent_transcript_path() {
        let base_dir = Path::new("/var/log/qbit/transcripts");
        let session_id = "session-123";
        let agent_id = "coder";
        let request_id = "req-456";

        let path = sub_agent_transcript_path(base_dir, session_id, agent_id, request_id);

        assert_eq!(
            path,
            PathBuf::from(
                "/var/log/qbit/transcripts/session-123/subagents/coder-req-456/transcript.json"
            )
        );
    }

    #[tokio::test]
    async fn test_sub_agent_transcript_writer_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let writer =
            SubAgentTranscriptWriter::new(temp_dir.path(), "session-001", "analyzer", "req-001")
                .await
                .expect("Failed to create writer");

        // Append an event
        let event = AiEvent::SubAgentToolRequest {
            agent_id: "analyzer".to_string(),
            tool_name: "read_file".to_string(),
            args: serde_json::json!({"path": "/tmp/test.rs"}),
            request_id: "tool-001".to_string(),
            parent_request_id: "req-001".to_string(),
        };
        writer.append(&event).await.expect("Failed to append");

        // Verify file was created
        assert!(writer.path().exists());

        // Verify content
        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read");
        let entries: Vec<serde_json::Value> = serde_json::from_str(&content).expect("Invalid JSON");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["type"], "sub_agent_tool_request");
    }

    #[tokio::test]
    async fn test_sub_agent_transcript_writer_appends_multiple() {
        let temp_dir = TempDir::new().unwrap();
        let writer =
            SubAgentTranscriptWriter::new(temp_dir.path(), "session-002", "coder", "req-002")
                .await
                .expect("Failed to create writer");

        // Append tool request
        let request_event = AiEvent::SubAgentToolRequest {
            agent_id: "coder".to_string(),
            tool_name: "write_file".to_string(),
            args: serde_json::json!({"path": "/tmp/new.rs", "content": "fn main() {}"}),
            request_id: "tool-002".to_string(),
            parent_request_id: "req-002".to_string(),
        };
        writer
            .append(&request_event)
            .await
            .expect("Failed to append request");

        // Append tool result
        let result_event = AiEvent::SubAgentToolResult {
            agent_id: "coder".to_string(),
            tool_name: "write_file".to_string(),
            success: true,
            result: serde_json::json!({"written": true}),
            request_id: "tool-002".to_string(),
            parent_request_id: "req-002".to_string(),
        };
        writer
            .append(&result_event)
            .await
            .expect("Failed to append result");

        // Verify both entries
        let content = tokio::fs::read_to_string(writer.path())
            .await
            .expect("Failed to read");
        let entries: Vec<serde_json::Value> = serde_json::from_str(&content).expect("Invalid JSON");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["type"], "sub_agent_tool_request");
        assert_eq!(entries[1]["type"], "sub_agent_tool_result");
    }
}
