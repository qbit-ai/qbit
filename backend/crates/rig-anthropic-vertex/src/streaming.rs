//! Streaming response handling for Anthropic Vertex AI.

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::AnthropicVertexError;
use crate::types::{ContentDelta, StreamEvent, Usage};

/// A streaming response from the Anthropic Vertex AI API.
pub struct StreamingResponse {
    /// The underlying byte stream
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    /// Buffer for incomplete SSE data
    buffer: String,
    /// Accumulated text content
    accumulated_text: String,
    /// Accumulated thinking signature (for extended thinking)
    accumulated_signature: String,
    /// Whether the stream has completed
    done: bool,
    /// Input tokens from MessageStart (Anthropic sends input_tokens in message_start,
    /// but only output_tokens in message_delta, so we need to track them separately)
    input_tokens: Option<u32>,
}

impl StreamingResponse {
    /// Create a new streaming response from a reqwest response.
    pub fn new(response: reqwest::Response) -> Self {
        tracing::info!("StreamingResponse::new - creating stream from response");
        tracing::debug!(
            "StreamingResponse::new - content-type: {:?}",
            response.headers().get("content-type")
        );
        tracing::debug!(
            "StreamingResponse::new - content-length: {:?}",
            response.headers().get("content-length")
        );
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            accumulated_text: String::new(),
            accumulated_signature: String::new(),
            done: false,
            input_tokens: None,
        }
    }

    /// Parse an SSE line into a stream event.
    ///
    /// SSE format is:
    /// ```text
    /// event: content_block_delta
    /// data: {"type":"content_block_delta",...}
    /// ```
    ///
    /// We must only match `data: ` at the START of a line, not inside JSON content.
    /// This prevents false matches when streamed text contains "data: " strings.
    fn parse_sse_line(line: &str) -> Option<Result<StreamEvent, AnthropicVertexError>> {
        let line = line.trim();

        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Parse SSE properly: find data line that starts at beginning of a line.
        // We take the LAST data: line in case there are multiple (shouldn't happen,
        // but defensive coding against malformed responses).
        let mut data_content: Option<&str> = None;

        for subline in line.split('\n') {
            let subline = subline.trim();
            // Only match "data: " at the START of the line
            if let Some(content) = subline.strip_prefix("data: ") {
                data_content = Some(content);
            }
        }

        let data_content = match data_content {
            Some(d) => d.trim(),
            None => {
                tracing::trace!(
                    "SSE: No data field found in: {}",
                    &line[..line.len().min(100)]
                );
                return None;
            }
        };

        // Skip [DONE] message
        if data_content == "[DONE]" {
            tracing::debug!("SSE: Received [DONE] marker");
            return None;
        }

        match serde_json::from_str::<StreamEvent>(data_content) {
            Ok(ref event) => Some(Ok(event.clone())),
            Err(e) => {
                tracing::warn!(
                    "SSE: Failed to parse event: {} - data: {}",
                    e,
                    &data_content[..data_content.len().min(200)]
                );
                Some(Err(AnthropicVertexError::ParseError(format!(
                    "Failed to parse stream event: {} - data: {}",
                    e, data_content
                ))))
            }
        }
    }
}

/// A chunk from the streaming response.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text delta
    TextDelta {
        text: String,
        /// Accumulated text so far (for convenience)
        #[allow(dead_code)] // Available for consumers who need running total
        accumulated: String,
    },
    /// Thinking/reasoning delta (extended thinking mode)
    ThinkingDelta { thinking: String },
    /// Thinking signature (emitted when signature is complete)
    ThinkingSignature { signature: String },
    /// Tool use started
    ToolUseStart { id: String, name: String },
    /// Tool input delta
    ToolInputDelta { partial_json: String },
    /// Stream completed
    Done {
        /// The reason the stream stopped
        #[allow(dead_code)] // Created for API completeness; pattern matched with `..`
        stop_reason: Option<String>,
        usage: Option<Usage>,
    },
    /// Error occurred
    Error { message: String },

    // Server tool events (Claude's native web_search/web_fetch)
    /// Server tool (web_search/web_fetch) started by Claude
    ServerToolUseStart {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Web search results received from Claude's native web search
    WebSearchResult {
        tool_use_id: String,
        results: serde_json::Value,
    },
    /// Web fetch result received from Claude's native web fetch
    WebFetchResult {
        tool_use_id: String,
        url: String,
        content: serde_json::Value,
    },
}

impl Stream for StreamingResponse {
    type Item = Result<StreamChunk, AnthropicVertexError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            tracing::trace!("poll_next: already done");
            return Poll::Ready(None);
        }

        loop {
            // Check if we have complete lines in the buffer
            if let Some(newline_pos) = self.buffer.find("\n\n") {
                let line = self.buffer[..newline_pos].to_string();
                self.buffer = self.buffer[newline_pos + 2..].to_string();
                tracing::trace!(
                    "poll_next: found SSE line, {} chars remaining in buffer",
                    self.buffer.len()
                );

                if let Some(result) = Self::parse_sse_line(&line) {
                    match result {
                        Ok(event) => {
                            let chunk = self.event_to_chunk(event);
                            if let Some(chunk) = chunk {
                                return Poll::Ready(Some(Ok(chunk)));
                            }
                            // Continue processing if we got a non-yielding event
                            continue;
                        }
                        Err(e) => return Poll::Ready(Some(Err(e))),
                    }
                }
                continue;
            }

            // Need more data from the stream
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    let bytes_len = bytes.len();
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                        tracing::debug!(
                            "poll_next: received {} bytes, buffer now {} chars",
                            bytes_len,
                            self.buffer.len()
                        );
                        // Log first 200 chars of buffer for debugging
                        if self.buffer.len() < 500 {
                            tracing::debug!("poll_next: buffer content: {:?}", self.buffer);
                        }
                    } else {
                        tracing::warn!(
                            "poll_next: received {} bytes but not valid UTF-8",
                            bytes_len
                        );
                    }
                    // Continue to process the buffer
                }
                Poll::Ready(Some(Err(e))) => {
                    tracing::error!("poll_next: stream error: {}", e);
                    return Poll::Ready(Some(Err(AnthropicVertexError::StreamError(
                        e.to_string(),
                    ))));
                }
                Poll::Ready(None) => {
                    tracing::info!(
                        "poll_next: stream ended, buffer has {} chars remaining",
                        self.buffer.len()
                    );
                    if !self.buffer.is_empty() {
                        tracing::debug!(
                            "poll_next: remaining buffer: {:?}",
                            &self.buffer[..self.buffer.len().min(500)]
                        );
                    }
                    self.done = true;
                    // Process any remaining buffer
                    if !self.buffer.is_empty() {
                        if let Some(result) = Self::parse_sse_line(&self.buffer) {
                            self.buffer.clear();
                            match result {
                                Ok(event) => {
                                    if let Some(chunk) = self.event_to_chunk(event) {
                                        return Poll::Ready(Some(Ok(chunk)));
                                    }
                                }
                                Err(e) => return Poll::Ready(Some(Err(e))),
                            }
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl StreamingResponse {
    /// Convert a stream event to a stream chunk.
    fn event_to_chunk(&mut self, event: StreamEvent) -> Option<StreamChunk> {
        let chunk = match event {
            StreamEvent::ContentBlockDelta { delta, index: _ } => match delta {
                ContentDelta::TextDelta { text } => Some(StreamChunk::TextDelta {
                    text,
                    accumulated: self.accumulated_text.clone(),
                }),
                ContentDelta::InputJsonDelta { partial_json } => {
                    Some(StreamChunk::ToolInputDelta { partial_json })
                }
                ContentDelta::ThinkingDelta { thinking } => {
                    Some(StreamChunk::ThinkingDelta { thinking })
                }
                ContentDelta::SignatureDelta { signature } => {
                    // Accumulate signature for later emission
                    self.accumulated_signature.push_str(&signature);
                    None
                }
            },
            StreamEvent::ContentBlockStart {
                content_block,
                index,
            } => {
                match content_block {
                    crate::types::ContentBlock::ToolUse { id, name, .. } => {
                        tracing::info!(
                            "event_to_chunk: ToolUseStart index={} name={}",
                            index,
                            name
                        );
                        Some(StreamChunk::ToolUseStart { id, name })
                    }
                    crate::types::ContentBlock::Thinking { .. } => {
                        tracing::debug!("event_to_chunk: Thinking block start index={}", index);
                        None // Thinking content comes via ThinkingDelta
                    }
                    // Server tool use (Claude's native web_search/web_fetch)
                    crate::types::ContentBlock::ServerToolUse { id, name, input } => {
                        tracing::info!(
                            "event_to_chunk: ServerToolUseStart index={} name={}",
                            index,
                            name
                        );
                        Some(StreamChunk::ServerToolUseStart { id, name, input })
                    }
                    // Web search tool result
                    crate::types::ContentBlock::WebSearchToolResult {
                        tool_use_id,
                        content,
                    } => {
                        tracing::info!(
                            "event_to_chunk: WebSearchToolResult index={} tool_use_id={}",
                            index,
                            tool_use_id
                        );
                        Some(StreamChunk::WebSearchResult {
                            tool_use_id,
                            results: content,
                        })
                    }
                    // Web fetch tool result
                    crate::types::ContentBlock::WebFetchToolResult {
                        tool_use_id,
                        content,
                    } => {
                        // Try to extract URL from content for convenience
                        let url = content
                            .get("url")
                            .and_then(|u| u.as_str())
                            .unwrap_or("")
                            .to_string();
                        tracing::info!(
                            "event_to_chunk: WebFetchToolResult index={} tool_use_id={} url={}",
                            index,
                            tool_use_id,
                            url
                        );
                        Some(StreamChunk::WebFetchResult {
                            tool_use_id,
                            url,
                            content,
                        })
                    }
                    _ => {
                        tracing::debug!(
                            "event_to_chunk: ContentBlockStart index={} (text, skipped)",
                            index
                        );
                        None // Text blocks don't need special handling at start
                    }
                }
            }
            StreamEvent::MessageDelta { delta, usage } => {
                // Use input_tokens from MessageDelta if available (newer API behavior),
                // otherwise fall back to MessageStart value
                let input_tokens = if usage.input_tokens > 0 {
                    usage.input_tokens
                } else {
                    self.input_tokens.unwrap_or(0)
                };
                let combined_usage = Usage {
                    input_tokens,
                    output_tokens: usage.output_tokens,
                    cache_creation_input_tokens: usage.cache_creation_input_tokens,
                    cache_read_input_tokens: usage.cache_read_input_tokens,
                };
                tracing::info!(
                    "event_to_chunk: MessageDelta stop_reason={:?} input_tokens={} output_tokens={}",
                    delta.stop_reason, combined_usage.input_tokens, combined_usage.output_tokens
                );
                self.done = true;
                Some(StreamChunk::Done {
                    stop_reason: delta.stop_reason.map(|r| format!("{:?}", r)),
                    usage: Some(combined_usage),
                })
            }
            StreamEvent::MessageStop => {
                tracing::info!("event_to_chunk: MessageStop");
                self.done = true;
                Some(StreamChunk::Done {
                    stop_reason: None,
                    usage: None,
                })
            }
            StreamEvent::Error { error } => {
                tracing::error!(
                    "event_to_chunk: Error type={} message={}",
                    error.error_type,
                    error.message
                );
                Some(StreamChunk::Error {
                    message: error.message,
                })
            }
            StreamEvent::MessageStart { message } => {
                // Capture input_tokens from MessageStart - Anthropic only sends input_tokens here,
                // and only output_tokens in MessageDelta
                self.input_tokens = Some(message.usage.input_tokens);
                tracing::debug!(
                    "event_to_chunk: MessageStart input_tokens={}",
                    message.usage.input_tokens
                );
                None
            }
            StreamEvent::ContentBlockStop { index } => {
                tracing::debug!("event_to_chunk: ContentBlockStop index={}", index);
                // If we have an accumulated signature, emit it now (thinking block ended)
                if !self.accumulated_signature.is_empty() {
                    let signature = std::mem::take(&mut self.accumulated_signature);
                    tracing::info!(
                        "event_to_chunk: Emitting ThinkingSignature len={}",
                        signature.len()
                    );
                    Some(StreamChunk::ThinkingSignature { signature })
                } else {
                    None
                }
            }
            StreamEvent::Ping => {
                tracing::trace!("event_to_chunk: Ping (skipped)");
                None
            }
        };
        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MessageDeltaContent, StopReason, StreamMessageStart};

    /// Helper to create a mock StreamingResponse for testing event_to_chunk
    fn create_test_response() -> StreamingResponse {
        // We can't easily create a real StreamingResponse without a reqwest::Response,
        // so we'll test the token tracking logic directly
        StreamingResponse {
            inner: Box::pin(futures::stream::empty()),
            buffer: String::new(),
            accumulated_text: String::new(),
            accumulated_signature: String::new(),
            done: false,
            input_tokens: None,
        }
    }

    #[test]
    fn test_message_start_captures_input_tokens() {
        let mut response = create_test_response();

        // Simulate MessageStart event with input_tokens
        let message_start = StreamEvent::MessageStart {
            message: StreamMessageStart {
                id: "msg_123".to_string(),
                message_type: "message".to_string(),
                role: "assistant".to_string(),
                model: "claude-3-5-sonnet".to_string(),
                usage: Usage {
                    input_tokens: 15000,
                    output_tokens: 0, // Output tokens not known yet at message_start
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
            },
        };

        let chunk = response.event_to_chunk(message_start);

        // MessageStart should not produce a chunk (returns None)
        assert!(chunk.is_none());
        // But it should capture the input_tokens
        assert_eq!(response.input_tokens, Some(15000));
    }

    #[test]
    fn test_message_delta_combines_tokens() {
        let mut response = create_test_response();

        // First, simulate MessageStart to capture input_tokens
        response.input_tokens = Some(12500);

        // Now simulate MessageDelta with output_tokens
        let message_delta = StreamEvent::MessageDelta {
            delta: MessageDeltaContent {
                stop_reason: Some(StopReason::EndTurn),
                stop_sequence: None,
            },
            usage: Usage {
                input_tokens: 0, // Anthropic sends 0 here (only output_tokens)
                output_tokens: 450,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
        };

        let chunk = response.event_to_chunk(message_delta);

        // Should produce a Done chunk
        assert!(chunk.is_some());
        if let Some(StreamChunk::Done { usage, .. }) = chunk {
            let usage = usage.expect("Usage should be present");
            // input_tokens should be from MessageStart (12500)
            assert_eq!(usage.input_tokens, 12500);
            // output_tokens should be from MessageDelta (450)
            assert_eq!(usage.output_tokens, 450);
        } else {
            panic!("Expected StreamChunk::Done");
        }
    }

    #[test]
    fn test_message_delta_without_message_start() {
        let mut response = create_test_response();

        // Edge case: MessageDelta arrives without MessageStart
        // (shouldn't happen in practice, but defensive coding)
        let message_delta = StreamEvent::MessageDelta {
            delta: MessageDeltaContent {
                stop_reason: Some(StopReason::EndTurn),
                stop_sequence: None,
            },
            usage: Usage {
                input_tokens: 0,
                output_tokens: 300,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
        };

        let chunk = response.event_to_chunk(message_delta);

        if let Some(StreamChunk::Done { usage, .. }) = chunk {
            let usage = usage.expect("Usage should be present");
            // input_tokens should default to 0 since MessageStart wasn't received
            assert_eq!(usage.input_tokens, 0);
            assert_eq!(usage.output_tokens, 300);
        } else {
            panic!("Expected StreamChunk::Done");
        }
    }

    #[test]
    fn test_full_streaming_sequence_token_tracking() {
        let mut response = create_test_response();

        // Simulate a full streaming sequence:
        // 1. MessageStart (with input_tokens)
        // 2. ContentBlockStart (text)
        // 3. ContentBlockDelta (text chunks)
        // 4. ContentBlockStop
        // 5. MessageDelta (with output_tokens)
        // 6. MessageStop

        // 1. MessageStart
        let _ = response.event_to_chunk(StreamEvent::MessageStart {
            message: StreamMessageStart {
                id: "msg_test".to_string(),
                message_type: "message".to_string(),
                role: "assistant".to_string(),
                model: "claude-3-5-sonnet".to_string(),
                usage: Usage {
                    input_tokens: 8500,
                    output_tokens: 0,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                },
            },
        });
        assert_eq!(response.input_tokens, Some(8500));

        // 5. MessageDelta (final)
        let chunk = response.event_to_chunk(StreamEvent::MessageDelta {
            delta: MessageDeltaContent {
                stop_reason: Some(StopReason::EndTurn),
                stop_sequence: None,
            },
            usage: Usage {
                input_tokens: 0,
                output_tokens: 275,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            },
        });

        // Verify combined usage
        if let Some(StreamChunk::Done { usage, .. }) = chunk {
            let usage = usage.expect("Usage should be present");
            assert_eq!(usage.input_tokens, 8500, "input_tokens from MessageStart");
            assert_eq!(usage.output_tokens, 275, "output_tokens from MessageDelta");
        } else {
            panic!("Expected StreamChunk::Done");
        }
    }

    #[test]
    fn test_message_delta_with_input_tokens() {
        let mut response = create_test_response();

        // Simulate MessageStart capturing initial input_tokens
        response.input_tokens = Some(5000);

        // Newer API behavior: MessageDelta includes input_tokens
        // This should take precedence over the MessageStart value
        let message_delta = StreamEvent::MessageDelta {
            delta: MessageDeltaContent {
                stop_reason: Some(StopReason::EndTurn),
                stop_sequence: None,
            },
            usage: Usage {
                input_tokens: 15672, // Non-zero, should be used
                output_tokens: 408,
            },
        };

        let chunk = response.event_to_chunk(message_delta);

        if let Some(StreamChunk::Done { usage, .. }) = chunk {
            let usage = usage.expect("Usage should be present");
            // input_tokens should come from MessageDelta (15672), not MessageStart (5000)
            assert_eq!(usage.input_tokens, 15672);
            assert_eq!(usage.output_tokens, 408);
        } else {
            panic!("Expected StreamChunk::Done");
        }
    }

    #[test]
    fn test_usage_struct_serialization() {
        // Test that Usage struct serializes/deserializes correctly
        let usage = Usage {
            input_tokens: 50000,
            output_tokens: 1500,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("\"input_tokens\":50000"));
        assert!(json.contains("\"output_tokens\":1500"));

        let parsed: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.input_tokens, 50000);
        assert_eq!(parsed.output_tokens, 1500);
    }

    #[test]
    fn test_usage_default_for_missing_fields() {
        // Anthropic sometimes omits input_tokens in message_delta
        // Verify serde(default) works correctly
        let json = r#"{"output_tokens": 200}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.input_tokens, 0); // default
        assert_eq!(usage.output_tokens, 200);
    }
}
