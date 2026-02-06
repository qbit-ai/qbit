//! Localhost OAuth callback server.

use anyhow::{anyhow, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Result from the OAuth callback.
#[derive(Debug)]
pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

/// Start a localhost callback server on an ephemeral port.
///
/// Returns (port, receiver) where the receiver will get the callback result.
pub async fn start_callback_server() -> Result<(u16, oneshot::Receiver<Result<CallbackResult>>)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("Failed to bind callback server")?;

    let port = listener.local_addr()?.port();
    tracing::debug!("OAuth callback server listening on port {}", port);

    let (tx, rx) = oneshot::channel();

    tokio::spawn(async move {
        let result = handle_callback(listener).await;
        let _ = tx.send(result);
    });

    Ok((port, rx))
}

async fn handle_callback(listener: TcpListener) -> Result<CallbackResult> {
    let (mut socket, addr) = listener
        .accept()
        .await
        .context("Failed to accept connection")?;

    tracing::debug!("Accepted callback connection from {}", addr);

    let mut buffer = vec![0u8; 4096];
    let n = socket
        .read(&mut buffer)
        .await
        .context("Failed to read from socket")?;

    let request = String::from_utf8_lossy(&buffer[..n]);
    tracing::debug!("Received callback request:\n{}", request);

    // Parse the HTTP request line to get the path with query parameters
    let first_line = request
        .lines()
        .next()
        .ok_or_else(|| anyhow!("Empty request"))?;

    let parts: Vec<&str> = first_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("Invalid HTTP request line"));
    }

    let path = parts[1];

    // Parse query parameters
    let query = if let Some(idx) = path.find('?') {
        &path[idx + 1..]
    } else {
        return Err(anyhow!("No query parameters in callback"));
    };

    let mut code = None;
    let mut state = None;

    for param in query.split('&') {
        if let Some((key, value)) = param.split_once('=') {
            match key {
                "code" => code = Some(urlencoding::decode(value)?.to_string()),
                "state" => state = Some(urlencoding::decode(value)?.to_string()),
                _ => {}
            }
        }
    }

    let code = code.ok_or_else(|| anyhow!("Missing 'code' parameter"))?;
    let state = state.ok_or_else(|| anyhow!("Missing 'state' parameter"))?;

    // Send success response
    let response = "HTTP/1.1 200 OK\r\n\
                   Content-Type: text/html; charset=utf-8\r\n\
                   Connection: close\r\n\
                   \r\n\
                   <!DOCTYPE html>\
                   <html>\
                   <head><title>Authentication Successful</title></head>\
                   <body>\
                   <h1>✓ Authentication Successful</h1>\
                   <p>You have successfully authenticated with Qbit.</p>\
                   <p>You can close this window and return to your terminal.</p>\
                   </body>\
                   </html>";

    socket
        .write_all(response.as_bytes())
        .await
        .context("Failed to write response")?;

    socket.flush().await.context("Failed to flush socket")?;

    tracing::info!("OAuth callback completed successfully");

    Ok(CallbackResult { code, state })
}
