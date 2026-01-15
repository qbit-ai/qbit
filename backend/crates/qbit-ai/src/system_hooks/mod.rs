//! System Hooks - Extensible hook system for agent behavior customization.
//!
//! This module provides a mechanism to inject contextual messages during agent
//! execution. Hooks can trigger on:
//!
//! - **User messages**: Keyword or regex matches in user input
//! - **Agent responses**: Keyword or regex matches in agent output
//! - **Pre-tool execution**: Before a tool runs (can block or inject messages)
//! - **Post-tool execution**: After a tool completes (can inject messages)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        System Hooks                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │  Message Hooks                    Tool Hooks                     │
//! │  ─────────────                    ──────────                     │
//! │  • UserMessage                    • PreToolExecution             │
//! │  • AgentResponse                  • PostToolExecution            │
//! │                                                                  │
//! │  Matchers:                        Matchers:                      │
//! │  • Keyword                        • ToolName                     │
//! │  • Regex                          • ToolCategory                 │
//! │  • Custom predicate               • Custom predicate             │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use qbit_ai::system_hooks::{HookRegistry, format_system_hooks};
//!
//! // Create registry with built-in hooks
//! let registry = HookRegistry::new();
//!
//! // Run message hooks
//! let ctx = MessageHookContext::user_input("help me", "session-123");
//! let messages = registry.run_message_hooks(&ctx);
//!
//! // Run pre-tool hooks
//! let ctx = PreToolContext::new("write_file", &args, "session-123");
//! match registry.run_pre_tool_hooks(&ctx) {
//!     PreToolResult::Block(reason) => { /* tool blocked */ }
//!     PreToolResult::AllowWithMessage(msg) => { /* inject msg after tool */ }
//!     PreToolResult::Allow => { /* proceed normally */ }
//! }
//!
//! // Run post-tool hooks
//! let ctx = PostToolContext::new("update_plan", &args, &result, true, 100, "session-123");
//! let messages = registry.run_post_tool_hooks(&ctx);
//!
//! // Format messages for injection
//! let formatted = format_system_hooks(&messages);
//! ```

mod builtins;
mod context;
mod format;
mod hooks;
mod matcher;
mod registry;

// Re-export public API
pub use context::{MessageHookContext, MessageType, PostToolContext, PreToolContext};
pub use format::format_system_hooks;
pub use hooks::{PreToolResult, ToolHookPhase};
pub use matcher::{MessageMatcher, ToolMatcher};
pub use registry::{HookRegistry, HookRegistryStats};

// Re-export hook types for advanced usage (creating custom hooks)
pub use hooks::{MessageHook, ToolHook};
