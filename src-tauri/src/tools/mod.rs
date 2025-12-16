//! Local tool registry - drop-in replacement for vtcode_core::tools::ToolRegistry
//!
//! This module provides a complete implementation of tool execution for the AI agent system.
//! It is designed as a drop-in replacement for vtcode_core's ToolRegistry, maintaining
//! exact interface compatibility for seamless migration.
//!
//! ## Interface Contract
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
//! ## Success/Failure Contract
//!
//! All tools follow this return format:
//! - Success: JSON without "error" field, shell commands include `"exit_code": 0`
//! - Failure: JSON with "error" field, shell commands include non-zero exit_code
//!
//! This contract is enforced by agentic_loop.rs:429-436.

mod definitions;
mod directory_ops;
mod error;
mod file_ops;
mod registry;
mod shell;
mod traits;

pub use definitions::{build_function_declarations, FunctionDeclaration};
pub use error::ToolError;
pub use registry::ToolRegistry;
pub use traits::Tool;

// Re-export for compatibility with vtcode_core::tools::registry::build_function_declarations
pub mod registry_exports {
    pub use super::definitions::build_function_declarations;
}
