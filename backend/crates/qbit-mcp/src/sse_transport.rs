//! Legacy SSE (Server-Sent Events) transport for MCP protocol version 2024-11-05.
//!
//! The legacy SSE protocol works as follows:
//! 1. Client sends GET to the SSE endpoint (e.g., `/sse`)
//! 2. Server responds with an SSE stream
//! 3. The first event has type `endpoint` with a URL for sending messages
//! 4. Client sends JSON-RPC messages via POST to that endpoint URL
//! 5. Server sends JSON-RPC responses/notifications via the SSE stream as `message` events

use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use futures::stream::Stream;
use futures::{Sink, StreamExt};
use reqwest::Url;
use rmcp::model::JsonRpcMessage;
use rmcp::service::{RoleClient, RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::sink_stream::SinkStreamTransport;
use sse_stream::SseStream;

/// A legacy SSE transport that implements rmcp's Transport trait via SinkStreamTransport.
///
/// This connects to an MCP server using the deprecated HTTP+SSE protocol:
/// - Incoming messages arrive via an SSE event stream (GET)
/// - Outgoing messages are sent via HTTP POST to a server-provided endpoint
pub async fn connect(
    url: &str,
    client: reqwest::Client,
) -> Result<
    SinkStreamTransport<
        impl Sink<TxJsonRpcMessage<RoleClient>, Error = SseTransportError> + Send + Unpin,
        impl Stream<Item = RxJsonRpcMessage<RoleClient>> + Send + Unpin,
    >,
> {
    // 1. Connect to the SSE endpoint
    let response = client
        .get(url)
        .header("Accept", "text/event-stream")
        .send()
        .await
        .map_err(|e| anyhow!("Failed to connect to SSE endpoint: {}", e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "SSE endpoint returned status {}",
            response.status()
        ));
    }

    // 2. Parse the SSE stream from the response body
    let byte_stream = response.bytes_stream();
    let mut sse_stream = SseStream::from_byte_stream(byte_stream);

    // 3. Wait for the `endpoint` event to get the message POST URL
    let endpoint_url = wait_for_endpoint(&mut sse_stream, url).await?;
    tracing::info!("[mcp:sse] Got endpoint URL: {}", endpoint_url);

    // 4. Create the sink (outgoing POST messages)
    let sink = SseSink::new(endpoint_url, client);

    // 5. Create the stream (incoming SSE messages) — filter to only `message` events
    let stream = SseMessageStream::new(sse_stream);

    Ok(SinkStreamTransport::new(sink, stream))
}

/// Wait for the `endpoint` SSE event and extract the message URL.
async fn wait_for_endpoint<S>(sse_stream: &mut S, base_url: &str) -> Result<String>
where
    S: Stream<Item = Result<sse_stream::Sse, sse_stream::Error>> + Unpin,
{
    // The server should send an `endpoint` event fairly quickly
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while let Some(event) = sse_stream.next().await {
            let sse =
                event.map_err(|e| anyhow!("SSE stream error while waiting for endpoint: {}", e))?;

            if sse.event.as_deref() == Some("endpoint") {
                if let Some(data) = sse.data {
                    let endpoint = data.trim().to_string();
                    if endpoint.is_empty() {
                        return Err(anyhow!("Empty endpoint URL in SSE event"));
                    }
                    // The endpoint may be relative or absolute
                    return resolve_endpoint_url(base_url, &endpoint);
                }
                return Err(anyhow!("Endpoint event had no data"));
            }
        }
        Err(anyhow!("SSE stream closed before receiving endpoint event"))
    });

    timeout
        .await
        .map_err(|_| anyhow!("Timed out waiting for endpoint event from SSE server"))?
}

/// Resolve the endpoint URL, handling both absolute and relative URLs.
fn resolve_endpoint_url(base_url: &str, endpoint: &str) -> Result<String> {
    if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        return Ok(endpoint.to_string());
    }

    // Relative URL — resolve against the base
    let base =
        Url::parse(base_url).map_err(|e| anyhow!("Invalid base URL '{}': {}", base_url, e))?;
    let resolved = base
        .join(endpoint)
        .map_err(|e| anyhow!("Failed to resolve endpoint URL '{}': {}", endpoint, e))?;
    Ok(resolved.to_string())
}

/// Error type for SSE transport operations.
#[derive(Debug, thiserror::Error)]
pub enum SseTransportError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

type PendingRequest =
    Pin<Box<dyn std::future::Future<Output = Result<(), SseTransportError>> + Send>>;

/// Sink that sends JSON-RPC messages via HTTP POST to the SSE endpoint.
struct SseSink {
    endpoint_url: String,
    client: reqwest::Client,
    /// In-flight POST request
    pending: Option<PendingRequest>,
}

impl SseSink {
    fn new(endpoint_url: String, client: reqwest::Client) -> Self {
        Self {
            endpoint_url,
            client,
            pending: None,
        }
    }
}

impl Sink<TxJsonRpcMessage<RoleClient>> for SseSink {
    type Error = SseTransportError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        // If there's a pending request, poll it to completion first
        if let Some(fut) = &mut this.pending {
            match fut.as_mut().poll(cx) {
                Poll::Ready(result) => {
                    this.pending = None;
                    Poll::Ready(result)
                }
                Poll::Pending => Poll::Pending,
            }
        } else {
            Poll::Ready(Ok(()))
        }
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> Result<(), Self::Error> {
        let this = self.get_mut();
        let body = serde_json::to_string(&item)?;
        let client = this.client.clone();
        let url = this.endpoint_url.clone();

        this.pending = Some(Box::pin(async move {
            client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;
            Ok(())
        }));

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Flush is the same as poll_ready — wait for pending to complete
        self.poll_ready(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_flush(cx)
    }
}

/// Stream adapter that filters SSE events to only yield JSON-RPC `message` events.
struct SseMessageStream<S> {
    inner: S,
}

impl<S> SseMessageStream<S> {
    fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S> Stream for SseMessageStream<S>
where
    S: Stream<Item = Result<sse_stream::Sse, sse_stream::Error>> + Unpin,
{
    type Item = RxJsonRpcMessage<RoleClient>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(sse))) => {
                    // Only process `message` events (the default event type in SSE)
                    let is_message = sse.event.as_deref() == Some("message")
                        || (sse.event.is_none() && sse.data.is_some());

                    if !is_message {
                        // Skip non-message events (e.g., ping, endpoint)
                        continue;
                    }

                    if let Some(data) = &sse.data {
                        match serde_json::from_str::<JsonRpcMessage<_, _, _>>(data) {
                            Ok(msg) => return Poll::Ready(Some(msg)),
                            Err(e) => {
                                tracing::warn!("[mcp:sse] Failed to parse JSON-RPC message: {}", e);
                                continue;
                            }
                        }
                    }
                    continue;
                }
                Poll::Ready(Some(Err(e))) => {
                    tracing::warn!("[mcp:sse] SSE stream error: {}", e);
                    // Continue on transient errors
                    continue;
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_endpoint_url_absolute() {
        let result =
            resolve_endpoint_url("https://example.com/sse", "https://other.com/messages").unwrap();
        assert_eq!(result, "https://other.com/messages");
    }

    #[test]
    fn test_resolve_endpoint_url_relative() {
        let result =
            resolve_endpoint_url("https://example.com/sse", "/messages?session=abc").unwrap();
        assert_eq!(result, "https://example.com/messages?session=abc");
    }

    #[test]
    fn test_resolve_endpoint_url_relative_no_leading_slash() {
        let result =
            resolve_endpoint_url("https://example.com/api/sse", "messages?session=abc").unwrap();
        assert_eq!(result, "https://example.com/api/messages?session=abc");
    }
}
