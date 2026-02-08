//! Tauri commands for MCP server management.
//!
//! These commands enable the frontend to:
//! - List configured MCP servers and their connection status
//! - View available tools from connected servers
//! - Connect/disconnect individual servers
//! - Trust project-specific MCP configurations
//!
//! The MCP manager is global (shared across all sessions) and initialized
//! in the background during app startup.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
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
/// Live connection status is reported from the global MCP manager.
#[tauri::command]
pub async fn mcp_list_servers(
    workspace_path: Option<String>,
    state: State<'_, AppState>,
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

    // Get live status from the global MCP manager (if initialized)
    let manager_guard = state.mcp_manager.read().await;
    let manager = manager_guard.as_ref();

    let mut servers = Vec::new();
    for (name, server_config) in config.mcp_servers {
        let transport = match server_config.transport() {
            McpTransportType::Stdio => "stdio",
            McpTransportType::Http => "http",
            McpTransportType::Sse => "sse",
        };

        let source = if user_config.contains(&name) {
            "user"
        } else {
            "project"
        };

        // Get live connection status from the global manager
        let (status, tool_count, error) = if let Some(mgr) = manager {
            match mgr.server_status(&name).await {
                Some(qbit_mcp::ServerStatus::Connected { tool_count }) => {
                    (McpServerStatus::Connected, Some(tool_count), None)
                }
                Some(qbit_mcp::ServerStatus::Error(msg)) => {
                    (McpServerStatus::Error, None, Some(msg))
                }
                Some(qbit_mcp::ServerStatus::Disconnected) | None => {
                    (McpServerStatus::Disconnected, None, None)
                }
            }
        } else {
            // Manager not yet initialized
            (McpServerStatus::Disconnected, None, None)
        };

        servers.push(McpServerInfo {
            name,
            transport: transport.to_string(),
            enabled: server_config.enabled,
            status,
            tool_count,
            error,
            source: source.to_string(),
        });
    }

    Ok(servers)
}

/// List all tools from connected MCP servers.
///
/// This retrieves tools from the global MCP manager.
#[tauri::command]
pub async fn mcp_list_tools(state: State<'_, AppState>) -> Result<Vec<McpToolInfo>, String> {
    let manager_guard = state.mcp_manager.read().await;
    let manager = manager_guard
        .as_ref()
        .ok_or_else(|| "MCP manager not initialized yet".to_string())?;

    let tools = manager.list_tools().await.map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for tool in tools {
        let full_name = format!(
            "mcp__{}__{}",
            qbit_mcp::sanitize_name(&tool.server_name),
            qbit_mcp::sanitize_name(&tool.tool_name)
        );

        if let Ok((server_name, tool_name)) = qbit_mcp::parse_mcp_tool_name(&full_name) {
            result.push(McpToolInfo {
                name: full_name,
                server_name,
                tool_name,
                description: tool.description.clone(),
            });
        }
    }

    Ok(result)
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

/// Connect to an MCP server.
///
/// The server must be configured in the workspace config.
/// After connecting, all active agent sessions have their MCP tools refreshed.
#[tauri::command]
pub async fn mcp_connect(server_name: String, state: State<'_, AppState>) -> Result<(), String> {
    // Get the global MCP manager
    let manager_guard = state.mcp_manager.read().await;
    let manager = manager_guard.as_ref().ok_or_else(|| {
        "MCP manager not initialized yet. Please wait for background initialization to complete."
            .to_string()
    })?;
    let manager = Arc::clone(manager);
    drop(manager_guard);

    // Connect to the server
    manager
        .connect(&server_name)
        .await
        .map_err(|e| format!("Failed to connect to MCP server '{}': {}", server_name, e))?;

    // Refresh MCP tools on all active bridges
    refresh_all_bridge_mcp_tools(&state).await;

    Ok(())
}

/// Disconnect from an MCP server.
///
/// After disconnecting, all active agent sessions have their MCP tools refreshed.
#[tauri::command]
pub async fn mcp_disconnect(server_name: String, state: State<'_, AppState>) -> Result<(), String> {
    // Get the global MCP manager
    let manager_guard = state.mcp_manager.read().await;
    let manager = manager_guard.as_ref().ok_or_else(|| {
        "MCP manager not initialized yet. Please wait for background initialization to complete."
            .to_string()
    })?;
    let manager = Arc::clone(manager);
    drop(manager_guard);

    // Disconnect from the server
    manager.disconnect(&server_name).await.map_err(|e| {
        format!(
            "Failed to disconnect from MCP server '{}': {}",
            server_name, e
        )
    })?;

    // Refresh MCP tools on all active bridges
    refresh_all_bridge_mcp_tools(&state).await;

    Ok(())
}

/// Refresh MCP tool definitions on all active agent bridges.
///
/// Called after connect/disconnect to keep all sessions in sync with the global manager.
async fn refresh_all_bridge_mcp_tools(state: &AppState) {
    let bridges = state.ai_state.bridges.read().await;
    for (session_id, bridge) in bridges.iter() {
        crate::ai::commands::setup_bridge_mcp_tools(bridge, state).await;
        tracing::debug!("[mcp] Refreshed MCP tools for session {}", session_id);
    }
}
