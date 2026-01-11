//! Default implementation of ToolProvider for qbit-ai.
//!
//! This module provides a concrete implementation of the ToolProvider trait
//! that uses the local tool_definitions and tool_executors modules.

use qbit_sub_agents::ToolProvider;
use rig::completion::request::ToolDefinition;

use crate::tool_definitions::{filter_tools_by_allowed, get_all_tool_definitions};
use crate::tool_executors::{execute_web_fetch_tool, normalize_run_pty_cmd_args};

/// Default tool provider that uses qbit-ai's tool definitions and executors.
pub struct DefaultToolProvider;

impl DefaultToolProvider {
    /// Create a new DefaultToolProvider.
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultToolProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ToolProvider for DefaultToolProvider {
    fn get_all_tool_definitions(&self) -> Vec<ToolDefinition> {
        get_all_tool_definitions()
    }

    fn filter_tools_by_allowed(
        &self,
        tools: Vec<ToolDefinition>,
        allowed: &[String],
    ) -> Vec<ToolDefinition> {
        filter_tools_by_allowed(tools, allowed)
    }

    async fn execute_web_fetch_tool(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> (serde_json::Value, bool) {
        execute_web_fetch_tool(tool_name, args).await
    }

    fn normalize_run_pty_cmd_args(&self, args: serde_json::Value) -> serde_json::Value {
        normalize_run_pty_cmd_args(args)
    }
}
