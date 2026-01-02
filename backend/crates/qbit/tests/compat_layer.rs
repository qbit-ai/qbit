//! Integration tests for the compatibility layer.
//!
//! These tests verify that the compat layer correctly provides access to
//! the local qbit implementations (qbit_tools, qbit_core::session).
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test --test compat_layer
//! ```

use qbit_lib::compat;
use serde_json::json;
use tempfile::TempDir;

// =============================================================================
// Tool Registry Tests
// =============================================================================

mod tools {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry_creation() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();
        let registry = compat::tools::ToolRegistry::new(workspace).await;
        let _ = registry;
    }

    #[tokio::test]
    async fn test_tool_registry_available_tools() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();
        let registry = compat::tools::ToolRegistry::new(workspace).await;
        let tools = registry.available_tools();

        assert!(!tools.is_empty(), "ToolRegistry should have available tools");

        let expected_tools = ["read_file", "write_file", "run_pty_cmd"];
        for tool in expected_tools {
            assert!(
                tools.contains(&tool.to_string()),
                "Tool '{}' should be available",
                tool
            );
        }
    }

    #[tokio::test]
    async fn test_tool_registry_execute_read_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let test_content = "Hello, World!";
        std::fs::write(workspace.join("test.txt"), test_content)
            .expect("Failed to create test file");

        let mut registry = compat::tools::ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "test.txt"}))
            .await;

        assert!(result.is_ok(), "read_file should succeed");
        let value = result.unwrap();
        assert!(value.get("error").is_none(), "Successful read should not have error field");

        let content = value.get("content").and_then(|v| v.as_str());
        assert!(content.is_some(), "Result should have content field");
        assert!(content.unwrap().contains(test_content), "Content should contain file contents");
    }

    #[tokio::test]
    async fn test_tool_registry_execute_read_file_not_found() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();

        let mut registry = compat::tools::ToolRegistry::new(workspace).await;
        let result = registry
            .execute_tool("read_file", json!({"path": "nonexistent.txt"}))
            .await;

        assert!(result.is_ok(), "execute_tool should return Ok with error JSON");
        let value = result.unwrap();
        assert!(value.get("error").is_some(), "Failed read should have error field");
    }

    #[test]
    fn test_build_function_declarations() {
        let declarations = compat::tools::build_function_declarations();
        assert!(!declarations.is_empty(), "Should return tool declarations");

        for decl in &declarations {
            assert!(!decl.name.is_empty(), "Declaration should have name");
            assert!(!decl.description.is_empty(), "Declaration '{}' should have description", decl.name);
        }

        let names: Vec<&str> = declarations.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"read_file"), "Should declare read_file");
        assert!(names.contains(&"write_file"), "Should declare write_file");
        assert!(names.contains(&"edit_file"), "Should declare edit_file");
    }

    #[tokio::test]
    async fn test_tool_registry_shell_exit_code_contract() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let workspace = temp.path().to_path_buf();
        let mut registry = compat::tools::ToolRegistry::new(workspace).await;

        // Success case
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "echo hello"}))
            .await;
        assert!(result.is_ok(), "run_pty_cmd should succeed");
        let value = result.unwrap();
        assert_eq!(value.get("exit_code").and_then(|v| v.as_i64()), Some(0));
        assert!(value.get("error").is_none());

        // Failure case
        let result = registry
            .execute_tool("run_pty_cmd", json!({"command": "exit 1"}))
            .await;
        assert!(result.is_ok());
        let value = result.unwrap();
        assert!(value.get("exit_code").and_then(|v| v.as_i64()).map(|c| c != 0).unwrap_or(false));
    }
}

// =============================================================================
// Session Archive Tests
// =============================================================================

mod session {
    use super::*;
    use serial_test::serial;

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

    #[tokio::test]
    #[serial]
    async fn test_session_finalize_and_find() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

        let metadata = compat::session::SessionArchiveMetadata::new(
            "finalize-test",
            "/tmp/workspace".to_string(),
            "test-model",
            "test-provider",
            "default",
            "standard",
        );

        let session_id = metadata.session_id.clone();

        let archive = compat::session::SessionArchive::new(metadata)
            .await
            .expect("Failed to create archive");

        let messages = vec![compat::session::SessionMessage::with_tool_call_id(
            compat::session::MessageRole::User,
            "Test message",
            None,
        )];

        let path = archive
            .finalize(vec!["Test transcript".to_string()], 1, vec![], messages)
            .expect("Failed to finalize");

        assert!(path.exists(), "Session file should exist after finalize");

        let found = compat::session::find_session_by_identifier(&session_id)
            .await
            .expect("find_session should succeed");

        assert!(found.is_some(), "Session should be found by ID");

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[tokio::test]
    #[serial]
    async fn test_list_recent_sessions() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        std::env::set_var("VT_SESSION_DIR", temp.path());

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

            archive.finalize(vec![], 1, vec![], messages).expect("Failed to finalize");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let sessions = compat::session::list_recent_sessions(0).await.expect("list should succeed");
        assert_eq!(sessions.len(), 3, "Should find all 3 sessions");

        let limited = compat::session::list_recent_sessions(2).await.expect("limited list should succeed");
        assert_eq!(limited.len(), 2, "Should respect limit");

        std::env::remove_var("VT_SESSION_DIR");
    }

    #[test]
    fn test_message_role_variants() {
        let _user = compat::session::MessageRole::User;
        let _assistant = compat::session::MessageRole::Assistant;
        let _system = compat::session::MessageRole::System;
        let _tool = compat::session::MessageRole::Tool;
    }

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
