//! MCP (Model Context Protocol) client integration for Qbit.
//!
//! This crate provides:
//! - MCP config loading and trust handling
//! - MCP client/transport management via rmcp
//! - Tool conversion to Qbit's tool definition format

pub mod client;
pub mod config;
pub mod loader;
pub mod manager;
pub mod oauth;
pub mod sse_transport;
pub mod tools;

pub use client::{McpClientConnection, McpClientHandler};
pub use config::{McpConfigFile, McpServerConfig, McpTransportType};
pub use loader::{
    interpolate_env_vars, is_project_config_trusted, load_mcp_config, trust_project_config,
    TrustedMcpConfigs,
};
pub use manager::{McpManager, McpToolResult, McpToolResultContent, ServerStatus};
pub use tools::{convert_mcp_result_to_tool_result, parse_mcp_tool_name, sanitize_name, McpTool};
