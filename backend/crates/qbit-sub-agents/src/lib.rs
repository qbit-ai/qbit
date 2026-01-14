//! Sub-agent system for Qbit.
//!
//! This crate provides the sub-agent system infrastructure including:
//! - Sub-agent definitions with custom system prompts and tool restrictions
//! - Sub-agent registry for managing available agents
//! - Context management for passing state between agents
//! - Sub-agent execution with tool support
//! - Default sub-agent definitions for common tasks
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: qbit-core, qbit-udiff, qbit-web, rig-core, vtcode-core
//! - Used by: qbit-ai (for sub-agent orchestration)
//!
//! # Core Components
//!
//! - **SubAgentDefinition**: Defines a specialized sub-agent
//! - **SubAgentRegistry**: Registry of available sub-agents
//! - **SubAgentContext**: Context passed between agents
//! - **SubAgentResult**: Result returned by sub-agent execution
//! - **execute_sub_agent**: Main execution function
//! - **ToolProvider**: Trait for tool definition/execution injection
//!
//! # Example
//!
//! ```ignore
//! use qbit_sub_agents::{SubAgentDefinition, SubAgentRegistry, create_default_sub_agents};
//!
//! // Create a registry with default sub-agents
//! let mut registry = SubAgentRegistry::new();
//! registry.register_multiple(create_default_sub_agents());
//!
//! // Get a specific sub-agent
//! if let Some(analyzer) = registry.get("analyzer") {
//!     println!("Found: {}", analyzer.name);
//! }
//! ```

pub mod defaults;
pub mod definition;
pub mod executor;
pub mod schemas;
pub mod transcript;

// Re-export main types from definition module
pub use definition::{
    SubAgentContext, SubAgentDefinition, SubAgentRegistry, SubAgentResult, MAX_AGENT_DEPTH,
};

// Re-export default sub-agents function
pub use defaults::create_default_sub_agents;

// Re-export executor types
pub use executor::{execute_sub_agent, SubAgentExecutorContext, ToolProvider};
