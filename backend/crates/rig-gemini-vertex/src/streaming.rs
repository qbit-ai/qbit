//! Streaming response handling for Gemini Vertex AI.

use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::error::GeminiVertexError;
use crate::types::{GenerateContentResponse, UsageMetadata};

/// A streaming response from the Gemini Vertex AI API.
pub struct StreamingResponse {
    /// The underlying byte stream
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    /// Buffer for incomplete SSE data
    buffer: String,
    /// Whether the stream has completed
    done: bool,
    /// Last usage metadata received
    last_usage: Option<UsageMetadata>,
    /// Pending text to yield (for final chunk handling)
    pending_text: Option<String>,
    /// Whether we've sent the final Done chunk
    sent_done: bool,
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
            last_usage: None,
            pending_text: None,
            sent_done: false,
        }
    }

    /// Parse an SSE line into a GenerateContentResponse.
    ///
    /// SSE format for Gemini streaming:
    /// ```text
    /// data: {"candidates":[...],"usageMetadata":{...}}
    /// ```
    fn parse_sse_line(line: &str) -> Option<Result<GenerateContentResponse, GeminiVertexError>> {
        let line = line.trim();

        if line.is_empty() || line.starts_with(':') {
            return None;
        }

        // Find data line
        let mut data_content: Option<&str> = None;

        for subline in line.split('\n') {
            let subline = subline.trim();
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

        match serde_json::from_str::<GenerateContentResponse>(data_content) {
            Ok(response) => Some(Ok(response)),
            Err(e) => {
                tracing::warn!(
                    "SSE: Failed to parse response: {} - data: {}",
                    e,
                    &data_content[..data_content.len().min(200)]
                );
                Some(Err(GeminiVertexError::ParseError(format!(
                    "Failed to parse stream response: {} - data: {}",
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
        /// The text content
        text: String,
    },
    /// Function call
    FunctionCall {
        /// Function name
        name: String,
        /// Function arguments
        args: serde_json::Value,
    },
    /// Thinking content (for reasoning models)
    ThinkingDelta {
        /// The thinking content
        thinking: String,
    },
    /// Stream completed
    Done {
        /// Usage metadata
        usage: Option<UsageMetadata>,
    },
}

impl Stream for StreamingResponse {
    type Item = Result<StreamChunk, GeminiVertexError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // If we've already sent Done, we're finished
        if self.sent_done {
            return Poll::Ready(None);
        }

        // Check if we have pending text to yield first
        if let Some(text) = self.pending_text.take() {
            return Poll::Ready(Some(Ok(StreamChunk::TextDelta { text })));
        }

        // If stream is done but we haven't sent Done yet
        if self.done {
            self.sent_done = true;
            return Poll::Ready(Some(Ok(StreamChunk::Done {
                usage: self.last_usage.clone(),
            })));
        }

        loop {
            // Check if we have complete lines in the buffer
            if let Some(newline_pos) = self.buffer.find("\n\n") {
                let line = self.buffer[..newline_pos].to_string();
                self.buffer = self.buffer[newline_pos + 2..].to_string();
                tracing::debug!("SSE: Found line (len={}): {:?}", line.len(), &line[..line.len().min(200)]);

                if let Some(result) = Self::parse_sse_line(&line) {
                    match result {
                        Ok(response) => {
                            // Store usage metadata
                            if let Some(usage) = &response.usage_metadata {
                                self.last_usage = Some(usage.clone());
                            }

                            // Process candidates
                            if let Some(candidate) = response.candidates.first() {
                                // Process parts FIRST (before checking finish_reason)
                                // because the final chunk may contain both text AND finish_reason
                                for part in &candidate.content.parts {
                                    // Check for thinking content first
                                    if part.thought == Some(true) {
                                        if let Some(text) = &part.text {
                                            if !text.is_empty() {
                                                return Poll::Ready(Some(Ok(
                                                    StreamChunk::ThinkingDelta {
                                                        thinking: text.clone(),
                                                    },
                                                )));
                                            }
                                        }
                                        continue;
                                    }

                                    // Check for text
                                    if let Some(text) = &part.text {
                                        if !text.is_empty() {
                                            return Poll::Ready(Some(Ok(StreamChunk::TextDelta {
                                                text: text.clone(),
                                            })));
                                        }
                                    }

                                    // Check for function call
                                    if let Some(fc) = &part.function_call {
                                        return Poll::Ready(Some(Ok(StreamChunk::FunctionCall {
                                            name: fc.name.clone(),
                                            args: fc.args.clone(),
                                        })));
                                    }
                                }

                                // Check for finish reason AFTER processing parts
                                if candidate.finish_reason.is_some() {
                                    self.done = true;
                                    return Poll::Ready(Some(Ok(StreamChunk::Done {
                                        usage: self.last_usage.clone(),
                                    })));
                                }
                            }
                            // Continue processing if no yielding event
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
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        tracing::debug!("SSE: Received {} bytes: {:?}", bytes.len(), &text[..text.len().min(200)]);
                        self.buffer.push_str(text);
                        tracing::debug!("SSE: Buffer now {} bytes", self.buffer.len());
                    } else {
                        tracing::warn!("Received non-UTF8 bytes in stream");
                    }
                    // Continue to process the buffer
                }
                Poll::Ready(Some(Err(e))) => {
                    self.done = true;
                    return Poll::Ready(Some(Err(GeminiVertexError::StreamError(e.to_string()))));
                }
                Poll::Ready(None) => {
                    // Stream ended
                    self.done = true;
                    tracing::debug!("SSE: Stream ended, buffer has {} bytes remaining", self.buffer.len());

                    // Process any remaining data in buffer
                    if !self.buffer.is_empty() {
                        tracing::debug!("SSE: Processing remaining buffer: {:?}", &self.buffer[..self.buffer.len().min(200)]);
                        if let Some(result) = Self::parse_sse_line(&self.buffer) {
                            self.buffer.clear();
                            match result {
                                Ok(response) => {
                                    if let Some(usage) = &response.usage_metadata {
                                        self.last_usage = Some(usage.clone());
                                    }
                                    // Process candidates and yield text content before Done
                                    if let Some(candidate) = response.candidates.first() {
                                        for part in &candidate.content.parts {
                                            if let Some(text) = &part.text {
                                                if !text.is_empty() && part.thought != Some(true) {
                                                    tracing::debug!("SSE: Yielding final text chunk: {} chars", text.len());
                                                    // Return the text now, Done will be sent on next poll
                                                    return Poll::Ready(Some(Ok(StreamChunk::TextDelta {
                                                        text: text.clone(),
                                                    })));
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => return Poll::Ready(Some(Err(e))),
                            }
                        }
                    }

                    // Send Done on next poll
                    self.sent_done = true;
                    return Poll::Ready(Some(Ok(StreamChunk::Done {
                        usage: self.last_usage.clone(),
                    })));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}
