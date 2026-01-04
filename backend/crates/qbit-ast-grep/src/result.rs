//! Result types for AST-grep operations.

use serde::{Deserialize, Serialize};

/// A single match from an AST-grep search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    /// Path to the file containing the match (relative to workspace).
    pub file: String,
    /// Line number where the match starts (1-indexed).
    pub line: usize,
    /// Column number where the match starts (1-indexed).
    pub column: usize,
    /// The matched text.
    pub text: String,
    /// End line number (1-indexed).
    pub end_line: usize,
    /// End column number (1-indexed).
    pub end_column: usize,
}

/// Result of an AST-grep search operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// All matches found.
    pub matches: Vec<SearchMatch>,
    /// Number of files searched.
    pub files_searched: usize,
}

impl SearchResult {
    /// Create a new empty search result.
    pub fn new() -> Self {
        Self {
            matches: Vec::new(),
            files_searched: 0,
        }
    }

    /// Get the number of matches.
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Check if there are no matches.
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }
}

impl Default for SearchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A single replacement made during an AST-grep replace operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    /// Path to the file that was modified (relative to workspace).
    pub file: String,
    /// Line number where the replacement was made (1-indexed).
    pub line: usize,
    /// The original text that was replaced.
    pub original: String,
    /// The new text after replacement.
    pub replacement: String,
}

/// Result of an AST-grep replace operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceResult {
    /// List of files that were modified.
    pub files_modified: Vec<String>,
    /// Total number of replacements made.
    pub replacements_count: usize,
    /// Details of each replacement.
    pub changes: Vec<Replacement>,
}

impl ReplaceResult {
    /// Create a new empty replace result.
    pub fn new() -> Self {
        Self {
            files_modified: Vec::new(),
            replacements_count: 0,
            changes: Vec::new(),
        }
    }
}

impl Default for ReplaceResult {
    fn default() -> Self {
        Self::new()
    }
}
