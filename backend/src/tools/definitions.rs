//! Tool function declarations for LLM consumption.
//!
//! This module provides the `build_function_declarations()` function that returns
//! tool schemas in the format expected by LLM providers.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Function declaration format for LLM tool calling.
///
/// This struct matches the format expected by vtcode_core::tools::registry::FunctionDeclaration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    /// Tool name (must match what the tool registry expects)
    pub name: String,
    /// Human-readable description for the LLM
    pub description: String,
    /// JSON Schema for the tool's parameters
    pub parameters: Value,
}

/// Build all tool declarations for LLM consumption.
///
/// This is a drop-in replacement for vtcode_core::tools::registry::build_function_declarations().
///
/// Returns a vector of function declarations that describe all available tools
/// and their parameter schemas.
pub fn build_function_declarations() -> Vec<FunctionDeclaration> {
    vec![
        // ====================================================================
        // File Operations
        // ====================================================================
        FunctionDeclaration {
            name: "read_file".to_string(),
            description: "Read the contents of a file. Supports optional line range for reading specific sections.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "line_start": {
                        "type": "integer",
                        "description": "Starting line number (1-indexed)"
                    },
                    "line_end": {
                        "type": "integer",
                        "description": "Ending line number (1-indexed, inclusive)"
                    }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "write_file".to_string(),
            description: "Write content to a file, replacing existing content. Creates the file and parent directories if they don't exist.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "create_file".to_string(),
            description: "Create a new file with the specified content. Fails if the file already exists.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path for the new file (relative to workspace)"
                    },
                    "content": {
                        "type": "string",
                        "description": "Initial content for the file"
                    }
                },
                "required": ["path", "content"]
            }),
        },
        FunctionDeclaration {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing text. The old_text must match exactly once in the file.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "Text to find and replace (must match exactly once)"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "Replacement text"
                    },
                    "display_description": {
                        "type": "string",
                        "description": "Human-readable description of the edit"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        FunctionDeclaration {
            name: "delete_file".to_string(),
            description: "Delete a file from the filesystem.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file to delete (relative to workspace)"
                    }
                },
                "required": ["path"]
            }),
        },
        // ====================================================================
        // Directory Operations
        // ====================================================================
        FunctionDeclaration {
            name: "list_files".to_string(),
            description: "List files matching a glob pattern. Respects .gitignore by default.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to search (relative to workspace, default: root)"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match files (e.g., '*.rs', '**/*.ts')"
                    },
                    "recursive": {
                        "type": "boolean",
                        "description": "Search recursively (default: true)"
                    }
                },
                "required": []
            }),
        },
        FunctionDeclaration {
            name: "list_directory".to_string(),
            description: "List the contents of a directory with file/directory type indicators.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path (relative to workspace)"
                    }
                },
                "required": ["path"]
            }),
        },
        FunctionDeclaration {
            name: "grep_file".to_string(),
            description: "Search file contents using regex pattern. Returns matching lines with file paths and line numbers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "File or directory to search (default: workspace root)"
                    },
                    "include": {
                        "type": "string",
                        "description": "Glob pattern to filter files (e.g., '*.rs')"
                    }
                },
                "required": ["pattern"]
            }),
        },
        // ====================================================================
        // Planning
        // ====================================================================
        FunctionDeclaration {
            name: "update_plan".to_string(),
            description: "Create or update the task plan. Use this to track progress on multi-step tasks. Each step should have a description and status (pending, in_progress, or completed). Only one step can be in_progress at a time.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "explanation": {
                        "type": "string",
                        "description": "Optional high-level explanation or summary of the plan"
                    },
                    "plan": {
                        "type": "array",
                        "description": "List of plan steps (1-12 steps)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "step": {
                                    "type": "string",
                                    "description": "Description of this step"
                                },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"],
                                    "description": "Current status of the step"
                                }
                            },
                            "required": ["step"]
                        }
                    }
                },
                "required": ["plan"]
            }),
        },
        // ====================================================================
        // Shell Execution
        // ====================================================================
        FunctionDeclaration {
            name: "run_pty_cmd".to_string(),
            description: "Execute a shell command and return the output. Commands run in a shell environment with access to common tools.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "Shell command to execute"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (relative to workspace)"
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Timeout in seconds (default: 120)"
                    }
                },
                "required": ["command"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_function_declarations_returns_all_tools() {
        let declarations = build_function_declarations();

        // Should have exactly 10 tools (9 original + update_plan)
        assert_eq!(declarations.len(), 10);

        // Verify all expected tools are present
        let names: Vec<&str> = declarations.iter().map(|d| d.name.as_str()).collect();

        // File operations
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"create_file"));
        assert!(names.contains(&"edit_file"));
        assert!(names.contains(&"delete_file"));

        // Directory operations
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"grep_file"));

        // Shell
        assert!(names.contains(&"run_pty_cmd"));

        // Planning
        assert!(names.contains(&"update_plan"));
    }

    #[test]
    fn test_declarations_have_valid_schemas() {
        let declarations = build_function_declarations();

        for decl in declarations {
            // Each declaration should have a non-empty name
            assert!(!decl.name.is_empty(), "Declaration should have a name");

            // Each declaration should have a non-empty description
            assert!(
                !decl.description.is_empty(),
                "Declaration should have a description"
            );

            // Parameters should be an object type
            assert_eq!(
                decl.parameters.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "Parameters should be an object type for {}",
                decl.name
            );

            // Parameters should have a properties field
            assert!(
                decl.parameters.get("properties").is_some(),
                "Parameters should have properties for {}",
                decl.name
            );
        }
    }

    #[test]
    fn test_read_file_declaration() {
        let declarations = build_function_declarations();
        let read_file = declarations
            .iter()
            .find(|d| d.name == "read_file")
            .expect("read_file should exist");

        // Should have path as required
        let required = read_file.parameters["required"].as_array().unwrap();
        assert!(required.contains(&json!("path")));

        // Should have line_start and line_end as optional
        let props = read_file.parameters["properties"].as_object().unwrap();
        assert!(props.contains_key("path"));
        assert!(props.contains_key("line_start"));
        assert!(props.contains_key("line_end"));
    }

    #[test]
    fn test_edit_file_declaration() {
        let declarations = build_function_declarations();
        let edit_file = declarations
            .iter()
            .find(|d| d.name == "edit_file")
            .expect("edit_file should exist");

        // Should have path, old_text, new_text as required
        let required = edit_file.parameters["required"].as_array().unwrap();
        assert!(required.contains(&json!("path")));
        assert!(required.contains(&json!("old_text")));
        assert!(required.contains(&json!("new_text")));
    }

    #[test]
    fn test_run_pty_cmd_declaration() {
        let declarations = build_function_declarations();
        let run_pty_cmd = declarations
            .iter()
            .find(|d| d.name == "run_pty_cmd")
            .expect("run_pty_cmd should exist");

        // Should have command as required
        let required = run_pty_cmd.parameters["required"].as_array().unwrap();
        assert!(required.contains(&json!("command")));

        // Should have cwd and timeout as optional
        let props = run_pty_cmd.parameters["properties"].as_object().unwrap();
        assert!(props.contains_key("command"));
        assert!(props.contains_key("cwd"));
        assert!(props.contains_key("timeout"));
    }

    #[test]
    fn test_function_declaration_serialization() {
        let decl = FunctionDeclaration {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "arg": {"type": "string"}
                },
                "required": ["arg"]
            }),
        };

        // Should serialize to JSON
        let json_str = serde_json::to_string(&decl).unwrap();
        assert!(json_str.contains("test_tool"));

        // Should deserialize back
        let parsed: FunctionDeclaration = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.name, "test_tool");
    }
}
