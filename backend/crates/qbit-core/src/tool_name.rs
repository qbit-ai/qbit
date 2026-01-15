//! Tool name enumeration and categorization.
//!
//! This module provides type-safe tool name handling through the `ToolName` enum,
//! replacing string-based tool names throughout the codebase.

use serde::{Deserialize, Serialize};

/// Enumeration of all known tool names.
///
/// This provides type-safe tool identification, preventing typos and enabling
/// exhaustive matching in tool handlers and hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ToolName {
    // === File Operations ===
    /// Read contents of a file
    ReadFile,
    /// Write contents to a file (overwrite)
    WriteFile,
    /// Edit a file with search/replace
    EditFile,
    /// Create a new file
    CreateFile,
    /// Delete a file
    DeleteFile,

    // === Directory Operations ===
    /// List files matching a pattern
    ListFiles,
    /// List directory contents
    ListDirectory,
    /// Search file contents with grep
    GrepFile,

    // === Shell Execution ===
    /// Execute a command in PTY
    RunPtyCmd,
    /// Alias for RunPtyCmd (user-friendly name)
    RunCommand,

    // === Web Operations ===
    /// Fetch and extract web content
    WebFetch,
    /// Web search via Tavily
    WebSearch,
    /// Web search with answer via Tavily
    WebSearchAnswer,
    /// Extract content from URLs via Tavily
    WebExtract,
    /// Crawl website via Tavily
    WebCrawl,
    /// Map website structure via Tavily
    WebMap,

    // === Planning ===
    /// Update task plan
    UpdatePlan,

    // === Code Indexer ===
    /// Search code in index
    IndexerSearchCode,
    /// Search files in index
    IndexerSearchFiles,
    /// Analyze a file's structure
    IndexerAnalyzeFile,
    /// Extract symbols from a file
    IndexerExtractSymbols,
    /// Get code metrics for a file
    IndexerGetMetrics,
    /// Detect file language
    IndexerDetectLanguage,

    // === AST Operations ===
    /// AST-based code search
    AstGrep,
    /// AST-based code replacement
    AstGrepReplace,

    // === Workflow ===
    /// Execute a workflow
    RunWorkflow,
}

impl ToolName {
    /// Get the string representation of the tool name.
    ///
    /// This returns the exact string that matches what the LLM requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            // File Operations
            Self::ReadFile => "read_file",
            Self::WriteFile => "write_file",
            Self::EditFile => "edit_file",
            Self::CreateFile => "create_file",
            Self::DeleteFile => "delete_file",

            // Directory Operations
            Self::ListFiles => "list_files",
            Self::ListDirectory => "list_directory",
            Self::GrepFile => "grep_file",

            // Shell
            Self::RunPtyCmd => "run_pty_cmd",
            Self::RunCommand => "run_command",

            // Web
            Self::WebFetch => "web_fetch",
            Self::WebSearch => "web_search",
            Self::WebSearchAnswer => "web_search_answer",
            Self::WebExtract => "web_extract",
            Self::WebCrawl => "web_crawl",
            Self::WebMap => "web_map",

            // Planning
            Self::UpdatePlan => "update_plan",

            // Indexer
            Self::IndexerSearchCode => "indexer_search_code",
            Self::IndexerSearchFiles => "indexer_search_files",
            Self::IndexerAnalyzeFile => "indexer_analyze_file",
            Self::IndexerExtractSymbols => "indexer_extract_symbols",
            Self::IndexerGetMetrics => "indexer_get_metrics",
            Self::IndexerDetectLanguage => "indexer_detect_language",

            // AST
            Self::AstGrep => "ast_grep",
            Self::AstGrepReplace => "ast_grep_replace",

            // Workflow
            Self::RunWorkflow => "run_workflow",
        }
    }

    /// Parse a tool name from a string.
    ///
    /// Returns `None` for unknown tool names (e.g., dynamic sub-agent tools).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            // File Operations
            "read_file" => Some(Self::ReadFile),
            "write_file" => Some(Self::WriteFile),
            "edit_file" => Some(Self::EditFile),
            "create_file" => Some(Self::CreateFile),
            "delete_file" => Some(Self::DeleteFile),

            // Directory Operations
            "list_files" => Some(Self::ListFiles),
            "list_directory" => Some(Self::ListDirectory),
            "grep_file" => Some(Self::GrepFile),

            // Shell
            "run_pty_cmd" => Some(Self::RunPtyCmd),
            "run_command" => Some(Self::RunCommand),

            // Web
            "web_fetch" => Some(Self::WebFetch),
            "web_search" | "tavily_search" => Some(Self::WebSearch),
            "web_search_answer" | "tavily_search_answer" => Some(Self::WebSearchAnswer),
            "web_extract" | "tavily_extract" => Some(Self::WebExtract),
            "web_crawl" | "tavily_crawl" => Some(Self::WebCrawl),
            "web_map" | "tavily_map" => Some(Self::WebMap),

            // Planning
            "update_plan" => Some(Self::UpdatePlan),

            // Indexer
            "indexer_search_code" => Some(Self::IndexerSearchCode),
            "indexer_search_files" => Some(Self::IndexerSearchFiles),
            "indexer_analyze_file" => Some(Self::IndexerAnalyzeFile),
            "indexer_extract_symbols" => Some(Self::IndexerExtractSymbols),
            "indexer_get_metrics" => Some(Self::IndexerGetMetrics),
            "indexer_detect_language" => Some(Self::IndexerDetectLanguage),

            // AST
            "ast_grep" => Some(Self::AstGrep),
            "ast_grep_replace" => Some(Self::AstGrepReplace),

            // Workflow
            "run_workflow" => Some(Self::RunWorkflow),

            // Unknown (includes dynamic sub-agent tools like "sub_agent_*")
            _ => None,
        }
    }

    /// Get the semantic category of this tool.
    pub fn category(&self) -> ToolCategory {
        match self {
            // File Operations
            Self::ReadFile | Self::WriteFile | Self::EditFile | Self::CreateFile | Self::DeleteFile => {
                ToolCategory::FileOps
            }

            // Directory Operations
            Self::ListFiles | Self::ListDirectory | Self::GrepFile => ToolCategory::DirectoryOps,

            // Shell
            Self::RunPtyCmd | Self::RunCommand => ToolCategory::Shell,

            // Web
            Self::WebFetch | Self::WebSearch | Self::WebSearchAnswer | Self::WebExtract | Self::WebCrawl | Self::WebMap => {
                ToolCategory::Web
            }

            // Planning
            Self::UpdatePlan => ToolCategory::Planning,

            // Indexer
            Self::IndexerSearchCode
            | Self::IndexerSearchFiles
            | Self::IndexerAnalyzeFile
            | Self::IndexerExtractSymbols
            | Self::IndexerGetMetrics
            | Self::IndexerDetectLanguage => ToolCategory::Indexer,

            // AST
            Self::AstGrep | Self::AstGrepReplace => ToolCategory::Ast,

            // Workflow
            Self::RunWorkflow => ToolCategory::Workflow,
        }
    }

    /// Check if this tool is read-only (doesn't modify files or execute commands).
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::ReadFile
                | Self::ListFiles
                | Self::ListDirectory
                | Self::GrepFile
                | Self::WebFetch
                | Self::WebSearch
                | Self::WebSearchAnswer
                | Self::WebExtract
                | Self::WebCrawl
                | Self::WebMap
                | Self::IndexerSearchCode
                | Self::IndexerSearchFiles
                | Self::IndexerAnalyzeFile
                | Self::IndexerExtractSymbols
                | Self::IndexerGetMetrics
                | Self::IndexerDetectLanguage
                | Self::AstGrep
        )
    }

    /// Check if this is a sub-agent tool name.
    ///
    /// Sub-agent tools are dynamically named as "sub_agent_<id>".
    pub fn is_sub_agent_tool(name: &str) -> bool {
        name.starts_with("sub_agent_")
    }

    /// Extract the sub-agent ID from a sub-agent tool name.
    ///
    /// Returns `None` if the name is not a sub-agent tool.
    pub fn sub_agent_id(name: &str) -> Option<&str> {
        name.strip_prefix("sub_agent_")
    }
}

impl std::fmt::Display for ToolName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl AsRef<str> for ToolName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// Semantic categorization of tools for hook matching and policy enforcement.
///
/// This differs from the routing-based `ToolCategory` in `tool_execution.rs` -
/// this is about semantic grouping (what the tool *does*), not how it's routed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// File read/write/edit operations
    FileOps,
    /// Directory listing and search operations
    DirectoryOps,
    /// Shell command execution
    Shell,
    /// Web fetching and search operations
    Web,
    /// Task planning operations
    Planning,
    /// Code indexing and analysis
    Indexer,
    /// AST-based code operations
    Ast,
    /// Multi-step workflow execution
    Workflow,
    /// Sub-agent delegation
    SubAgent,
}

impl ToolCategory {
    /// Get all tool names in this category.
    pub fn tools(&self) -> &'static [ToolName] {
        match self {
            Self::FileOps => &[
                ToolName::ReadFile,
                ToolName::WriteFile,
                ToolName::EditFile,
                ToolName::CreateFile,
                ToolName::DeleteFile,
            ],
            Self::DirectoryOps => &[
                ToolName::ListFiles,
                ToolName::ListDirectory,
                ToolName::GrepFile,
            ],
            Self::Shell => &[ToolName::RunPtyCmd, ToolName::RunCommand],
            Self::Web => &[
                ToolName::WebFetch,
                ToolName::WebSearch,
                ToolName::WebSearchAnswer,
                ToolName::WebExtract,
                ToolName::WebCrawl,
                ToolName::WebMap,
            ],
            Self::Planning => &[ToolName::UpdatePlan],
            Self::Indexer => &[
                ToolName::IndexerSearchCode,
                ToolName::IndexerSearchFiles,
                ToolName::IndexerAnalyzeFile,
                ToolName::IndexerExtractSymbols,
                ToolName::IndexerGetMetrics,
                ToolName::IndexerDetectLanguage,
            ],
            Self::Ast => &[ToolName::AstGrep, ToolName::AstGrepReplace],
            Self::Workflow => &[ToolName::RunWorkflow],
            Self::SubAgent => &[], // Dynamic, not enumerable
        }
    }

    /// Check if this category contains read-only tools.
    pub fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::DirectoryOps | Self::Indexer
        )
    }
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileOps => write!(f, "file_ops"),
            Self::DirectoryOps => write!(f, "directory_ops"),
            Self::Shell => write!(f, "shell"),
            Self::Web => write!(f, "web"),
            Self::Planning => write!(f, "planning"),
            Self::Indexer => write!(f, "indexer"),
            Self::Ast => write!(f, "ast"),
            Self::Workflow => write!(f, "workflow"),
            Self::SubAgent => write!(f, "sub_agent"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_name_roundtrip() {
        let tools = [
            ToolName::ReadFile,
            ToolName::WriteFile,
            ToolName::EditFile,
            ToolName::RunPtyCmd,
            ToolName::WebFetch,
            ToolName::UpdatePlan,
            ToolName::IndexerSearchCode,
            ToolName::AstGrep,
        ];

        for tool in tools {
            let s = tool.as_str();
            let parsed = ToolName::from_str(s);
            assert_eq!(parsed, Some(tool), "Roundtrip failed for {:?}", tool);
        }
    }

    #[test]
    fn test_tool_name_from_str_unknown() {
        assert_eq!(ToolName::from_str("unknown_tool"), None);
        assert_eq!(ToolName::from_str("sub_agent_coder"), None);
        assert_eq!(ToolName::from_str(""), None);
    }

    #[test]
    fn test_tool_name_aliases() {
        // tavily_* should map to web_*
        assert_eq!(ToolName::from_str("tavily_search"), Some(ToolName::WebSearch));
        assert_eq!(ToolName::from_str("web_search"), Some(ToolName::WebSearch));
        assert_eq!(ToolName::from_str("tavily_extract"), Some(ToolName::WebExtract));
    }

    #[test]
    fn test_tool_category() {
        assert_eq!(ToolName::ReadFile.category(), ToolCategory::FileOps);
        assert_eq!(ToolName::WriteFile.category(), ToolCategory::FileOps);
        assert_eq!(ToolName::RunPtyCmd.category(), ToolCategory::Shell);
        assert_eq!(ToolName::WebFetch.category(), ToolCategory::Web);
        assert_eq!(ToolName::UpdatePlan.category(), ToolCategory::Planning);
        assert_eq!(ToolName::IndexerSearchCode.category(), ToolCategory::Indexer);
    }

    #[test]
    fn test_is_read_only() {
        assert!(ToolName::ReadFile.is_read_only());
        assert!(ToolName::ListFiles.is_read_only());
        assert!(ToolName::GrepFile.is_read_only());
        assert!(ToolName::WebSearch.is_read_only());
        assert!(ToolName::IndexerSearchCode.is_read_only());
        assert!(ToolName::AstGrep.is_read_only());

        assert!(!ToolName::WriteFile.is_read_only());
        assert!(!ToolName::EditFile.is_read_only());
        assert!(!ToolName::RunPtyCmd.is_read_only());
        assert!(!ToolName::AstGrepReplace.is_read_only());
    }

    #[test]
    fn test_sub_agent_detection() {
        assert!(ToolName::is_sub_agent_tool("sub_agent_coder"));
        assert!(ToolName::is_sub_agent_tool("sub_agent_researcher"));
        assert!(!ToolName::is_sub_agent_tool("read_file"));
        assert!(!ToolName::is_sub_agent_tool("sub_agent"));

        assert_eq!(ToolName::sub_agent_id("sub_agent_coder"), Some("coder"));
        assert_eq!(ToolName::sub_agent_id("read_file"), None);
    }

    #[test]
    fn test_category_tools() {
        let file_ops = ToolCategory::FileOps.tools();
        assert!(file_ops.contains(&ToolName::ReadFile));
        assert!(file_ops.contains(&ToolName::WriteFile));
        assert!(!file_ops.contains(&ToolName::RunPtyCmd));

        let shell = ToolCategory::Shell.tools();
        assert!(shell.contains(&ToolName::RunPtyCmd));
        assert!(shell.contains(&ToolName::RunCommand));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ToolName::ReadFile), "read_file");
        assert_eq!(format!("{}", ToolCategory::FileOps), "file_ops");
    }

    #[test]
    fn test_serde_roundtrip() {
        let tool = ToolName::ReadFile;
        let json = serde_json::to_string(&tool).unwrap();
        assert_eq!(json, "\"read_file\"");

        let parsed: ToolName = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tool);
    }
}
