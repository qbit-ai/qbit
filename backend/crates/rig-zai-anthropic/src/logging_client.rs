//! Logging HTTP client wrapper for debugging raw API responses.
//!
//! This module provides a wrapper around reqwest::Client that logs all
//! HTTP requests and responses, particularly useful for debugging API
//! compatibility issues with Z.AI's Anthropic-compatible endpoint.
//!
//! Logs are written to tracing output at INFO/DEBUG levels

use bytes::Bytes;
use futures::StreamExt;
use http::{Request, Response};
use rig::http_client::{
    multipart::MultipartForm, Error, HttpClientExt, LazyBody, Result, StreamingResponse,
};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::sse_transformer::SseTransformerStream;

/// A logging HTTP client wrapper that logs all requests and responses.
///
/// This client wraps a reqwest::Client and logs:
/// - Request method, URL, headers, and body
/// - Response status, headers, and body
/// - For streaming responses, logs each chunk as it's received
#[derive(Clone)]
pub struct LoggingClient {
    inner: reqwest::Client,
    request_counter: std::sync::Arc<AtomicU64>,
}

impl LoggingClient {
    /// Create a new logging client with default reqwest::Client.
    pub fn new() -> Self {
        Self {
            inner: reqwest::Client::new(),
            request_counter: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new logging client with a custom reqwest::Client.
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            inner: client,
            request_counter: std::sync::Arc::new(AtomicU64::new(0)),
        }
    }

    fn next_request_id(&self) -> u64 {
        self.request_counter.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for LoggingClient {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LoggingClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggingClient")
            .field("inner", &"reqwest::Client")
            .finish()
    }
}

fn instance_error<E: std::error::Error + Send + Sync + 'static>(error: E) -> Error {
    Error::Instance(Box::new(error))
}

impl HttpClientExt for LoggingClient {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<Output = Result<Response<LazyBody<U>>>> + Send + 'static
    where
        T: Into<Bytes> + Send,
        U: From<Bytes> + Send + 'static,
    {
        let request_id = self.next_request_id();
        let (parts, body) = req.into_parts();
        let body_bytes: Bytes = body.into();

        // Log request
        tracing::debug!(
            request_id = request_id,
            method = %parts.method,
            uri = %parts.uri,
            "ZAI HTTP Request (non-streaming)"
        );
        let body_str = String::from_utf8_lossy(&body_bytes);
        tracing::trace!(
            request_id = request_id,
            headers = ?parts.headers,
            body = %body_str,
            "ZAI HTTP Request details"
        );

        let req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .body(body_bytes.clone());

        async move {
            let response = req.send().await.map_err(instance_error)?;
            let status = response.status();

            tracing::debug!(
                request_id = request_id,
                status = %status,
                "ZAI HTTP Response"
            );

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                tracing::error!(
                    request_id = request_id,
                    status = %status,
                    body = %error_text,
                    "ZAI HTTP Error Response"
                );
                return Err(Error::InvalidStatusCodeWithMessage(status, error_text));
            }

            let mut res = Response::builder().status(status);
            if let Some(hs) = res.headers_mut() {
                *hs = response.headers().clone();
            }

            let body: LazyBody<U> = Box::pin(async move {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| Error::Instance(Box::new(e)))?;

                let body_str = String::from_utf8_lossy(&bytes);
                tracing::trace!(
                    request_id = request_id,
                    body_len = bytes.len(),
                    body = %body_str,
                    "ZAI HTTP Response body"
                );

                // DETAILED LOGGING: Extract and log tool call inputs from non-streaming responses
                // This helps debug malformed tool arguments from Z.AI
                if let Ok(response_json) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    if let Some(content) = response_json.get("content").and_then(|c| c.as_array()) {
                        for item in content {
                            if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                                let tool_name = item
                                    .get("name")
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("unknown");
                                let tool_input = item.get("input");
                                tracing::info!(
                                    request_id = request_id,
                                    tool_name = %tool_name,
                                    tool_input = ?tool_input,
                                    "ZAI Non-streaming tool call detected"
                                );
                            }
                        }
                    }
                }

                Ok(U::from(bytes))
            });

            res.body(body).map_err(Error::Protocol)
        }
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl std::future::Future<Output = Result<Response<LazyBody<U>>>> + Send + 'static
    where
        U: From<Bytes> + Send + 'static,
    {
        let request_id = self.next_request_id();
        let (parts, body) = req.into_parts();
        let body = reqwest::multipart::Form::from(body);

        tracing::debug!(
            request_id = request_id,
            method = %parts.method,
            uri = %parts.uri,
            "ZAI HTTP Multipart Request"
        );

        let req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .multipart(body);

        async move {
            let response = req.send().await.map_err(instance_error)?;
            let status = response.status();

            tracing::debug!(
                request_id = request_id,
                status = %status,
                "ZAI HTTP Multipart Response"
            );

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                tracing::error!(
                    request_id = request_id,
                    status = %status,
                    body = %error_text,
                    "ZAI HTTP Multipart Error"
                );
                return Err(Error::InvalidStatusCodeWithMessage(status, error_text));
            }

            let mut res = Response::builder().status(status);
            if let Some(hs) = res.headers_mut() {
                *hs = response.headers().clone();
            }

            let body: LazyBody<U> = Box::pin(async move {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|e| Error::Instance(Box::new(e)))?;
                Ok(U::from(bytes))
            });

            res.body(body).map_err(Error::Protocol)
        }
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl std::future::Future<Output = Result<StreamingResponse>> + Send
    where
        T: Into<Bytes>,
    {
        let request_id = self.next_request_id();
        let (parts, body) = req.into_parts();
        let body_bytes: Bytes = body.into();

        // Log request with full details for streaming (this is what we need for debugging)
        tracing::info!(
            request_id = request_id,
            method = %parts.method,
            uri = %parts.uri,
            "ZAI Streaming Request"
        );
        let body_str = String::from_utf8_lossy(&body_bytes);
        tracing::debug!(
            request_id = request_id,
            body = %body_str,
            "ZAI Streaming Request body"
        );

        let req = self
            .inner
            .request(parts.method, parts.uri.to_string())
            .headers(parts.headers)
            .body(body_bytes)
            .build()
            .map_err(|x| Error::Instance(x.into()))
            .unwrap();

        let client = self.inner.clone();

        async move {
            let response = client.execute(req).await.map_err(instance_error)?;
            let status = response.status();

            tracing::info!(
                request_id = request_id,
                status = %status,
                "ZAI Streaming Response started"
            );

            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_default();
                tracing::error!(
                    request_id = request_id,
                    status = %status,
                    body = %error_text,
                    "ZAI Streaming Error Response"
                );
                return Err(Error::InvalidStatusCodeWithMessage(status, error_text));
            }

            let response_headers = response.headers().clone();

            // First, apply the SSE transformer to fix malformed JSON from Z.AI
            let byte_stream = response.bytes_stream();
            let transformed_stream = SseTransformerStream::new(byte_stream);

            // Then wrap the stream to log each chunk
            let logged_stream = transformed_stream.map(move |chunk_result| {
                match &chunk_result {
                    Ok(bytes) => {
                        let text = String::from_utf8_lossy(bytes);
                        tracing::info!(
                            request_id = request_id,
                            chunk_len = bytes.len(),
                            chunk = %text,
                            "ZAI Stream chunk received"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            request_id = request_id,
                            error = %e,
                            "ZAI Stream chunk error"
                        );
                    }
                }
                chunk_result.map_err(|e| Error::Instance(Box::new(e)))
            });

            // BoxedStream expects: Pin<Box<dyn WasmCompatSendStream<InnerItem = Result<Bytes>>>>
            // WasmCompatSendStream is implemented for any T: Stream<Item = Result<Bytes, Error>> + Send
            let boxed_stream: Pin<
                Box<dyn rig::wasm_compat::WasmCompatSendStream<InnerItem = Result<Bytes>>>,
            > = Box::pin(logged_stream);

            let mut res = Response::builder().status(status);
            if let Some(hs) = res.headers_mut() {
                *hs = response_headers;
            }

            res.body(boxed_stream).map_err(Error::Protocol)
        }
    }
}
