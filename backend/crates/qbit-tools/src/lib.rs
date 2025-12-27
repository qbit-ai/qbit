//! Local tool registry - drop-in replacement for vtcode_core::tools::ToolRegistry
//!
//! This crate provides a complete implementation of tool execution for the AI agent system.
//! It is designed as a drop-in replacement for vtcode_core's ToolRegistry, maintaining
//! exact interface compatibility for seamless migration.
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: qbit-core (for foundation types)
//! - Used by: qbit (main application)
//!
//! # Interface Contract
//!
//! The following interface MUST be preserved for compatibility:
//!
//! ```rust,ignore
//! // Creation (llm_client.rs:84)
//! ToolRegistry::new(workspace.to_path_buf()).await
//!
//! // Tool execution (agentic_loop.rs:425)
//! registry.execute_tool(tool_name, tool_args.clone()).await
//! // Returns: Result<serde_json::Value, Error>
//!
//! // Tool listing
//! registry.available_tools()
//! // Returns: Vec<String>
//! ```
//!
//! # Success/Failure Contract
//!
//! All tools follow this return format:
//! - Success: JSON without "error" field, shell commands include `"exit_code": 0`
//! - Failure: JSON with "error" field, shell commands include non-zero exit_code
//!
//! This contract is enforced by agentic_loop.rs:429-436.
//!
//! # Usage
//!
//! ```rust,ignore
//! use qbit_tools::{ToolRegistry, FunctionDeclaration, build_function_declarations};
//!
//! // Create registry for workspace
//! let registry = ToolRegistry::new(workspace_path).await;
//!
//! // Execute tool
//! let result = registry.execute_tool("read_file", args).await?;
//!
//! // Get available tools
//! let tools = registry.available_tools();
//! ```

mod definitions;
mod error;
mod registry;

pub use definitions::{build_function_declarations, FunctionDeclaration};
pub use error::ToolError;
pub use registry::ToolRegistry;

// Re-export Tool trait from qbit-core for backward compatibility
pub use qbit_core::Tool;

// Re-export for compatibility with vtcode_core::tools::registry::build_function_declarations
pub mod registry_exports {
    pub use super::definitions::build_function_declarations;
}
