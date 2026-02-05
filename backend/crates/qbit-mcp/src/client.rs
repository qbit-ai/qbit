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

async fn connect_stdio(
    config: &McpServerConfig,
    handler: McpClientHandler,
) -> Result<McpClientConnection> {
    let command = config
        .command
        .clone()
        .ok_or_else(|| anyhow!("Missing command for stdio MCP server"))?;

    let mut cmd = Command::new(command);
    cmd.args(&config.args);
    for (key, value) in &config.env {
        cmd.env(key, interpolate_env_vars(value));
    }

    let transport = TokioChildProcess::new(cmd).context("Failed to create MCP child process")?;
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

    if let Some(auth) = headers
        .get(reqwest::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            config_builder = config_builder.auth_header(token.to_string());
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
