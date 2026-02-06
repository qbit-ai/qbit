//! SSE streaming parser and stream handling for Z.AI API.

use futures::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::ZaiError;
use crate::text_tool_parser;
use crate::types::{ChatCompletionChunk, ChoiceDeltaToolCall, Usage};

/// A streaming response from the Z.AI API.
pub struct StreamingResponse {
    /// The underlying byte stream
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    /// Buffer for incomplete SSE data
    buffer: String,
    /// Whether the stream has completed
    done: bool,
    /// Accumulated tool calls by index
    tool_calls: HashMap<u32, AccumulatedToolCall>,
    /// Final usage (captured from last chunk)
    usage: Option<Usage>,
    /// Accumulated text content for pseudo-XML tool call detection
    text_buffer: String,
    /// Accumulated reasoning content for pseudo-XML tool call detection
    reasoning_buffer: String,
    /// Queued stream chunks to emit (used for pseudo-XML tool calls)
    pending_chunks: Vec<StreamChunk>,
    /// Counter for generating unique tool call IDs for pseudo-XML tool calls
    pseudo_tool_call_counter: u32,
}

/// Accumulated tool call state
#[derive(Debug, Clone, Default)]
pub struct AccumulatedToolCall {
    /// Tool call ID
    pub id: String,
    /// Function name
    pub name: String,
    /// Accumulated arguments JSON
    pub arguments: String,
}

impl StreamingResponse {
    /// Create a new streaming response from a reqwest response.
    pub fn new(response: reqwest::Response) -> Self {
        tracing::debug!(
            "StreamingResponse::new - content-type: {:?}",
            response.headers().get("content-type")
        );
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            done: false,
            tool_calls: HashMap::new(),
            usage: None,
            text_buffer: String::new(),
            reasoning_buffer: String::new(),
            pending_chunks: Vec::new(),
            pseudo_tool_call_counter: 0,
        }
    }

    /// Check for and extract pseudo-XML tool calls from a buffer.
    /// Returns tool call chunks to emit if found, and updates the buffer.
    fn extract_pseudo_xml_tool_calls(
        buffer: &mut String,
        counter: &mut u32,
        source: &str,
    ) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();

        // Only check if we have a complete tool call tag
        if !buffer.contains("</tool_call>") {
            return chunks;
        }

        let (parsed_calls, remaining_text) = text_tool_parser::parse_tool_calls_from_text(buffer);

        if !parsed_calls.is_empty() {
            tracing::info!(
                "Extracted {} pseudo-XML tool call(s) from {} content",
                parsed_calls.len(),
                source
            );

            // Convert parsed calls to AccumulatedToolCall and queue them
            let mut accumulated: Vec<AccumulatedToolCall> = Vec::new();
            for parsed in parsed_calls {
                *counter += 1;
                let id = format!("pseudo_call_{}", counter);

                accumulated.push(AccumulatedToolCall {
                    id,
                    name: parsed.name,
                    arguments: serde_json::to_string(&parsed.arguments).unwrap_or_default(),
                });
            }

            // Emit as a ToolCallsComplete chunk
            chunks.push(StreamChunk::ToolCallsComplete {
                tool_calls: accumulated,
            });

            // Update the buffer with remaining text
            *buffer = remaining_text;
        }

        chunks
    }

    /// Check for pseudo-XML tool calls in text buffer.
    fn check_for_pseudo_xml_tool_calls_in_text(&mut self) -> Vec<StreamChunk> {
        Self::extract_pseudo_xml_tool_calls(
            &mut self.text_buffer,
            &mut self.pseudo_tool_call_counter,
            "text",
        )
    }

    /// Check for pseudo-XML tool calls in reasoning buffer.
    fn check_for_pseudo_xml_tool_calls_in_reasoning(&mut self) -> Vec<StreamChunk> {
        Self::extract_pseudo_xml_tool_calls(
            &mut self.reasoning_buffer,
            &mut self.pseudo_tool_call_counter,
            "reasoning",
        )
    }

    /// Parse an SSE line into a stream chunk.
    fn parse_sse_line(&mut self, line: &str) -> Option<Result<StreamChunk, ZaiError>> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Extract data content
        let data_content = if let Some(content) = line.strip_prefix("data: ") {
            content.trim()
        } else if let Some(content) = line.strip_prefix("data:") {
            content.trim()
        } else {
            tracing::trace!("SSE: No data field in: {}", &line[..line.len().min(100)]);
            return None;
        };

        // Check for [DONE] sentinel
        if data_content.starts_with("[DONE]") {
            tracing::debug!("SSE: Received [DONE] marker");
            self.done = true;
            return Some(Ok(StreamChunk::Done {
                usage: self.usage.take(),
            }));
        }

        // Parse JSON
        match serde_json::from_str::<ChatCompletionChunk>(data_content) {
            Ok(chunk) => {
                // Capture usage if present
                if let Some(ref usage) = chunk.usage {
                    self.usage = Some(usage.clone());
                }
                Some(Ok(self.process_chunk(chunk)))
            }
            Err(e) => {
                // Check if it's an error response
                if let Ok(error_resp) = serde_json::from_str::<serde_json::Value>(data_content) {
                    if let Some(error) = error_resp.get("error") {
                        let message = error
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error")
                            .to_string();
                        let code = error.get("code").and_then(|c| c.as_str()).map(String::from);
                        return Some(Err(ZaiError::Api {
                            status: 0,
                            message,
                            code,
                        }));
                    }
                }
                tracing::warn!(
                    "SSE: Failed to parse chunk: {} - data: {}",
                    e,
                    &data_content[..data_content.len().min(200)]
                );
                Some(Err(ZaiError::Json(e)))
            }
        }
    }

    /// Process a parsed chunk into a stream chunk.
    fn process_chunk(&mut self, chunk: ChatCompletionChunk) -> StreamChunk {
        // Get the first choice (we only handle single choice)
        let choice = match chunk.choices.first() {
            Some(c) => c,
            None => return StreamChunk::Empty,
        };

        // Check for finish reason
        if choice.finish_reason.is_some() {
            // Collect completed tool calls
            let tool_calls: Vec<AccumulatedToolCall> =
                self.tool_calls.drain().map(|(_, v)| v).collect();
            if !tool_calls.is_empty() {
                return StreamChunk::ToolCallsComplete { tool_calls };
            }
            return StreamChunk::Done { usage: chunk.usage };
        }

        let delta = &choice.delta;

        // Handle reasoning content (thinking)
        if let Some(ref reasoning) = delta.reasoning_content {
            if !reasoning.is_empty() {
                // Accumulate reasoning for pseudo-XML tool call detection
                self.reasoning_buffer.push_str(reasoning);

                // Check for pseudo-XML tool calls in accumulated reasoning
                let tool_chunks = self.check_for_pseudo_xml_tool_calls_in_reasoning();
                if !tool_chunks.is_empty() {
                    // Queue the tool calls for emission
                    self.pending_chunks.extend(tool_chunks);
                    // Return first pending chunk
                    if let Some(chunk) = self.pending_chunks.pop() {
                        return chunk;
                    }
                }

                // If no tool calls found, emit as regular reasoning delta
                return StreamChunk::ReasoningDelta {
                    reasoning: reasoning.clone(),
                };
            }
        }

        // Handle text content
        if let Some(ref content) = delta.content {
            if !content.is_empty() {
                // Accumulate text for pseudo-XML tool call detection
                self.text_buffer.push_str(content);

                // Check for pseudo-XML tool calls in accumulated text
                let tool_chunks = self.check_for_pseudo_xml_tool_calls_in_text();
                if !tool_chunks.is_empty() {
                    // Queue the tool calls for emission
                    self.pending_chunks.extend(tool_chunks);
                    // Return first pending chunk
                    if let Some(chunk) = self.pending_chunks.pop() {
                        return chunk;
                    }
                }

                // If no tool calls found, emit as regular text delta
                return StreamChunk::TextDelta {
                    text: content.clone(),
                };
            }
        }

        // Handle tool calls
        if let Some(ref tool_calls) = delta.tool_calls {
            for tc in tool_calls {
                self.accumulate_tool_call(tc);
            }
            // Emit a ToolCallDelta for each tool call delta
            if let Some(tc) = tool_calls.first() {
                if let Some(ref func) = tc.function {
                    if let Some(ref args) = func.arguments {
                        if !args.is_empty() {
                            return StreamChunk::ToolCallDelta {
                                index: tc.index,
                                arguments: args.clone(),
                            };
                        }
                    }
                    // Emit tool call start if we have id and name
                    if let (Some(ref id), Some(ref name)) = (&tc.id, &func.name) {
                        return StreamChunk::ToolCallStart {
                            index: tc.index,
                            id: id.clone(),
                            name: name.clone(),
                        };
                    }
                }
            }
        }

        StreamChunk::Empty
    }

    /// Accumulate tool call deltas by index.
    fn accumulate_tool_call(&mut self, delta: &ChoiceDeltaToolCall) {
        let entry = self.tool_calls.entry(delta.index).or_default();

        // Set ID if present
        if let Some(ref id) = delta.id {
            entry.id = id.clone();
        }

        // Accumulate function details
        if let Some(ref func) = delta.function {
            if let Some(ref name) = func.name {
                entry.name = name.clone();
            }
            if let Some(ref args) = func.arguments {
                entry.arguments.push_str(args);
            }
        }
    }
}

/// A chunk from the streaming response.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content delta
    TextDelta { text: String },
    /// Reasoning content delta (thinking)
    ReasoningDelta { reasoning: String },
    /// Tool call started
    ToolCallStart {
        #[allow(dead_code)]
        index: u32,
        id: String,
        name: String,
    },
    /// Tool call arguments delta
    ToolCallDelta {
        #[allow(dead_code)]
        index: u32,
        arguments: String,
    },
    /// Tool calls completed (all accumulated)
    ToolCallsComplete {
        tool_calls: Vec<AccumulatedToolCall>,
    },
    /// Stream completed
    Done { usage: Option<Usage> },
    /// Error occurred
    #[allow(dead_code)]
    Error { message: String },
    /// Empty chunk (no meaningful content)
    Empty,
}

impl Stream for StreamingResponse {
    type Item = Result<StreamChunk, ZaiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        // First, drain any pending chunks (from pseudo-XML tool call extraction)
        if let Some(chunk) = self.pending_chunks.pop() {
            return Poll::Ready(Some(Ok(chunk)));
        }

        loop {
            // Process complete lines in buffer
            // SSE events are separated by double newlines
            if let Some(newline_pos) = self.buffer.find("\n\n") {
                let line = self.buffer[..newline_pos].to_string();
                self.buffer = self.buffer[newline_pos + 2..].to_string();

                // Try to parse each line in the event block
                for subline in line.split('\n') {
                    if let Some(result) = self.parse_sse_line(subline) {
                        match &result {
                            Ok(StreamChunk::Empty) => continue,
                            Ok(StreamChunk::Done { .. }) => {
                                self.done = true;
                                return Poll::Ready(Some(result));
                            }
                            _ => return Poll::Ready(Some(result)),
                        }
                    }
                }
                continue;
            }

            // Also check for single newline (some servers use \n instead of \n\n)
            if let Some(newline_pos) = self.buffer.find('\n') {
                let line = self.buffer[..newline_pos].to_string();
                self.buffer = self.buffer[newline_pos + 1..].to_string();

                if let Some(result) = self.parse_sse_line(&line) {
                    match &result {
                        Ok(StreamChunk::Empty) => continue,
                        Ok(StreamChunk::Done { .. }) => {
                            self.done = true;
                            return Poll::Ready(Some(result));
                        }
                        _ => return Poll::Ready(Some(result)),
                    }
                }
                continue;
            }

            // Need more data from the stream
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                        tracing::trace!(
                            "Received {} bytes, buffer now {} chars",
                            bytes.len(),
                            self.buffer.len()
                        );
                    } else {
                        tracing::warn!("Received non-UTF-8 bytes: {} bytes", bytes.len());
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    tracing::error!("Stream error: {}", e);
                    return Poll::Ready(Some(Err(ZaiError::Http(e))));
                }
                Poll::Ready(None) => {
                    tracing::debug!(
                        "Stream ended, {} chars remaining in buffer",
                        self.buffer.len()
                    );
                    self.done = true;

                    // Process any remaining buffer
                    if !self.buffer.is_empty() {
                        let remaining = std::mem::take(&mut self.buffer);
                        for line in remaining.split('\n') {
                            if let Some(result) = self.parse_sse_line(line) {
                                match &result {
                                    Ok(StreamChunk::Empty) => continue,
                                    _ => return Poll::Ready(Some(result)),
                                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulated_tool_call() {
        let mut tc = AccumulatedToolCall {
            id: "call_123".to_string(),
            name: "get_weather".to_string(),
            ..Default::default()
        };
        tc.arguments.push_str("{\"location\":");
        tc.arguments.push_str("\"NYC\"}");
        assert_eq!(tc.arguments, "{\"location\":\"NYC\"}");
    }
}
