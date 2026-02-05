use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Root structure of mcp.json files.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigFile {
    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

/// Server transport type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransportType {
    #[default]
    Stdio,
    Http,
    Sse,
}

/// MCP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Transport type (default: stdio)
    #[serde(default)]
    pub transport: McpTransportType,

    /// Command for stdio transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,

    /// Arguments for the command
    #[serde(default)]
    pub args: Vec<String>,

    /// Environment variables (supports $VAR and ${VAR} syntax)
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// URL for HTTP/SSE transport
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// HTTP headers (supports $VAR and ${VAR} syntax)
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Whether this server is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Timeout in seconds for server startup (default: 30)
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_file_deserialize_empty() {
        let json = "{}";
        let config: McpConfigFile = serde_json::from_str(json).unwrap();
        assert!(config.mcp_servers.is_empty());
    }

    #[test]
    fn test_config_file_deserialize_basic() {
        let json = r#"{
            "mcpServers": {
                "test": {
                    "command": "echo"
                }
            }
        }"#;
        let config: McpConfigFile = serde_json::from_str(json).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        assert!(config.mcp_servers.contains_key("test"));
    }

    #[test]
    fn test_server_config_defaults() {
        let json = r#"{ "command": "test" }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();

        // Check defaults
        assert!(matches!(config.transport, McpTransportType::Stdio));
        assert!(config.enabled);
        assert_eq!(config.timeout, 30);
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
        assert!(config.headers.is_empty());
        assert!(config.url.is_none());
    }

    #[test]
    fn test_server_config_stdio() {
        let json = r#"{
            "transport": "stdio",
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path"],
            "env": {
                "DEBUG": "true"
            }
        }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();

        assert!(matches!(config.transport, McpTransportType::Stdio));
        assert_eq!(config.command.as_deref(), Some("npx"));
        assert_eq!(
            config.args,
            vec!["-y", "@modelcontextprotocol/server-filesystem", "/path"]
        );
        assert_eq!(config.env.get("DEBUG"), Some(&"true".to_string()));
    }

    #[test]
    fn test_server_config_http() {
        let json = r#"{
            "transport": "http",
            "url": "https://api.example.com/mcp",
            "headers": {
                "Authorization": "Bearer ${TOKEN}"
            }
        }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();

        assert!(matches!(config.transport, McpTransportType::Http));
        assert_eq!(config.url.as_deref(), Some("https://api.example.com/mcp"));
        assert_eq!(
            config.headers.get("Authorization"),
            Some(&"Bearer ${TOKEN}".to_string())
        );
    }

    #[test]
    fn test_server_config_disabled() {
        let json = r#"{
            "command": "test",
            "enabled": false
        }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();

        assert!(!config.enabled);
    }

    #[test]
    fn test_server_config_custom_timeout() {
        let json = r#"{
            "command": "test",
            "timeout": 60
        }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.timeout, 60);
    }

    #[test]
    fn test_transport_type_serde() {
        assert_eq!(
            serde_json::to_string(&McpTransportType::Stdio).unwrap(),
            "\"stdio\""
        );
        assert_eq!(
            serde_json::to_string(&McpTransportType::Http).unwrap(),
            "\"http\""
        );
        assert_eq!(
            serde_json::to_string(&McpTransportType::Sse).unwrap(),
            "\"sse\""
        );
    }

    #[test]
    fn test_config_roundtrip() {
        let mut servers = HashMap::new();
        servers.insert(
            "test".to_string(),
            McpServerConfig {
                transport: McpTransportType::Http,
                command: None,
                args: vec![],
                env: HashMap::new(),
                url: Some("https://example.com".to_string()),
                headers: HashMap::new(),
                enabled: true,
                timeout: 30,
            },
        );

        let config = McpConfigFile {
            mcp_servers: servers,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: McpConfigFile = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.mcp_servers.len(), 1);
        assert_eq!(
            parsed.mcp_servers["test"].url.as_deref(),
            Some("https://example.com")
        );
    }
}
