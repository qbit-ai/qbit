use anyhow::{anyhow, Result};
use rig::completion::ToolDefinition;
use serde_json::Value;

use crate::manager::{McpToolResult, McpToolResultContent};

#[derive(Debug, Clone)]
pub struct McpTool {
    pub server_name: String,
    pub tool_name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

impl McpTool {
    /// Convert to rig ToolDefinition for LLM consumption.
    pub fn to_tool_definition(&self) -> ToolDefinition {
        let full_name = format!(
            "mcp__{}__{}",
            sanitize_name(&self.server_name),
            sanitize_name(&self.tool_name)
        );

        ToolDefinition {
            name: full_name,
            description: self
                .description
                .clone()
                .unwrap_or_else(|| format!("MCP tool from {}", self.server_name)),
            parameters: self.input_schema.clone(),
        }
    }
}

pub fn parse_mcp_tool_name(name: &str) -> Result<(String, String)> {
    let mut parts = name.splitn(3, "__");
    let prefix = parts.next().unwrap_or_default();
    if prefix != "mcp" {
        return Err(anyhow!("Invalid MCP tool name: {}", name));
    }
    let server = parts
        .next()
        .ok_or_else(|| anyhow!("Missing MCP server name"))?;
    let tool = parts
        .next()
        .ok_or_else(|| anyhow!("Missing MCP tool name"))?;
    Ok((server.to_string(), tool.to_string()))
}

pub fn convert_mcp_result_to_tool_result(result: McpToolResult) -> (Value, bool) {
    let mut contents = Vec::new();
    for content in result.content {
        match content {
            McpToolResultContent::Text(text) => contents.push(Value::String(text)),
            McpToolResultContent::Image { data, mime_type } => contents.push(serde_json::json!({
                "type": "image",
                "data": data,
                "mime_type": mime_type,
            })),
            McpToolResultContent::Resource { uri, text } => contents.push(serde_json::json!({
                "type": "resource",
                "uri": uri,
                "text": text,
            })),
        }
    }

    let value = serde_json::json!({
        "content": contents,
        "is_error": result.is_error,
    });

    (value, !result.is_error)
}

fn sanitize_name(value: &str) -> String {
    value.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mcp_tool_name_valid() {
        let (server, tool) = parse_mcp_tool_name("mcp__my_server__my_tool").unwrap();
        assert_eq!(server, "my_server");
        assert_eq!(tool, "my_tool");
    }

    #[test]
    fn test_parse_mcp_tool_name_with_underscores() {
        let (server, tool) =
            parse_mcp_tool_name("mcp__server_name__tool_with_underscores").unwrap();
        assert_eq!(server, "server_name");
        assert_eq!(tool, "tool_with_underscores");
    }

    #[test]
    fn test_parse_mcp_tool_name_tool_with_double_underscore() {
        // Tool name can contain __ since we use splitn(3, "__")
        let (server, tool) = parse_mcp_tool_name("mcp__server__tool__with__more").unwrap();
        assert_eq!(server, "server");
        assert_eq!(tool, "tool__with__more");
    }

    #[test]
    fn test_parse_mcp_tool_name_invalid_prefix() {
        let result = parse_mcp_tool_name("notmcp__server__tool");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mcp_tool_name_missing_server() {
        let result = parse_mcp_tool_name("mcp__");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_mcp_tool_name_missing_tool() {
        let result = parse_mcp_tool_name("mcp__server");
        assert!(result.is_err());
    }

    #[test]
    fn test_mcp_tool_to_definition() {
        let tool = McpTool {
            server_name: "test-server".to_string(),
            tool_name: "my-tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "arg1": { "type": "string" }
                }
            }),
        };

        let def = tool.to_tool_definition();

        // Server and tool names should have hyphens replaced with underscores
        assert_eq!(def.name, "mcp__test_server__my_tool");
        assert_eq!(def.description, "A test tool");
        assert_eq!(def.parameters["type"], "object");
    }

    #[test]
    fn test_mcp_tool_to_definition_no_description() {
        let tool = McpTool {
            server_name: "server".to_string(),
            tool_name: "tool".to_string(),
            description: None,
            input_schema: serde_json::json!({}),
        };

        let def = tool.to_tool_definition();
        assert_eq!(def.description, "MCP tool from server");
    }

    #[test]
    fn test_convert_mcp_result_text() {
        let result = McpToolResult {
            content: vec![McpToolResultContent::Text("Hello world".to_string())],
            is_error: false,
        };

        let (value, success) = convert_mcp_result_to_tool_result(result);

        assert!(success);
        assert_eq!(value["is_error"], false);
        assert_eq!(value["content"][0], "Hello world");
    }

    #[test]
    fn test_convert_mcp_result_error() {
        let result = McpToolResult {
            content: vec![McpToolResultContent::Text("Error message".to_string())],
            is_error: true,
        };

        let (value, success) = convert_mcp_result_to_tool_result(result);

        assert!(!success);
        assert_eq!(value["is_error"], true);
    }

    #[test]
    fn test_convert_mcp_result_image() {
        let result = McpToolResult {
            content: vec![McpToolResultContent::Image {
                data: "base64data".to_string(),
                mime_type: "image/png".to_string(),
            }],
            is_error: false,
        };

        let (value, _) = convert_mcp_result_to_tool_result(result);

        assert_eq!(value["content"][0]["type"], "image");
        assert_eq!(value["content"][0]["data"], "base64data");
        assert_eq!(value["content"][0]["mime_type"], "image/png");
    }

    #[test]
    fn test_convert_mcp_result_resource() {
        let result = McpToolResult {
            content: vec![McpToolResultContent::Resource {
                uri: "file:///path/to/file".to_string(),
                text: Some("file contents".to_string()),
            }],
            is_error: false,
        };

        let (value, _) = convert_mcp_result_to_tool_result(result);

        assert_eq!(value["content"][0]["type"], "resource");
        assert_eq!(value["content"][0]["uri"], "file:///path/to/file");
        assert_eq!(value["content"][0]["text"], "file contents");
    }

    #[test]
    fn test_convert_mcp_result_multiple_contents() {
        let result = McpToolResult {
            content: vec![
                McpToolResultContent::Text("First".to_string()),
                McpToolResultContent::Text("Second".to_string()),
            ],
            is_error: false,
        };

        let (value, _) = convert_mcp_result_to_tool_result(result);

        assert_eq!(value["content"].as_array().unwrap().len(), 2);
        assert_eq!(value["content"][0], "First");
        assert_eq!(value["content"][1], "Second");
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("my-server"), "my_server");
        assert_eq!(sanitize_name("already_ok"), "already_ok");
        assert_eq!(
            sanitize_name("multiple-dashes-here"),
            "multiple_dashes_here"
        );
    }
}
