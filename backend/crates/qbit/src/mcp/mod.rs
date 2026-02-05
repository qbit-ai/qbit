//! MCP (Model Context Protocol) Tauri commands.
//!
//! This module provides commands for managing MCP server connections:
//! - Listing configured servers and their status
//! - Manual connect/disconnect control
//! - Listing available tools
//! - Config management (add/remove servers)
//! - Project config trust

mod commands;

pub use commands::{
    mcp_connect, mcp_disconnect, mcp_get_config, mcp_has_project_config, mcp_is_project_trusted,
    mcp_list_servers, mcp_list_tools, mcp_trust_project_config,
};
