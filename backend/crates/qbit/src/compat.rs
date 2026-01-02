//! Compatibility layer for vtcode-core migration.
//!
//! This module provides unified imports that abstract over the underlying
//! implementations. Local implementations from qbit_tools and qbit_core are
//! now the default and only option.
//!
//! ## Usage
//!
//! Import from the compat layer for consistent access:
//!
//! ```rust,ignore
//! use crate::compat::tools::ToolRegistry;
//! use crate::compat::session::SessionArchive;
//! ```

// =============================================================================
// Tool Registry
// =============================================================================

/// Tool registry compatibility module.
///
/// Provides `ToolRegistry` and related types from qbit_tools.
pub mod tools {
    pub use qbit_tools::{build_function_declarations, FunctionDeclaration, Tool, ToolRegistry};
}

// =============================================================================
// Session Archive
// =============================================================================

/// Session archive compatibility module.
///
/// Provides `SessionArchive`, `SessionMessage`, `MessageRole`, and related
/// types from qbit_core::session.
pub mod session {
    pub use qbit_core::session::{
        find_session_by_identifier, get_sessions_dir, list_recent_sessions, MessageContent,
        MessageRole, SessionArchive, SessionArchiveMetadata, SessionListing, SessionMessage,
        SessionSnapshot,
    };
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that tool module exports are available.
    #[test]
    fn test_tools_exports_available() {
        fn _assert_tool_registry_exists<T: Sized>(_: &T) {}
        let _: fn() -> Vec<tools::FunctionDeclaration> = tools::build_function_declarations;
        fn _accepts_registry(_: &tools::ToolRegistry) {}
    }

    /// Test that session module exports are available.
    #[test]
    fn test_session_exports_available() {
        fn _accepts_role(_: session::MessageRole) {}
        let _: fn() -> anyhow::Result<std::path::PathBuf> = session::get_sessions_dir;
    }

    /// Test that the expected interface is available.
    #[test]
    fn test_interface_compatibility() {
        // Tool interface requirements:
        // - ToolRegistry::new(workspace: PathBuf) -> Self
        // - ToolRegistry::execute_tool(&mut self, name: &str, args: Value) -> Result<Value>
        // - ToolRegistry::available_tools(&self) -> Vec<String>
        // - build_function_declarations() -> Vec<FunctionDeclaration>

        // Session interface requirements:
        // - SessionArchive::new(metadata: SessionArchiveMetadata) -> Result<Self>
        // - SessionArchive::finalize(transcript, count, tools, messages) -> Result<PathBuf>
        // - SessionArchiveMetadata::new(...) -> Self
        // - SessionMessage::with_tool_call_id(role, content, tool_call_id) -> Self
        // - MessageRole::{User, Assistant, System, Tool}
        // - find_session_by_identifier(id) -> Result<Option<SessionListing>>
        // - list_recent_sessions(limit) -> Result<Vec<SessionListing>>
    }
}
