//! Error types for tool execution.

use thiserror::Error;

/// Errors that can occur during tool execution.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool not found in registry
    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    /// Invalid arguments provided to tool
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    /// Missing required argument
    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    /// File system operation failed
    #[error("File operation failed: {0}")]
    FileOperation(String),

    /// Path is outside workspace
    #[error("Path is outside workspace: {0}")]
    PathOutsideWorkspace(String),

    /// Shell command execution failed
    #[error("Shell command failed: {0}")]
    ShellCommand(String),

    /// Command timed out
    #[error("Command timed out after {0} seconds")]
    Timeout(u64),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Regex error
    #[error("Invalid regex pattern: {0}")]
    Regex(String),

    /// Edit failed - no matches found
    #[error("Edit failed: no matches found for the search text")]
    EditNoMatch,

    /// Edit failed - multiple matches found
    #[error("Edit failed: found {0} matches, expected exactly 1")]
    EditMultipleMatches(usize),

    /// File already exists
    #[error("File already exists: {0}")]
    FileExists(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Binary file detected
    #[error("Cannot read binary file: {0}")]
    BinaryFile(String),
}

impl ToolError {
    /// Convert error to JSON value with error field
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": self.to_string()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_json_contains_error_field() {
        let err = ToolError::FileNotFound("test.txt".to_string());
        let json = err.to_json();

        assert!(json.get("error").is_some());
        assert!(json["error"].as_str().unwrap().contains("File not found"));
    }

    #[test]
    fn test_edit_errors() {
        let no_match = ToolError::EditNoMatch;
        assert!(no_match.to_string().contains("no matches"));

        let multi = ToolError::EditMultipleMatches(3);
        assert!(multi.to_string().contains("3 matches"));
    }
}
