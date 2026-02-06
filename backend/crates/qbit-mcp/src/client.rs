// Allow manual async fn in trait implementations (required by rmcp trait)
#![allow(clippy::manual_async_fn)]

use anyhow::{anyhow, Context, Result};
use rmcp::handler::client::ClientHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, Content, Implementation,
    InitializeRequestParams, RawContent,
};
use rmcp::service::{self, NotificationContext, RequestContext, RoleClient, RunningService};
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
};
use rmcp::ServiceExt;
#[cfg(unix)]
use std::sync::OnceLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::config::{McpServerConfig, McpTransportType};
use crate::loader::interpolate_env_vars;
use crate::manager::{McpToolResult, McpToolResultContent};
use crate::tools::McpTool;

pub type McpClientConnection = RunningService<RoleClient, McpClientHandler>;

#[derive(Clone)]
pub struct McpClientHandler {
    server_name: String,
    tool_sender: tokio::sync::mpsc::UnboundedSender<(String, Vec<String>)>,
}

impl McpClientHandler {
    pub fn new(
        server_name: String,
        tool_sender: tokio::sync::mpsc::UnboundedSender<(String, Vec<String>)>,
    ) -> Self {
        Self {
            server_name,
            tool_sender,
        }
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

impl ClientHandler for McpClientHandler {
    fn on_tool_list_changed(
        &self,
        context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let server_name = self.server_name.clone();
        let tool_sender = self.tool_sender.clone();
        async move {
            match context.peer.list_all_tools().await {
                Ok(result) => {
                    let tools = result
                        .into_iter()
                        .map(|tool| tool.name.to_string())
                        .collect();
                    let _ = tool_sender.send((server_name, tools));
                }
                Err(err) => {
                    tracing::warn!("Failed to refresh MCP tools list: {}", err);
                }
            }
        }
    }

    fn on_progress(
        &self,
        _params: rmcp::model::ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn on_resource_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn on_prompt_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        async move {}
    }

    fn list_roots(
        &self,
        _context: RequestContext<RoleClient>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListRootsResult, rmcp::ErrorData>>
           + Send
           + '_ {
        async move { Ok(rmcp::model::ListRootsResult { roots: vec![] }) }
    }

    fn get_info(&self) -> InitializeRequestParams {
        InitializeRequestParams {
            meta: None,
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "qbit".to_string(),
                title: None,
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
        }
    }
}

pub async fn connect_mcp_server(
    name: &str,
    config: &McpServerConfig,
    tool_sender: tokio::sync::mpsc::UnboundedSender<(String, Vec<String>)>,
) -> Result<McpClientConnection> {
    let handler = McpClientHandler::new(name.to_string(), tool_sender);

    match config.transport() {
        McpTransportType::Stdio => connect_stdio(config, handler).await,
        McpTransportType::Http => connect_http(config, handler).await,
        McpTransportType::Sse => connect_sse(config, handler).await,
    }
}

/// Resolves the user's shell PATH by spawning a login shell.
/// Cached via OnceLock — only runs once per app lifetime.
/// Returns None if resolution fails (the inherited PATH will be used).
#[cfg(unix)]
fn resolve_shell_path() -> Option<&'static str> {
    static SHELL_PATH: OnceLock<Option<String>> = OnceLock::new();

    SHELL_PATH
        .get_or_init(|| {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| {
                if cfg!(target_os = "macos") {
                    "/bin/zsh".to_string()
                } else {
                    "/bin/sh".to_string()
                }
            });

            let output = match std::process::Command::new(&shell)
                .args(["-lic", "echo __QBIT_PATH_MARKER__=$PATH"])
                .output()
            {
                Ok(output) => output,
                Err(e) => {
                    tracing::warn!("Failed to spawn login shell to resolve PATH: {}", e);
                    return None;
                }
            };

            if !output.status.success() {
                tracing::warn!(
                    "Login shell exited with status {} while resolving PATH",
                    output.status
                );
                return None;
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Some(path) = line.strip_prefix("__QBIT_PATH_MARKER__=") {
                    let path = path.trim().to_string();
                    tracing::debug!("Resolved shell PATH: {}", path);
                    return Some(path);
                }
            }

            tracing::warn!("Failed to extract PATH from login shell output");
            None
        })
        .as_ref()
        .map(|s| s.as_str())
}

/// Returns the path to ~/.qbit/mcp-logs.log, creating the directory if needed.
async fn mcp_log_file() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let qbit_dir = home.join(".qbit");
    if !qbit_dir.exists() {
        if let Err(e) = tokio::fs::create_dir_all(&qbit_dir).await {
            tracing::warn!("Failed to create ~/.qbit directory: {}", e);
            return None;
        }
    }
    Some(qbit_dir.join("mcp-logs.log"))
}

/// Append a line to the log file.
async fn append_to_log(path: &std::path::Path, line: &str) {
    use tokio::io::AsyncWriteExt;
    match tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
    {
        Ok(mut file) => {
            let _ = file.write_all(line.as_bytes()).await;
        }
        Err(e) => {
            tracing::warn!("Failed to write to MCP log file: {}", e);
        }
    }
}

/// Try to obtain an OAuth access token for a server, if applicable.
async fn get_oauth_token(server_name: &str, config: &McpServerConfig) -> Result<Option<String>> {
    // Only attempt OAuth if we have explicit oauth config
    if config.oauth.is_none() {
        return Ok(None);
    }

    let url = match &config.url {
        Some(u) => u,
        None => return Ok(None),
    };

    match crate::oauth::flow::ensure_access_token(server_name, url, config.oauth.as_ref()).await {
        Ok(token) => Ok(Some(token)),
        Err(e) => {
            tracing::debug!("OAuth flow skipped or failed: {}", e);
            Ok(None)
        }
    }
}

async fn connect_stdio(
    config: &McpServerConfig,
    handler: McpClientHandler,
) -> Result<McpClientConnection> {
    let server_name = handler.server_name.clone();
    let command = config
        .command
        .clone()
        .ok_or_else(|| anyhow!("Missing command for stdio MCP server"))?;

    let mut cmd = Command::new(command);
    cmd.args(&config.args);

    // Resolve and inject shell PATH if not explicitly set in config.
    // On macOS/Linux, apps launched from Finder/dock don't inherit the user's
    // shell PATH, so commands like `npx` or `node` may not be found.
    #[cfg(unix)]
    if !config.env.contains_key("PATH") {
        if let Some(path) = resolve_shell_path() {
            cmd.env("PATH", path);
        }
    }

    for (key, value) in &config.env {
        cmd.env(key, interpolate_env_vars(value));
    }

    // Use the builder to pipe stderr so we can capture and log server output
    let (transport, stderr) = TokioChildProcess::builder(cmd)
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to create MCP child process")?;

    // Spawn a background task to read stderr and write to ~/.qbit/mcp-logs.log
    if let Some(stderr) = stderr {
        tokio::spawn(async move {
            let log_file = mcp_log_file().await;
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(mcp_server = %server_name, "[mcp:{}] {}", server_name, line);
                if let Some(ref file) = log_file {
                    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                    let log_line = format!("[{}] [{}] {}\n", timestamp, server_name, line);
                    let _ = append_to_log(file, &log_line).await;
                }
            }
        });
    }

    let service = service::serve_client(handler, transport).await?;
    Ok(service)
}

async fn connect_http(
    config: &McpServerConfig,
    handler: McpClientHandler,
) -> Result<McpClientConnection> {
    let url = config
        .url
        .as_ref()
        .ok_or_else(|| anyhow!("Missing URL for HTTP MCP server"))?;

    // Attempt OAuth if configured
    let oauth_token = get_oauth_token(handler.server_name(), config).await?;
    if oauth_token.is_some() {
        tracing::info!("Using OAuth token for HTTP transport");
    }

    let mut config_builder = StreamableHttpClientTransportConfig::with_uri(url.to_string());

    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in &config.headers {
        let resolved = interpolate_env_vars(value);
        if resolved.is_empty() {
            continue;
        }
        let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
            .with_context(|| format!("Invalid header name: {}", key))?;
        let header_value = reqwest::header::HeaderValue::from_str(&resolved)
            .with_context(|| format!("Invalid header value for {}", key))?;
        headers.insert(header_name, header_value);
    }

    // Use OAuth token if available, otherwise check static headers
    let bearer_token = if let Some(ref token) = oauth_token {
        Some(token.clone())
    } else {
        headers
            .get(reqwest::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    };

    if let Some(token) = bearer_token {
        config_builder = config_builder.auth_header(token);
        if oauth_token.is_some() {
            headers.remove(reqwest::header::AUTHORIZATION);
        }
    }

    let client = if headers.is_empty() {
        reqwest::Client::new()
    } else {
        reqwest::Client::builder()
            .default_headers(headers)
            .build()?
    };

    let transport = StreamableHttpClientTransport::with_client(client, config_builder);
    let service = service::serve_client(handler, transport).await?;
    Ok(service)
}

async fn connect_sse(
    config: &McpServerConfig,
    handler: McpClientHandler,
) -> Result<McpClientConnection> {
    let url = config
        .url
        .as_ref()
        .ok_or_else(|| anyhow!("Missing URL for SSE MCP server"))?;

    // Attempt OAuth if configured
    let oauth_token = get_oauth_token(handler.server_name(), config).await?;
    if oauth_token.is_some() {
        tracing::info!("Using OAuth token for SSE transport");
    }

    let mut headers = reqwest::header::HeaderMap::new();
    for (key, value) in &config.headers {
        let resolved = interpolate_env_vars(value);
        if resolved.is_empty() {
            continue;
        }
        let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
            .with_context(|| format!("Invalid header name: {}", key))?;
        let header_value = reqwest::header::HeaderValue::from_str(&resolved)
            .with_context(|| format!("Invalid header value for {}", key))?;
        headers.insert(header_name, header_value);
    }

    // Inject OAuth token if available
    if let Some(token) = oauth_token {
        let auth_value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
            .context("Failed to create Authorization header from OAuth token")?;
        headers.insert(reqwest::header::AUTHORIZATION, auth_value);
    }

    let client = if headers.is_empty() {
        reqwest::Client::new()
    } else {
        reqwest::Client::builder()
            .default_headers(headers)
            .build()?
    };

    let transport = crate::sse_transport::connect(url, client).await?;
    let service = handler.serve(transport).await?;
    Ok(service)
}

pub fn convert_call_tool_result(result: CallToolResult) -> McpToolResult {
    let content = result
        .content
        .into_iter()
        .filter_map(convert_content)
        .collect();

    McpToolResult {
        content,
        is_error: result.is_error.unwrap_or(false),
    }
}

fn convert_content(content: Content) -> Option<McpToolResultContent> {
    match content.raw {
        RawContent::Text(text_content) => Some(McpToolResultContent::Text(text_content.text)),
        RawContent::Image(image_content) => Some(McpToolResultContent::Image {
            data: image_content.data,
            mime_type: image_content.mime_type,
        }),
        RawContent::Resource(resource) => {
            // ResourceContents is an untagged enum; we only extract embedded text resources for now.
            let (uri, text) = match resource.resource {
                rmcp::model::ResourceContents::TextResourceContents { uri, text, .. } => {
                    (uri, Some(text))
                }
                rmcp::model::ResourceContents::BlobResourceContents { uri, .. } => (uri, None),
            };

            Some(McpToolResultContent::Resource { uri, text })
        }
        _ => None,
    }
}

pub async fn list_tools(service: &McpClientConnection, server_name: &str) -> Result<Vec<McpTool>> {
    let tools = service.list_all_tools().await?;
    Ok(tools
        .into_iter()
        .map(|tool| McpTool {
            server_name: server_name.to_string(),
            tool_name: tool.name.to_string(),
            description: tool.description.map(|d| d.to_string()),
            input_schema: serde_json::to_value(tool.input_schema)
                .unwrap_or_else(|_| serde_json::json!({})),
        })
        .collect())
}

pub async fn call_tool(
    service: &McpClientConnection,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<McpToolResult> {
    let args = arguments.as_object().cloned().unwrap_or_default();
    let params = CallToolRequestParams {
        meta: None,
        name: tool_name.to_string().into(),
        arguments: Some(args),
        task: None,
    };

    let result = service.call_tool(params).await?;
    Ok(convert_call_tool_result(result))
}
