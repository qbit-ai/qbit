use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};

use crate::client::{call_tool, connect_mcp_server, list_tools, McpClientConnection};
use crate::config::McpServerConfig;
use crate::tools::{parse_mcp_tool_name, sanitize_name, McpTool};

#[derive(Debug, Clone)]
pub enum ServerStatus {
    Connected { tool_count: usize },
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpToolResultContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpToolResultContent {
    Text(String),
    Image { data: String, mime_type: String },
    Resource { uri: String, text: Option<String> },
}

type ToolChangeReceiver = Arc<RwLock<mpsc::UnboundedReceiver<(String, Vec<String>)>>>;

pub struct McpManager {
    servers: Arc<RwLock<HashMap<String, McpServerConnection>>>,
    tool_index: Arc<RwLock<HashMap<String, String>>>,
    config: HashMap<String, McpServerConfig>,
    status: Arc<RwLock<HashMap<String, ServerStatus>>>,
    tool_sender: mpsc::UnboundedSender<(String, Vec<String>)>,
    tool_receiver: ToolChangeReceiver,
}

pub struct McpServerConnection {
    pub name: String,
    pub config: McpServerConfig,
    pub service: McpClientConnection,
    pub tools: Vec<McpTool>,
}

impl McpManager {
    pub fn new(config: HashMap<String, McpServerConfig>) -> Self {
        let (tool_sender, tool_receiver) = mpsc::unbounded_channel();
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
            tool_index: Arc::new(RwLock::new(HashMap::new())),
            status: Arc::new(RwLock::new(HashMap::new())),
            config,
            tool_sender,
            tool_receiver: Arc::new(RwLock::new(tool_receiver)),
        }
    }

    pub fn config(&self) -> &HashMap<String, McpServerConfig> {
        &self.config
    }

    pub async fn connect_all(&self) -> Result<()> {
        let names: Vec<String> = self
            .config
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect();

        for name in names {
            if let Err(err) = self.connect(&name).await {
                tracing::warn!("Failed to connect MCP server '{}': {}", name, err);
            }
        }

        Ok(())
    }

    pub async fn connect(&self, name: &str) -> Result<()> {
        let config = self
            .config
            .get(name)
            .ok_or_else(|| anyhow!("Unknown MCP server '{}'", name))?
            .clone();

        let service = connect_mcp_server(name, &config, self.tool_sender.clone()).await?;
        let tools = list_tools(&service, name).await?;
        let mut servers = self.servers.write().await;

        let connection = McpServerConnection {
            name: name.to_string(),
            config: config.clone(),
            service,
            tools: tools.clone(),
        };
        servers.insert(name.to_string(), connection);

        let mut tool_index = self.tool_index.write().await;
        for tool in tools.iter() {
            // Index by both simple name and fully qualified name
            let simple_name = tool.tool_name.clone();
            let qualified_name = format!("mcp__{}__{}", name, simple_name);
            tool_index.insert(simple_name, name.to_string());
            tool_index.insert(qualified_name, name.to_string());
        }

        let mut status = self.status.write().await;
        status.insert(
            name.to_string(),
            ServerStatus::Connected {
                tool_count: tools.len(),
            },
        );

        Ok(())
    }

    pub async fn disconnect(&self, name: &str) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(connection) = servers.remove(name) {
            let _ = connection.service.cancel().await;
        }
        let mut status = self.status.write().await;
        status.insert(name.to_string(), ServerStatus::Disconnected);
        Ok(())
    }

    /// Disconnect from all connected servers.
    /// This should be called during app shutdown to ensure stdio processes are killed.
    pub async fn disconnect_all(&self) {
        let names: Vec<String> = {
            let servers = self.servers.read().await;
            servers.keys().cloned().collect()
        };

        for name in names {
            if let Err(e) = self.disconnect(&name).await {
                tracing::warn!(
                    "[mcp] Failed to disconnect from '{}' during shutdown: {}",
                    name,
                    e
                );
            } else {
                tracing::debug!("[mcp] Disconnected from '{}' during shutdown", name);
            }
        }
    }

    /// Shutdown the MCP manager, disconnecting from all servers.
    /// Alias for disconnect_all for API clarity.
    pub async fn shutdown(&self) {
        self.disconnect_all().await;
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        self.refresh_tools_from_notifications().await;

        let servers = self.servers.read().await;
        let mut tools = Vec::new();
        for connection in servers.values() {
            tools.extend(connection.tools.clone());
        }
        Ok(tools)
    }

    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult> {
        let (server_sanitized, tool_sanitized) = parse_mcp_tool_name(tool_name)?;
        let servers = self.servers.read().await;

        // Look up the server by sanitized name (hyphens replaced with underscores)
        let connection = servers
            .values()
            .find(|c| sanitize_name(&c.name) == server_sanitized)
            .ok_or_else(|| anyhow!("MCP server '{}' not connected", server_sanitized))?;

        // Find the original tool name by matching the sanitized version
        let original_tool_name = connection
            .tools
            .iter()
            .find(|t| sanitize_name(&t.tool_name) == tool_sanitized)
            .map(|t| t.tool_name.as_str())
            .unwrap_or(&tool_sanitized);

        call_tool(&connection.service, original_tool_name, arguments).await
    }

    pub async fn server_status(&self, name: &str) -> Option<ServerStatus> {
        self.status.read().await.get(name).cloned()
    }

    async fn refresh_tools_from_notifications(&self) {
        let mut receiver = self.tool_receiver.write().await;
        while let Ok((server, tool_names)) = receiver.try_recv() {
            let mut servers = self.servers.write().await;
            if let Some(connection) = servers.get_mut(&server) {
                let updated_tools = list_tools(&connection.service, &server).await;
                if let Ok(tools) = updated_tools {
                    connection.tools = tools.clone();

                    let mut tool_index = self.tool_index.write().await;
                    for tool in tools {
                        // Index by both simple name and fully qualified name
                        let simple_name = tool.tool_name.clone();
                        let qualified_name = format!("mcp__{}__{}", server, simple_name);
                        tool_index.insert(simple_name, server.clone());
                        tool_index.insert(qualified_name, server.clone());
                    }

                    let mut status = self.status.write().await;
                    status.insert(
                        server.clone(),
                        ServerStatus::Connected {
                            tool_count: tool_names.len(),
                        },
                    );
                }
            }
        }
    }
}
