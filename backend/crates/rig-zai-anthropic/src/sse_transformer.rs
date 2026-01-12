//! SSE stream transformer that fixes malformed JSON from Z.AI.
//!
//! This module provides a stream wrapper that intercepts SSE chunks and fixes
//! malformed `partial_json` values before they reach rig's parser.

use bytes::Bytes;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::json_fixer;

/// A stream wrapper that transforms SSE chunks to fix malformed JSON.
///
/// Wraps any byte stream and transforms chunks that contain `input_json_delta`
/// events with malformed `partial_json` values.
pub struct SseTransformerStream<S> {
    inner: Pin<Box<S>>,
}

impl<S> SseTransformerStream<S> {
    /// Create a new transformer stream wrapping the given inner stream.
    pub fn new(inner: S) -> Self {
        Self {
            inner: Box::pin(inner),
        }
    }
}

impl<S, E> Stream for SseTransformerStream<S>
where
    S: Stream<Item = Result<Bytes, E>>,
{
    type Item = Result<Bytes, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                let transformed = transform_sse_chunk(&bytes);
                Poll::Ready(Some(Ok(transformed)))
            }
            other => other,
        }
    }
}

/// Transform an SSE chunk, fixing malformed JSON in partial_json fields.
///
/// Processes the chunk and attempts to fix any malformed JSON in `input_json_delta`
/// events. If the chunk doesn't contain relevant data or can't be fixed, it's
/// passed through unchanged.
fn transform_sse_chunk(bytes: &Bytes) -> Bytes {
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => {
            tracing::trace!("SSE chunk is not valid UTF-8, passing through");
            return bytes.clone();
        }
    };

    // Quick check: does this chunk contain input_json_delta?
    if !text.contains("input_json_delta") {
        return bytes.clone();
    }

    tracing::debug!("SSE chunk contains input_json_delta, checking for malformed JSON");

    // Process each line (SSE format: "data: {...}\n\n")
    let mut result = String::with_capacity(text.len() + 128);
    let mut modified = false;

    // SSE events are separated by double newlines
    for part in text.split("\n\n") {
        if part.is_empty() {
            continue;
        }

        // Check if this part needs fixing
        let fixed_part = if part.contains("input_json_delta") && part.contains("partial_json") {
            fix_sse_event(part, &mut modified)
        } else {
            part.to_string()
        };

        result.push_str(&fixed_part);
        result.push_str("\n\n");
    }

    if modified {
        tracing::info!("Fixed malformed JSON in SSE chunk");
        Bytes::from(result)
    } else {
        bytes.clone()
    }
}

/// Fix an individual SSE event that may contain malformed partial_json.
fn fix_sse_event(event: &str, modified: &mut bool) -> String {
    let mut result = String::with_capacity(event.len() + 64);

    for line in event.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            // Try to fix the partial_json content
            if let Some(fixed) = fix_partial_json_in_event(data) {
                result.push_str("data: ");
                result.push_str(&fixed);
                result.push('\n');
                *modified = true;
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline (will be added by caller)
    result.trim_end_matches('\n').to_string()
}

/// Fix partial_json content within an SSE event JSON.
///
/// Parses the outer event JSON, extracts the partial_json field,
/// fixes any unquoted values, and re-serializes the event.
fn fix_partial_json_in_event(event_json: &str) -> Option<String> {
    // Parse the outer event JSON
    let mut event: serde_json::Value = match serde_json::from_str(event_json) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Failed to parse SSE event JSON: {}", e);
            return None;
        }
    };

    // Navigate to delta.partial_json
    let partial_json = match event.get("delta").and_then(|d| d.get("partial_json")) {
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => {
            tracing::trace!("No partial_json field found in event");
            return None;
        }
    };

    // DETAILED LOGGING: Always log partial_json for debugging Z.AI responses
    tracing::info!(
        "ZAI partial_json received (len={}): {}",
        partial_json.len(),
        &partial_json[..partial_json.len().min(500)]
    );

    // Check if it needs fixing
    if !json_fixer::needs_fixing(&partial_json) {
        tracing::trace!("partial_json doesn't need fixing");
        return None;
    }

    // Fix the partial_json value
    let fixed = json_fixer::fix_unquoted_values(&partial_json);
    tracing::info!(
        "ZAI partial_json FIXED: {} -> {}",
        &partial_json[..partial_json.len().min(200)],
        &fixed[..fixed.len().min(200)]
    );

    // Update the event
    if let Some(delta) = event.get_mut("delta") {
        delta["partial_json"] = serde_json::Value::String(fixed);
    }

    // Re-serialize
    match serde_json::to_string(&event) {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!("Failed to re-serialize fixed event: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn test_transform_non_json_delta() {
        let input = Bytes::from("data: {\"type\":\"text_delta\",\"text\":\"Hello\"}\n\n");
        let output = transform_sse_chunk(&input);
        assert_eq!(input, output);
    }

    #[test]
    fn test_transform_valid_json_delta() {
        let input = Bytes::from(
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":\".\",\"pattern\":\"*\"}"}}"#.to_string() + "\n\n"
        );
        let output = transform_sse_chunk(&input);
        // Should pass through unchanged since partial_json is already valid
        assert_eq!(input, output);
    }

    #[test]
    fn test_transform_malformed_json_delta() {
        let input = Bytes::from(
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":.,\"pattern\":*}"}}"#.to_string() + "\n\n"
        );
        let output = transform_sse_chunk(&input);

        // Should be different (fixed)
        assert_ne!(input, output);

        let output_str = std::str::from_utf8(&output).unwrap();
        assert!(output_str.contains(r#"\"path\":\".\"#));
        assert!(output_str.contains(r#"\"pattern\":\"*\"#));
    }

    #[test]
    fn test_transform_preserves_other_fields() {
        let input = Bytes::from(
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":.,\"recursive\":false}"}}"#.to_string() + "\n\n"
        );
        let output = transform_sse_chunk(&input);
        let output_str = std::str::from_utf8(&output).unwrap();

        // Should preserve the boolean
        assert!(output_str.contains("recursive"));
        assert!(output_str.contains("false"));

        // Should fix the path
        assert!(output_str.contains(r#"\"path\":\".\"#));
    }

    #[test]
    fn test_fix_partial_json_in_event() {
        let event = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":.}"}}"#;
        let fixed = fix_partial_json_in_event(event).unwrap();

        // Parse the fixed event to verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        let partial = parsed["delta"]["partial_json"].as_str().unwrap();
        assert_eq!(partial, r#"{"path":"."}"#);
    }

    #[test]
    fn test_fix_partial_json_preserves_structure() {
        let event = r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"pattern\":*}"}}"#;
        let fixed = fix_partial_json_in_event(event).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&fixed).unwrap();

        // Verify structure is preserved
        assert_eq!(parsed["type"], "content_block_delta");
        assert_eq!(parsed["index"], 2);
        assert_eq!(parsed["delta"]["type"], "input_json_delta");

        // Verify partial_json is fixed
        let partial = parsed["delta"]["partial_json"].as_str().unwrap();
        assert_eq!(partial, r#"{"pattern":"*"}"#);
    }

    #[test]
    fn test_multiple_events_in_chunk() {
        let input = Bytes::from(
            r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"path\":.}"}}

data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"pattern\":*}"}}

"#,
        );

        let output = transform_sse_chunk(&input);
        let output_str = std::str::from_utf8(&output).unwrap();

        // Both should be fixed
        assert!(output_str.contains(r#"\"path\":\".\"#));
        assert!(output_str.contains(r#"\"pattern\":\"*\"#));
    }
}
