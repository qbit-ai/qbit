//! Tauri commands for MCP server management.
//!
//! These commands enable the frontend to:
//! - List configured MCP servers and their connection status
//! - View available tools from connected servers
//! - Trust project-specific MCP configurations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::State;

use crate::state::AppState;

/// Information about a configured MCP server for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerInfo {
    /// Server name (from config key)
    pub name: String,
    /// Transport type (stdio, http)
    pub transport: String,
    /// Whether the server is enabled in config
    pub enabled: bool,
    /// Connection status
    pub status: McpServerStatus,
    /// Number of tools available (if connected)
    pub tool_count: Option<usize>,
    /// Error message (if status is Error)
    pub error: Option<String>,
    /// Source: "user" for ~/.qbit/mcp.json, "project" for <project>/.qbit/mcp.json
    pub source: String,
}

/// Server connection status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpServerStatus {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

/// Information about an MCP tool for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolInfo {
    /// Full tool name (mcp__{server}__{tool})
    pub name: String,
    /// Server this tool belongs to
    pub server_name: String,
    /// Original tool name from the server
    pub tool_name: String,
    /// Tool description
    pub description: Option<String>,
}

/// List all configured MCP servers with their status.
///
/// Returns servers from both user-global (~/.qbit/mcp.json) and
/// project-specific (<project>/.qbit/mcp.json) configurations.
#[tauri::command]
pub async fn mcp_list_servers(
    workspace_path: Option<String>,
    _state: State<'_, AppState>,
) -> Result<Vec<McpServerInfo>, String> {
    use qbit_mcp::{load_mcp_config, McpTransportType};

    // Get workspace path (from parameter or current dir as fallback)
    let workspace = match workspace_path {
        Some(p) => PathBuf::from(p),
        None => {
            // Fall back to current directory
            std::env::current_dir()
                .map_err(|e| format!("Failed to get current directory: {}", e))?
        }
    };

    // Load merged config
    let config = load_mcp_config(&workspace).map_err(|e| e.to_string())?;

    // Check which servers are from user vs project config
    let user_config = dirs::home_dir()
        .map(|h| h.join(".qbit/mcp.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<qbit_mcp::McpConfigFile>(&s).ok())
        .map(|c| {
            c.mcp_servers
                .keys()
                .cloned()
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    let mut servers = Vec::new();
    for (name, server_config) in config.mcp_servers {
        let transport = match server_config.transport {
            McpTransportType::Stdio => "stdio",
            McpTransportType::Http => "http",
            McpTransportType::Sse => "sse",
        };

        let source = if user_config.contains(&name) {
            "user"
        } else {
            "project"
        };

        servers.push(McpServerInfo {
            name,
            transport: transport.to_string(),
            enabled: server_config.enabled,
            // TODO: Track actual connection status per-session
            // For now, report as disconnected until connected via session
            status: McpServerStatus::Disconnected,
            tool_count: None,
            error: None,
            source: source.to_string(),
        });
    }

    Ok(servers)
}

/// List all tools from connected MCP servers for a session.
///
/// This retrieves tools from the McpManager associated with the session's AgentBridge.
#[tauri::command]
pub async fn mcp_list_tools(
    session_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<McpToolInfo>, String> {
    // Get the session's bridge
    let bridge = state
        .ai_state
        .get_session_bridge(&session_id)
        .await
        .ok_or_else(|| format!("Session '{}' not found", session_id))?;

    // Get MCP tool definitions from the bridge
    let tool_defs = bridge.mcp_tool_definitions().await;

    let mut tools = Vec::new();
    for def in tool_defs {
        // Parse the tool name: mcp__{server}__{tool}
        if let Ok((server_name, tool_name)) = qbit_mcp::parse_mcp_tool_name(&def.name) {
            tools.push(McpToolInfo {
                name: def.name.clone(),
                server_name,
                tool_name,
                description: Some(def.description.clone()),
            });
        }
    }

    Ok(tools)
}

/// Check if a project's MCP configuration is trusted.
#[tauri::command]
pub async fn mcp_is_project_trusted(project_path: String) -> Result<bool, String> {
    let path = PathBuf::from(project_path);
    Ok(qbit_mcp::is_project_config_trusted(&path))
}

/// Mark a project's MCP configuration as trusted.
///
/// This should be called after the user explicitly approves a project's
/// MCP configuration in the UI.
#[tauri::command]
pub async fn mcp_trust_project_config(project_path: String) -> Result<(), String> {
    let path = PathBuf::from(project_path);
    qbit_mcp::trust_project_config(&path).map_err(|e| e.to_string())
}

/// Get MCP configuration for a workspace.
///
/// Returns the merged configuration from user-global and project-specific sources.
#[tauri::command]
pub async fn mcp_get_config(
    workspace_path: String,
) -> Result<HashMap<String, serde_json::Value>, String> {
    use qbit_mcp::load_mcp_config;

    let workspace = PathBuf::from(workspace_path);
    let config = load_mcp_config(&workspace).map_err(|e| e.to_string())?;

    // Convert to JSON-serializable format
    let servers: HashMap<String, serde_json::Value> = config
        .mcp_servers
        .into_iter()
        .map(|(name, cfg)| {
            (
                name,
                serde_json::to_value(cfg).unwrap_or(serde_json::Value::Null),
            )
        })
        .collect();

    Ok(servers)
}

/// Check if MCP config exists for a workspace.
#[tauri::command]
pub async fn mcp_has_project_config(workspace_path: String) -> Result<bool, String> {
    let path = PathBuf::from(workspace_path).join(".qbit/mcp.json");
    Ok(path.exists())
}

/// Connect to an MCP server for a session.
///
/// The server must be configured in the session's workspace config.
#[tauri::command]
pub async fn mcp_connect(
    session_id: String,
    server_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Get the session's MCP manager
    let manager = state
        .ai_state
        .get_session_mcp_manager(&session_id)
        .await
        .ok_or_else(|| {
            format!(
                "No MCP manager for session '{}'. MCP may not be configured.",
                session_id
            )
        })?;

    // Connect to the server
    manager
        .connect(&server_name)
        .await
        .map_err(|e| format!("Failed to connect to MCP server '{}': {}", server_name, e))?;

    // Update bridge tool definitions with new tools from this server
    if let Some(bridge) = state.ai_state.get_session_bridge(&session_id).await {
        let tools = manager.list_tools().await.map_err(|e| e.to_string())?;
        let tool_definitions: Vec<rig::completion::ToolDefinition> =
            tools.iter().map(|tool| tool.to_tool_definition()).collect();
        bridge.set_mcp_tools(tool_definitions).await;
    }

    Ok(())
}

/// Disconnect from an MCP server for a session.
#[tauri::command]
pub async fn mcp_disconnect(
    session_id: String,
    server_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Get the session's MCP manager
    let manager = state
        .ai_state
        .get_session_mcp_manager(&session_id)
        .await
        .ok_or_else(|| {
            format!(
                "No MCP manager for session '{}'. MCP may not be configured.",
                session_id
            )
        })?;

    // Disconnect from the server
    manager.disconnect(&server_name).await.map_err(|e| {
        format!(
            "Failed to disconnect from MCP server '{}': {}",
            server_name, e
        )
    })?;

    // Update bridge tool definitions to remove tools from this server
    if let Some(bridge) = state.ai_state.get_session_bridge(&session_id).await {
        let tools = manager.list_tools().await.map_err(|e| e.to_string())?;
        let tool_definitions: Vec<rig::completion::ToolDefinition> =
            tools.iter().map(|tool| tool.to_tool_definition()).collect();
        bridge.set_mcp_tools(tool_definitions).await;
    }

    Ok(())
}
