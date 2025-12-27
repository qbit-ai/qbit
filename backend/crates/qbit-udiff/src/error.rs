//! Error types for unified diff parsing and application.

use std::fmt;

/// Error that occurred while applying a patch
#[derive(Debug, Clone)]
pub struct PatchError {
    /// Path to the file where the error occurred
    pub file_path: String,
    /// Index of the hunk that failed (0-based)
    pub hunk_idx: usize,
    /// Type of error that occurred
    pub error_type: PatchErrorType,
    /// Suggestion for how to fix the error
    pub suggestion: String,
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Patch error in {} (hunk {}): {}. Suggestion: {}",
            self.file_path, self.hunk_idx, self.error_type, self.suggestion
        )
    }
}

impl std::error::Error for PatchError {}

/// Type of patch error
#[derive(Debug, Clone)]
pub enum PatchErrorType {
    /// File not found at the specified path
    FileNotFound,
    /// No match found for the hunk's context
    NoMatch {
        /// The text that was searched for
        searched_for: String,
    },
    /// Multiple matches found for the hunk's context
    MultipleMatches {
        /// Number of matches found
        count: usize,
    },
    /// Invalid diff format
    InvalidFormat {
        /// Details about the formatting issue
        detail: String,
    },
}

impl fmt::Display for PatchErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PatchErrorType::FileNotFound => write!(f, "File not found"),
            PatchErrorType::NoMatch { searched_for } => {
                write!(f, "No match found for context: {}", searched_for)
            }
            PatchErrorType::MultipleMatches { count } => {
                write!(f, "Found {} matches, need unique context", count)
            }
            PatchErrorType::InvalidFormat { detail } => {
                write!(f, "Invalid format: {}", detail)
            }
        }
    }
}
