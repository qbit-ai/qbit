//! Streaming response handling for Gemini Vertex AI.

use async_stream::stream;
use futures::Stream;
use std::pin::Pin;

use crate::error::GeminiVertexError;
use crate::types::{GenerateContentResponse, UsageMetadata};

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
        /// Thought signature (for thinking models)
        signature: Option<String>,
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

/// Parse an SSE line into a GenerateContentResponse.
fn parse_sse_line(line: &str) -> Option<Result<GenerateContentResponse, GeminiVertexError>> {
    let line = line.trim();

    if line.is_empty() || line.starts_with(':') {
        return None;
    }

    // Find data line
    let data_content = line.strip_prefix("data: ")?;
    let data_content = data_content.trim();

    // Skip [DONE] message
    if data_content == "[DONE]" {
        tracing::trace!("SSE: Received [DONE] marker");
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

/// Create a streaming response from a reqwest response.
/// Uses async_stream for proper async yielding of chunks.
pub fn create_stream(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, GeminiVertexError>> + Send>> {
    tracing::debug!(
        "create_stream - content-type: {:?}",
        response.headers().get("content-type")
    );

    let stream = stream! {
        let mut byte_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut last_usage: Option<UsageMetadata> = None;

        use futures::StreamExt;
        while let Some(bytes_result) = byte_stream.next().await {
            match bytes_result {
                Ok(bytes) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        buffer.push_str(text);
                        tracing::trace!("SSE: Received {} bytes, buffer now {} bytes", bytes.len(), buffer.len());
                        // Debug: show what delimiters are present
                        let has_crlf = buffer.contains("\r\n\r\n");
                        let has_lf = buffer.contains("\n\n");
                        tracing::trace!("SSE: Buffer has CRLF: {}, LF: {}", has_crlf, has_lf);

                        // Process complete SSE events (separated by \n\n or \r\n\r\n)
                        // Try \r\n\r\n first (more bytes), then \n\n
                        while let Some((newline_pos, skip_len)) = buffer.find("\r\n\r\n")
                            .map(|p| (p, 4))
                            .or_else(|| buffer.find("\n\n").map(|p| (p, 2)))
                        {
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + skip_len..].to_string();

                            if let Some(result) = parse_sse_line(&line) {
                                match result {
                                    Ok(response) => {
                                        // Store usage metadata
                                        if let Some(usage) = &response.usage_metadata {
                                            last_usage = Some(usage.clone());
                                        }

                                        // Process candidates
                                        if let Some(candidate) = response.candidates.first() {
                                            // Process parts FIRST
                                            for part in &candidate.content.parts {
                                                // Check for thinking content first
                                                if part.thought == Some(true) {
                                                    if let Some(text) = &part.text {
                                                        if !text.is_empty() {
                                                            tracing::trace!("SSE: Yielding thinking chunk: {} chars", text.len());
                                                            yield Ok(StreamChunk::ThinkingDelta {
                                                                thinking: text.clone(),
                                                            });
                                                        }
                                                    }
                                                    continue;
                                                }

                                                // Check for text
                                                if let Some(text) = &part.text {
                                                    if !text.is_empty() {
                                                        tracing::trace!("SSE: Yielding text chunk: {} chars", text.len());
                                                        yield Ok(StreamChunk::TextDelta {
                                                            text: text.clone(),
                                                        });
                                                    }
                                                }

                                                // Check for function call
                                                if let Some(fc) = &part.function_call {
                                                    tracing::trace!("SSE: Yielding function call: {}, has_signature: {}", fc.name, part.thought_signature.is_some());
                                                    yield Ok(StreamChunk::FunctionCall {
                                                        name: fc.name.clone(),
                                                        args: fc.args.clone(),
                                                        signature: part.thought_signature.clone(),
                                                    });
                                                }
                                            }

                                            // Check for finish reason AFTER processing parts
                                            if candidate.finish_reason.is_some() {
                                                tracing::debug!("SSE: Stream finished with reason: {:?}", candidate.finish_reason);
                                                yield Ok(StreamChunk::Done {
                                                    usage: last_usage.clone(),
                                                });
                                                return;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        yield Err(e);
                                        return;
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::warn!("Received non-UTF8 bytes in stream");
                    }
                }
                Err(e) => {
                    tracing::error!("SSE: Stream error: {}", e);
                    yield Err(GeminiVertexError::StreamError(e.to_string()));
                    return;
                }
            }
        }

        // Stream ended - process any remaining buffer
        tracing::debug!("SSE: Stream ended, buffer has {} bytes remaining", buffer.len());
        if !buffer.is_empty() {
            if let Some(result) = parse_sse_line(&buffer) {
                match result {
                    Ok(response) => {
                        if let Some(usage) = &response.usage_metadata {
                            last_usage = Some(usage.clone());
                        }
                        // Process any remaining content
                        if let Some(candidate) = response.candidates.first() {
                            for part in &candidate.content.parts {
                                if part.thought != Some(true) {
                                    if let Some(text) = &part.text {
                                        if !text.is_empty() {
                                            tracing::trace!("SSE: Yielding final text chunk: {} chars", text.len());
                                            yield Ok(StreamChunk::TextDelta {
                                                text: text.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }
        }

        // Send final Done if not already sent
        yield Ok(StreamChunk::Done {
            usage: last_usage,
        });
    };

    Box::pin(stream)
}
