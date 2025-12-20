//! Integration tests for the compatibility layer.
//!
//! These tests verify that the compat layer correctly switches between
//! vtcode-core and local implementations based on the `local-tools` feature flag.
//!
//! ## Running Tests
//!
//! ```bash
//! # Test with vtcode-core (default)
//! cargo test --test compat_layer
//!
//! # Test with local implementations
//! cargo test --test compat_layer --features local-tools
//! ```
//!
//! ## Implementation Differences
//!
//! Note: There are some interface differences between vtcode-core and local
//! implementations that these tests account for:
//!
//! - vtcode-core's `available_tools()` is async, local is sync
//! - Session ID access differs between implementations
//!
//! The tests use conditional compilation to handle these differences while
//! verifying core functionality works correctly.

use qbit_lib::compat;
use serde_json::json;
use tempfile::TempDir;

// =============================================================================
// Tool Registry Tests
// =============================================================================

mod tools {
    use super::*;

    /// Verify that ToolRegistry can be created with the current implementation.
    #[tokio::test]
    async fn test_tool_registry_creation() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let registry = compat::tools::ToolRegistry::new(workspace).await;

        // Registry should be created successfully
        // (both implementations have the same constructor signature)
        let _ = registry;
    }

    /// Verify that available_tools() returns a list of tool names.
    #[tokio::test]
    async fn test_tool_registry_available_tools() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let registry = compat::tools::ToolRegistry::new(workspace).await;

        // Note: vtcode-core's available_tools() is async, local is sync
        // Use conditional compilation to handle the difference
        #[cfg(feature = "local-tools")]
        let tools = registry.available_tools();

        #[cfg(not(feature = "local-tools"))]
        let tools = registry.available_tools().await;

        // Both implementations should return some tools
        assert!(
            !tools.is_empty(),
            "ToolRegistry should have available tools"
        );

        // Core tools should be available in both implementations
        let expected_tools = ["read_file", "write_file", "run_pty_cmd"];
        for tool in expected_tools {
            assert!(
                tools.contains(&tool.to_string()),
                "Tool '{}' should be available",
                tool
            );
        }
    }

    /// Verify that execute_tool() works for a basic read operation.
    #[tokio::test]
    async fn test_tool_registry_execute_read_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        // Create a test file
        let test_content = "Hello, World!";
        std::fs::write(workspace.join("test.txt"), test_content)
            .expect("Failed to create test file");

        let mut registry = compat::tools::ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "test.txt"}))
            .await;

        // Should succeed
        assert!(result.is_ok(), "read_file should succeed");

        let value = result.unwrap();

        // Success format: should NOT have "error" field
        assert!(
            value.get("error").is_none(),
            "Successful read should not have error field"
        );

        // Should have content field with file contents
        let content = value.get("content").and_then(|v| v.as_str());
        assert!(content.is_some(), "Result should have content field");
        assert!(
            content.unwrap().contains(test_content),
            "Content should contain file contents"
        );
    }

    /// Verify that execute_tool() returns error format for missing files.
    #[tokio::test]
    async fn test_tool_registry_execute_read_file_not_found() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let mut registry = compat::tools::ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "nonexistent.txt"}))
            .await;

        // Should succeed (returns JSON with error, doesn't throw)
        assert!(
            result.is_ok(),
            "execute_tool should return Ok with error JSON"
        );

        let value = result.unwrap();

        // Failure format: should have "error" field
        assert!(
            value.get("error").is_some(),
            "Failed read should have error field"
        );
    }

    /// Verify that build_function_declarations() returns tool schemas.
    #[test]
    fn test_build_function_declarations() {
        let declarations = compat::tools::build_function_declarations();

        // Should return multiple declarations
        assert!(!declarations.is_empty(), "Should return tool declarations");

        // Each declaration should have name, description, and parameters
        for decl in &declarations {
            assert!(!decl.name.is_empty(), "Declaration should have name");
            assert!(
                !decl.description.is_empty(),
                "Declaration '{}' should have description",
                decl.name
            );
        }

        // Core tools should be declared
        let names: Vec<&str> = declarations.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"read_file"), "Should declare read_file");
        assert!(names.contains(&"write_file"), "Should declare write_file");
        assert!(names.contains(&"edit_file"), "Should declare edit_file");
    }

    /// Verify the success/failure contract for shell commands.
    #[tokio::test]
    async fn test_tool_registry_shell_exit_code_contract() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let mut registry = compat::tools::ToolRegistry::new(workspace).await;

        // Success case: exit code 0
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "echo hello"}))
            .await;

        assert!(result.is_ok(), "run_pty_cmd should succeed");
        let value = result.unwrap();

        let exit_code = value.get("exit_code").and_then(|v| v.as_i64());
        assert_eq!(
            exit_code,
            Some(0),
            "Successful command should have exit_code 0"
        );
        assert!(
            value.get("error").is_none(),
            "Successful command should not have error field"
        );

        // Failure case: non-zero exit code
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "exit 1"}))
            .await;

        assert!(
            result.is_ok(),
            "run_pty_cmd should return Ok even for failures"
        );
        let value = result.unwrap();

        let exit_code = value.get("exit_code").and_then(|v| v.as_i64());
        assert!(
            exit_code.map(|c| c != 0).unwrap_or(false),
            "Failed command should have non-zero exit_code"
        );
    }
}

// =============================================================================
// Session Archive Tests
// =============================================================================

mod session {
    use super::*;
    use serial_test::serial;

    /// Verify that SessionArchive can be created.
    #[tokio::test]
    #[serial]
    async fn test_session_archive_creation() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

        let metadata = compat::session::SessionArchiveMetadata::new(
            "test-workspace",
            "/tmp/workspace".to_string(),
            "test-model",
            "test-provider",
            "default",
            "standard",
        );

        let archive = compat::session::SessionArchive::new(metadata).await;
        assert!(archive.is_ok(), "SessionArchive creation should succeed");

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Verify that sessions can be finalized and retrieved.
    #[tokio::test]
    #[serial]
    async fn test_session_finalize_and_find() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

        // Create and finalize a session
        let metadata = compat::session::SessionArchiveMetadata::new(
            "finalize-test",
            "/tmp/workspace".to_string(),
            "test-model",
            "test-provider",
            "default",
            "standard",
        );

        // Note: session_id access differs between implementations
        // Local has it on metadata, vtcode-core has it on the listing
        #[cfg(feature = "local-tools")]
        let session_id = metadata.session_id.clone();

        let archive = compat::session::SessionArchive::new(metadata)
            .await
            .expect("Failed to create archive");

        // Create a test message
        let messages = vec![compat::session::SessionMessage::with_tool_call_id(
            compat::session::MessageRole::User,
            "Test message",
            None,
        )];

        let path = archive
            .finalize(vec!["Test transcript".to_string()], 1, vec![], messages)
            .expect("Failed to finalize");

        assert!(path.exists(), "Session file should exist after finalize");

        // For vtcode-core, we need to find the session by listing and extracting ID
        #[cfg(not(feature = "local-tools"))]
        let session_id = {
            let sessions = compat::session::list_recent_sessions(1)
                .await
                .expect("list should work");
            assert!(!sessions.is_empty(), "Should have at least one session");
            sessions[0].identifier()
        };

        // Find the session
        let found = compat::session::find_session_by_identifier(&session_id)
            .await
            .expect("find_session should succeed");

        assert!(found.is_some(), "Session should be found by ID");

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Verify that list_recent_sessions works.
    #[tokio::test]
    #[serial]
    async fn test_list_recent_sessions() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

        // Create multiple sessions
        for i in 0..3 {
            let metadata = compat::session::SessionArchiveMetadata::new(
                &format!("list-test-{}", i),
                format!("/tmp/workspace-{}", i),
                "model",
                "provider",
                "default",
                "standard",
            );

            let archive = compat::session::SessionArchive::new(metadata)
                .await
                .expect("Failed to create archive");

            let messages = vec![compat::session::SessionMessage::with_tool_call_id(
                compat::session::MessageRole::User,
                &format!("Message {}", i),
                None,
            )];

            archive
                .finalize(vec![], 1, vec![], messages)
                .expect("Failed to finalize");

            // Small delay to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // List all sessions
        let sessions = compat::session::list_recent_sessions(0)
            .await
            .expect("list should succeed");

        assert_eq!(sessions.len(), 3, "Should find all 3 sessions");

        // List with limit
        let limited = compat::session::list_recent_sessions(2)
            .await
            .expect("limited list should succeed");

        assert_eq!(limited.len(), 2, "Should respect limit");

        std::env::remove_var("VT_SESSION_DIR");
    }

    /// Verify MessageRole enum variants.
    #[test]
    fn test_message_role_variants() {
        // All implementations should have these variants
        let _user = compat::session::MessageRole::User;
        let _assistant = compat::session::MessageRole::Assistant;
        let _system = compat::session::MessageRole::System;
        let _tool = compat::session::MessageRole::Tool;
    }

    /// Verify get_sessions_dir returns a valid path.
    #[test]
    #[serial]
    fn test_get_sessions_dir() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

        let dir = compat::session::get_sessions_dir().expect("Should get sessions dir");

        assert!(dir.exists(), "Sessions dir should exist");
        assert_eq!(dir, temp.path(), "Should respect VT_SESSION_DIR env var");

        std::env::remove_var("VT_SESSION_DIR");
    }
}

// =============================================================================
// Feature Flag Verification
// =============================================================================

mod feature_flags {
    /// Document which implementation is being tested.
    #[test]
    fn test_report_active_implementation() {
        #[cfg(feature = "local-tools")]
        {
            println!("Testing with LOCAL implementations (local-tools feature enabled)");
            assert!(true);
        }

        #[cfg(not(feature = "local-tools"))]
        {
            println!("Testing with VTCODE-CORE implementations (default)");
            assert!(true);
        }
    }

    /// Verify that both implementations can coexist in the codebase.
    ///
    /// This test doesn't actually run both - it verifies that the conditional
    /// compilation is set up correctly. To test both implementations:
    ///
    /// ```bash
    /// cargo test --test compat_layer
    /// cargo test --test compat_layer --features local-tools
    /// ```
    #[test]
    fn test_conditional_compilation_setup() {
        // This test passes if it compiles, which means the feature flag
        // conditional compilation is set up correctly.

        // The actual type being used changes based on feature flag,
        // but the interface remains the same.
        type Registry = qbit_lib::compat::tools::ToolRegistry;

        // If this compiles, the type is accessible regardless of feature flag
        fn _accepts_registry(_: &Registry) {}
    }
}
