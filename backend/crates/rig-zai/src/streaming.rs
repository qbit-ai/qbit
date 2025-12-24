//! Streaming response handling for Z.AI API.
//!
//! This module handles SSE streaming from Z.AI's API, including the
//! `reasoning_content` field for thinking mode.

use crate::types::StreamingCompletionChunk;
use crate::ZaiError;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A streaming response from the Z.AI API.
pub struct StreamingResponse {
    /// The underlying byte stream
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    /// Buffer for incomplete SSE data
    buffer: String,
    /// Whether the stream has completed
    done: bool,
}

impl StreamingResponse {
    /// Create a new streaming response from a reqwest response.
    pub fn new(response: reqwest::Response) -> Self {
        tracing::debug!("Z.AI StreamingResponse::new - creating stream");
        Self {
            inner: Box::pin(response.bytes_stream()),
            buffer: String::new(),
            done: false,
        }
    }

    /// Parse an SSE data line.
    fn parse_sse_data(data: &str) -> Option<Result<StreamingCompletionChunk, ZaiError>> {
        let data = data.trim();

        if data.is_empty() || data == "[DONE]" {
            tracing::debug!("Z.AI SSE: skipping empty or DONE: {:?}", data);
            return None;
        }

        tracing::debug!("Z.AI SSE: parsing chunk: {}", &data[..data.len().min(200)]);

        match serde_json::from_str::<StreamingCompletionChunk>(data) {
            Ok(chunk) => {
                tracing::debug!("Z.AI SSE: parsed chunk with {} choices", chunk.choices.len());
                for (i, choice) in chunk.choices.iter().enumerate() {
                    tracing::debug!(
                        "Z.AI SSE: choice[{}] - reasoning_content: {:?}, content: {:?}, tool_calls: {:?}",
                        i,
                        choice.delta.reasoning_content.as_ref().map(|s| s.len()),
                        choice.delta.content.as_ref().map(|s| s.len()),
                        choice.delta.tool_calls.as_ref().map(|tc| tc.len())
                    );
                }
                Some(Ok(chunk))
            }
            Err(e) => {
                tracing::warn!("Failed to parse Z.AI chunk: {} - data: {}", e, data);
                // Don't yield error for parse failures, skip bad chunks
                None
            }
        }
    }
}

/// A chunk from the Z.AI streaming response.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Reasoning/thinking delta (streamed before content)
    Reasoning(String),
    /// Text content delta
    Text(String),
    /// Tool call start
    ToolCallStart {
        id: String,
        name: String,
        index: u32,
    },
    /// Tool call argument delta
    ToolCallDelta {
        id: String,
        delta: String,
    },
    /// Stream finished
    Done,
}

impl Stream for StreamingResponse {
    type Item = Result<StreamChunk, ZaiError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        loop {
            // Check for complete SSE lines in buffer
            // SSE format: "data: {...}\n\n" or "data: {...}\r\n\r\n"
            let separator_pos = self.buffer.find("\n\n")
                .map(|pos| (pos, 2))
                .or_else(|| self.buffer.find("\r\n\r\n").map(|pos| (pos, 4)));

            if let Some((newline_pos, sep_len)) = separator_pos {
                let line = self.buffer[..newline_pos].to_string();
                self.buffer = self.buffer[newline_pos + sep_len..].to_string();

                tracing::debug!("Z.AI SSE: found complete line ({} chars), buffer remaining: {} chars",
                    line.len(), self.buffer.len());

                // Parse SSE data lines
                for subline in line.lines() {
                    let subline = subline.trim();

                    tracing::trace!("Z.AI SSE subline: {}", &subline[..subline.len().min(100)]);

                    if subline.starts_with("data: ") {
                        let data = &subline[6..]; // Skip "data: "

                        if data == "[DONE]" {
                            tracing::info!("Z.AI stream completed with [DONE]");
                            self.done = true;
                            return Poll::Ready(Some(Ok(StreamChunk::Done)));
                        }

                        if let Some(result) = Self::parse_sse_data(data) {
                            match result {
                                Ok(chunk) => {
                                    // Process the chunk and yield appropriate stream chunks
                                    for choice in chunk.choices {
                                        let delta = &choice.delta;

                                        // Handle reasoning_content (thinking mode)
                                        if let Some(ref reasoning) = delta.reasoning_content {
                                            if !reasoning.is_empty() {
                                                tracing::trace!(
                                                    "Z.AI reasoning chunk: {} chars",
                                                    reasoning.len()
                                                );
                                                return Poll::Ready(Some(Ok(
                                                    StreamChunk::Reasoning(reasoning.clone())
                                                )));
                                            }
                                        }

                                        // Handle regular content
                                        if let Some(ref content) = delta.content {
                                            if !content.is_empty() {
                                                return Poll::Ready(Some(Ok(
                                                    StreamChunk::Text(content.clone())
                                                )));
                                            }
                                        }

                                        // Handle tool calls
                                        if let Some(ref tool_calls) = delta.tool_calls {
                                            for tool_call in tool_calls {
                                                // Tool call start with name
                                                if let Some(ref func) = tool_call.function {
                                                    if let Some(ref name) = func.name {
                                                        let id = tool_call.id.clone().unwrap_or_default();
                                                        return Poll::Ready(Some(Ok(
                                                            StreamChunk::ToolCallStart {
                                                                id,
                                                                name: name.clone(),
                                                                index: tool_call.index,
                                                            }
                                                        )));
                                                    }

                                                    // Tool call argument delta
                                                    if let Some(ref args) = func.arguments {
                                                        if !args.is_empty() {
                                                            let id = tool_call.id.clone().unwrap_or_default();
                                                            return Poll::Ready(Some(Ok(
                                                                StreamChunk::ToolCallDelta {
                                                                    id,
                                                                    delta: args.clone(),
                                                                }
                                                            )));
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        // Handle finish reason
                                        if choice.finish_reason.is_some() {
                                            tracing::debug!(
                                                "Z.AI finish reason: {:?}",
                                                choice.finish_reason
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Poll::Ready(Some(Err(e)));
                                }
                            }
                        }
                    }
                }
                continue;
            }

            // Need more data from the stream
            match Pin::new(&mut self.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                        // Log the raw data received (useful for debugging SSE issues)
                        tracing::debug!(
                            "Z.AI received {} bytes: {}",
                            bytes.len(),
                            text.chars().take(500).collect::<String>()
                        );
                    }
                    // Continue to process the buffer
                }
                Poll::Ready(Some(Err(e))) => {
                    tracing::error!("Z.AI stream error: {}", e);
                    return Poll::Ready(Some(Err(ZaiError::StreamError(e.to_string()))));
                }
                Poll::Ready(None) => {
                    tracing::debug!("Z.AI stream ended");
                    self.done = true;
                    // Process any remaining buffer
                    if !self.buffer.is_empty() {
                        for line in self.buffer.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data != "[DONE]" {
                                    if let Some(Ok(chunk)) = Self::parse_sse_data(data) {
                                        for choice in chunk.choices {
                                            if let Some(ref content) = choice.delta.content {
                                                if !content.is_empty() {
                                                    self.buffer.clear();
                                                    return Poll::Ready(Some(Ok(
                                                        StreamChunk::Text(content.clone())
                                                    )));
                                                }
                                            }
                                        }
                                    }
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

/// Create a streaming response from a reqwest Response.
pub fn process_streaming_response(response: reqwest::Response) -> StreamingResponse {
    StreamingResponse::new(response)
}
