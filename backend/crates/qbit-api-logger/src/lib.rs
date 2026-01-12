//! Raw LLM API request/response file logger.
//!
//! This module provides a global singleton logger for capturing raw LLM API
//! request/response JSON data. When enabled, it writes JSONL-formatted logs
//! to session-specific files in the configured directory.
//!
//! # Usage
//!
//! Configure the logger at session initialization:
//! ```ignore
//! use qbit_api_logger::API_LOGGER;
//!
//! API_LOGGER.configure(true, PathBuf::from("./logs/api"), "session-123".to_string());
//! ```
//!
//! Log from provider code:
//! ```ignore
//! API_LOGGER.log_request("zai", "glm-4.7", &request_json);
//! API_LOGGER.log_sse_chunk("zai", &raw_sse_data);
//! API_LOGGER.log_response("zai", "glm-4.7", &response_json);
//! ```

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use parking_lot::RwLock;
use serde::Serialize;

/// Global API logging state (thread-safe singleton pattern)
pub static API_LOGGER: once_cell::sync::Lazy<ApiLoggerState> =
    once_cell::sync::Lazy::new(ApiLoggerState::default);

/// Thread-safe API logger state.
pub struct ApiLoggerState {
    enabled: AtomicBool,
    extract_raw_sse: AtomicBool,
    log_dir: RwLock<Option<PathBuf>>,
    session_id: RwLock<Option<String>>,
}

impl Default for ApiLoggerState {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            extract_raw_sse: AtomicBool::new(false),
            log_dir: RwLock::new(None),
            session_id: RwLock::new(None),
        }
    }
}

/// A single log entry in JSONL format.
#[derive(Debug, Serialize)]
struct LogEntry<'a> {
    timestamp: String,
    #[serde(rename = "type")]
    entry_type: &'a str,
    provider: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    raw: Option<&'a str>,
}

/// Parse an SSE chunk into event type and JSON data.
///
/// SSE format is:
/// ```text
/// event: content_block_delta
/// data: {"type":"content_block_delta",...}
///
/// ```
///
/// Returns (event_type, parsed_json) if successful.
fn parse_sse_chunk(chunk: &str) -> Option<(&str, serde_json::Value)> {
    let mut event_type: Option<&str> = None;
    let mut data_content: Option<&str> = None;

    for line in chunk.lines() {
        let line = line.trim();
        if let Some(evt) = line.strip_prefix("event:") {
            event_type = Some(evt.trim());
        } else if let Some(data) = line.strip_prefix("data:") {
            data_content = Some(data.trim());
        }
    }

    let event = event_type?;
    let data_str = data_content?;

    // Parse the JSON data
    let data_json: serde_json::Value = serde_json::from_str(data_str).ok()?;

    Some((event, data_json))
}

impl ApiLoggerState {
    /// Configure the API logger.
    ///
    /// # Arguments
    /// * `enabled` - Whether logging is enabled
    /// * `extract_raw_sse` - Whether to parse SSE chunks as JSON instead of escaped strings
    /// * `log_dir` - Directory to write log files (e.g., `./logs/api`)
    /// * `session_id` - Current session ID for log file naming
    pub fn configure(
        &self,
        enabled: bool,
        extract_raw_sse: bool,
        log_dir: PathBuf,
        session_id: String,
    ) {
        self.enabled.store(enabled, Ordering::SeqCst);
        self.extract_raw_sse
            .store(extract_raw_sse, Ordering::SeqCst);
        *self.log_dir.write() = Some(log_dir);
        *self.session_id.write() = Some(session_id);

        if enabled {
            tracing::info!(
                "API logging enabled (extract_raw_sse={}), logs will be written to ./logs/api/",
                extract_raw_sse
            );
        }
    }

    /// Check if API logging is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Check if raw SSE extraction is enabled.
    pub fn should_extract_raw_sse(&self) -> bool {
        self.extract_raw_sse.load(Ordering::SeqCst)
    }

    /// Log a raw request JSON to file.
    ///
    /// Call this before sending the HTTP request to the LLM API.
    pub fn log_request(&self, provider: &str, model: &str, request_json: &serde_json::Value) {
        if !self.is_enabled() {
            return;
        }

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            entry_type: "request",
            provider,
            model: Some(model),
            event: None,
            data: Some(request_json),
            raw: None,
        };

        self.write_entry(&entry);
    }

    /// Log a raw SSE chunk to file.
    ///
    /// Call this for each SSE chunk received from the LLM API during streaming.
    /// If `extract_raw_sse` is enabled, the chunk will be parsed as structured
    /// SSE data (event type + JSON data) instead of an escaped string.
    pub fn log_sse_chunk(&self, provider: &str, chunk: &str) {
        if !self.is_enabled() {
            return;
        }

        if self.should_extract_raw_sse() {
            // Try to parse as SSE format: "event: <type>\ndata: <json>\n\n"
            if let Some((event_type, data_json)) = parse_sse_chunk(chunk) {
                let entry = LogEntry {
                    timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    entry_type: "sse_chunk",
                    provider,
                    model: None,
                    event: Some(event_type),
                    data: Some(&data_json),
                    raw: None,
                };
                self.write_entry(&entry);
                return;
            }
            // Fall through to raw logging if parsing fails
        }

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            entry_type: "sse_chunk",
            provider,
            model: None,
            event: None,
            data: None,
            raw: Some(chunk),
        };

        self.write_entry(&entry);
    }

    /// Log accumulated response data to file.
    ///
    /// Call this after the streaming response is complete with summary data.
    pub fn log_response(&self, provider: &str, model: &str, response_json: &serde_json::Value) {
        if !self.is_enabled() {
            return;
        }

        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            entry_type: "response",
            provider,
            model: Some(model),
            event: None,
            data: Some(response_json),
            raw: None,
        };

        self.write_entry(&entry);
    }

    /// Log an error that occurred during API interaction.
    pub fn log_error(&self, provider: &str, error: &str) {
        if !self.is_enabled() {
            return;
        }

        let error_json = serde_json::json!({ "error": error });
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            entry_type: "error",
            provider,
            model: None,
            event: None,
            data: Some(&error_json),
            raw: None,
        };

        self.write_entry(&entry);
    }

    /// Log a stream completion event (for tracking when streams end normally vs abnormally).
    pub fn log_stream_end(&self, provider: &str, reason: &str) {
        if !self.is_enabled() {
            return;
        }

        let reason_json = serde_json::json!({ "reason": reason });
        let entry = LogEntry {
            timestamp: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            entry_type: "stream_end",
            provider,
            model: None,
            event: None,
            data: Some(&reason_json),
            raw: None,
        };

        self.write_entry(&entry);
    }

    fn write_entry(&self, entry: &LogEntry) {
        let log_dir = self.log_dir.read();
        let session_id = self.session_id.read();

        let Some(dir) = log_dir.as_ref() else {
            return;
        };
        let session = session_id.as_deref().unwrap_or("default");

        // Create directory if needed
        if let Err(e) = fs::create_dir_all(dir) {
            tracing::warn!("Failed to create API log directory: {}", e);
            return;
        }

        let file_path = dir.join(format!("{}.jsonl", session));

        let json_line = match serde_json::to_string(entry) {
            Ok(json) => json,
            Err(e) => {
                tracing::warn!("Failed to serialize API log entry: {}", e);
                return;
            }
        };

        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
        {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", json_line) {
                    tracing::warn!("Failed to write API log entry: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to open API log file {:?}: {}", file_path, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_logging_when_disabled() {
        // When disabled, logging should be a no-op
        let logger = ApiLoggerState::default();
        assert!(!logger.is_enabled());

        // These should not panic even without configuration
        logger.log_request("test", "model", &serde_json::json!({}));
        logger.log_sse_chunk("test", "data: test");
        logger.log_response("test", "model", &serde_json::json!({}));
    }

    #[test]
    fn test_logging_when_enabled() {
        let temp_dir = tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs/api");

        let logger = ApiLoggerState::default();
        logger.configure(true, false, log_dir.clone(), "test-session".to_string());

        assert!(logger.is_enabled());
        assert!(!logger.should_extract_raw_sse());

        // Log some entries
        logger.log_request("zai", "glm-4.7", &serde_json::json!({"messages": []}));
        logger.log_sse_chunk("zai", "{\"choices\":[]}");
        logger.log_response("zai", "glm-4.7", &serde_json::json!({"done": true}));

        // Check file was created
        let log_file = log_dir.join("test-session.jsonl");
        assert!(log_file.exists());

        // Check content
        let content = fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);

        // Verify JSON structure
        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["type"], "request");
        assert_eq!(first["provider"], "zai");

        // Verify SSE chunk is logged as raw string
        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["type"], "sse_chunk");
        assert!(second.get("raw").is_some());
        assert!(second.get("data").is_none());
    }

    #[test]
    fn test_logging_with_extract_raw_sse() {
        let temp_dir = tempdir().unwrap();
        let log_dir = temp_dir.path().join("logs/api");

        let logger = ApiLoggerState::default();
        logger.configure(
            true,
            true,
            log_dir.clone(),
            "test-session-extract".to_string(),
        );

        assert!(logger.is_enabled());
        assert!(logger.should_extract_raw_sse());

        // Log SSE chunk in proper SSE format
        logger.log_sse_chunk(
            "zai",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"text\":\"hello\"}}\n\n",
        );

        // Check file was created
        let log_file = log_dir.join("test-session-extract.jsonl");
        assert!(log_file.exists());

        // Check content - should have parsed data with event type
        let content = fs::read_to_string(&log_file).unwrap();
        let entry: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(entry["type"], "sse_chunk");
        assert_eq!(entry["event"], "content_block_delta");
        assert!(entry.get("data").is_some());
        assert!(entry.get("raw").is_none());
        assert_eq!(entry["data"]["delta"]["text"], "hello");
    }

    #[test]
    fn test_parse_sse_chunk() {
        // Test parsing valid SSE chunk
        let chunk =
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"123\"}}\n\n";
        let result = parse_sse_chunk(chunk);
        assert!(result.is_some());

        let (event, data) = result.unwrap();
        assert_eq!(event, "message_start");
        assert_eq!(data["type"], "message_start");
        assert_eq!(data["message"]["id"], "123");
    }

    #[test]
    fn test_parse_sse_chunk_invalid() {
        // Test parsing invalid SSE chunk (no event line)
        let chunk = "data: {\"type\":\"test\"}\n\n";
        let result = parse_sse_chunk(chunk);
        assert!(result.is_none());

        // Test parsing chunk with invalid JSON
        let chunk = "event: test\ndata: not-json\n\n";
        let result = parse_sse_chunk(chunk);
        assert!(result.is_none());
    }
}
