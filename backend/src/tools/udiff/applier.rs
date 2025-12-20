//! Apply unified diffs to file contents with flexible matching.

use super::parser::ParsedHunk;

/// Result of applying hunks to a file
#[derive(Debug, Clone, PartialEq)]
pub enum ApplyResult {
    /// All hunks applied successfully
    Success {
        /// The new content after applying all hunks
        new_content: String,
    },
    /// Some hunks applied, some failed
    PartialSuccess {
        /// Indices of successfully applied hunks
        applied: Vec<usize>,
        /// Indices and error messages of failed hunks
        failed: Vec<(usize, String)>,
        /// The content after applying successful hunks
        new_content: String,
    },
    /// A hunk could not be matched
    NoMatch {
        /// Index of the hunk that failed
        hunk_idx: usize,
        /// Suggestion for fixing the issue
        suggestion: String,
    },
    /// Multiple matches found for a hunk
    MultipleMatches {
        /// Index of the hunk that failed
        hunk_idx: usize,
        /// Number of matches found
        count: usize,
    },
}

/// Applier for unified diffs
pub struct UdiffApplier;

impl UdiffApplier {
    /// Apply hunks to file content
    ///
    /// Tries multiple matching strategies in order:
    /// 1. Direct exact match
    /// 2. Normalized match (ignoring leading/trailing whitespace)
    /// 3. Fuzzy match (TODO: implement with similar crate)
    pub fn apply_hunks(content: &str, hunks: &[ParsedHunk]) -> ApplyResult {
        let mut current_content = content.to_string();
        let mut applied = Vec::new();
        let mut failed = Vec::new();

        for (idx, hunk) in hunks.iter().enumerate() {
            match Self::apply_single_hunk(&current_content, hunk) {
                Ok(new_content) => {
                    current_content = new_content;
                    applied.push(idx);
                }
                Err(HunkApplyError::NoMatch { suggestion }) => {
                    if applied.is_empty() {
                        // No hunks applied yet, return NoMatch
                        return ApplyResult::NoMatch {
                            hunk_idx: idx,
                            suggestion,
                        };
                    } else {
                        // Some hunks already applied
                        failed.push((idx, suggestion));
                    }
                }
                Err(HunkApplyError::MultipleMatches { count }) => {
                    if applied.is_empty() {
                        return ApplyResult::MultipleMatches {
                            hunk_idx: idx,
                            count,
                        };
                    } else {
                        failed.push((idx, format!("Found {} matches, need more context", count)));
                    }
                }
            }
        }

        if failed.is_empty() {
            ApplyResult::Success {
                new_content: current_content,
            }
        } else {
            ApplyResult::PartialSuccess {
                applied,
                failed,
                new_content: current_content,
            }
        }
    }

    /// Apply a single hunk to content
    fn apply_single_hunk(content: &str, hunk: &ParsedHunk) -> Result<String, HunkApplyError> {
        // Try direct match first
        if let Some(result) = Self::try_direct_apply(content, hunk) {
            return Ok(result);
        }

        // Try normalized match
        if let Some(result) = Self::try_normalized_apply(content, hunk) {
            return Ok(result);
        }

        // No match found
        Err(HunkApplyError::NoMatch {
            suggestion: format!(
                "Could not find context lines. Expected to find: {}",
                hunk.old_lines.join("\n")
            ),
        })
    }

    /// Try to apply hunk with exact string matching
    fn try_direct_apply(content: &str, hunk: &ParsedHunk) -> Option<String> {
        let old_text = hunk.old_lines.join("\n");
        let new_text = hunk.new_lines.join("\n");

        let matches: Vec<usize> = content.match_indices(&old_text).map(|(i, _)| i).collect();

        if matches.len() == 1 {
            // Exactly one match - apply the replacement
            let result = content.replacen(&old_text, &new_text, 1);
            Some(result)
        } else {
            None
        }
    }

    /// Try to apply hunk with normalized whitespace matching
    fn try_normalized_apply(content: &str, hunk: &ParsedHunk) -> Option<String> {
        let old_text = hunk.old_lines.join("\n");
        let new_text = hunk.new_lines.join("\n");

        // Normalize by trimming each line
        let normalized_old: Vec<&str> = old_text.lines().map(|l| l.trim()).collect();
        let normalized_new: Vec<&str> = new_text.lines().map(|l| l.trim()).collect();

        let content_lines: Vec<&str> = content.lines().collect();
        let mut matches = Vec::new();

        // Search for matching sequences in content
        for i in 0..=content_lines.len().saturating_sub(normalized_old.len()) {
            let window = &content_lines[i..i + normalized_old.len()];
            let normalized_window: Vec<&str> = window.iter().map(|l| l.trim()).collect();

            if normalized_window == normalized_old {
                matches.push(i);
            }
        }

        if matches.len() == 1 {
            // Exactly one match - apply the replacement
            let match_idx = matches[0];
            let mut result_lines: Vec<String> = Vec::new();

            // Add lines before match
            result_lines.extend(content_lines[..match_idx].iter().map(|s| s.to_string()));

            // Add new lines (preserving original indentation of first matched line)
            if let Some(first_line) = content_lines.get(match_idx) {
                let indent = Self::get_indentation(first_line);
                for new_line in &normalized_new {
                    if new_line.is_empty() {
                        result_lines.push(String::new());
                    } else {
                        result_lines.push(format!("{}{}", indent, new_line));
                    }
                }
            }

            // Add lines after match
            let after_match = match_idx + normalized_old.len();
            if after_match < content_lines.len() {
                result_lines.extend(content_lines[after_match..].iter().map(|s| s.to_string()));
            }

            Some(result_lines.join("\n"))
        } else {
            None
        }
    }

    /// Extract indentation from a line
    fn get_indentation(line: &str) -> String {
        line.chars()
            .take_while(|c| c.is_whitespace())
            .collect::<String>()
    }
}

/// Internal error type for hunk application
#[derive(Debug)]
enum HunkApplyError {
    NoMatch {
        suggestion: String,
    },
    #[allow(dead_code)]
    MultipleMatches {
        count: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::udiff::parser::ParsedHunk;

    #[test]
    fn test_apply_simple_hunk() {
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let hunk = ParsedHunk {
            context_anchor: None,
            old_lines: vec![
                "fn main() {".to_string(),
                "    println!(\"Hello\");".to_string(),
                "}".to_string(),
            ],
            new_lines: vec![
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
                "}".to_string(),
            ],
        };

        let result = UdiffApplier::apply_hunks(content, &[hunk]);
        match result {
            ApplyResult::Success { new_content } => {
                assert_eq!(
                    new_content,
                    "fn main() {\n    println!(\"Hello, world!\");\n}"
                );
            }
            _ => panic!("Expected Success, got {:?}", result),
        }
    }

    #[test]
    fn test_apply_multiple_hunks() {
        let content = "fn first() {\n    let x = 1;\n}\nfn second() {\n    let y = 3;\n}";
        let hunks = vec![
            ParsedHunk {
                context_anchor: None,
                old_lines: vec![
                    "fn first() {".to_string(),
                    "    let x = 1;".to_string(),
                    "}".to_string(),
                ],
                new_lines: vec![
                    "fn first() {".to_string(),
                    "    let x = 2;".to_string(),
                    "}".to_string(),
                ],
            },
            ParsedHunk {
                context_anchor: None,
                old_lines: vec![
                    "fn second() {".to_string(),
                    "    let y = 3;".to_string(),
                    "}".to_string(),
                ],
                new_lines: vec![
                    "fn second() {".to_string(),
                    "    let y = 4;".to_string(),
                    "}".to_string(),
                ],
            },
        ];

        let result = UdiffApplier::apply_hunks(content, &hunks);
        match result {
            ApplyResult::Success { new_content } => {
                assert!(new_content.contains("let x = 2;"));
                assert!(new_content.contains("let y = 4;"));
            }
            _ => panic!("Expected Success, got {:?}", result),
        }
    }

    #[test]
    fn test_apply_no_match() {
        let content = "fn main() {\n    println!(\"Different\");\n}";
        let hunk = ParsedHunk {
            context_anchor: None,
            old_lines: vec![
                "fn main() {".to_string(),
                "    println!(\"Hello\");".to_string(),
            ],
            new_lines: vec![
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
            ],
        };

        let result = UdiffApplier::apply_hunks(content, &[hunk]);
        match result {
            ApplyResult::NoMatch { hunk_idx, .. } => {
                assert_eq!(hunk_idx, 0);
            }
            _ => panic!("Expected NoMatch, got {:?}", result),
        }
    }

    #[test]
    fn test_apply_normalized_whitespace() {
        let content = "fn main() {\n  println!(\"Hello\");\n}"; // 2 spaces indent
        let hunk = ParsedHunk {
            context_anchor: None,
            old_lines: vec![
                "fn main() {".to_string(),
                "println!(\"Hello\");".to_string(), // No indent in hunk
                "}".to_string(),
            ],
            new_lines: vec![
                "fn main() {".to_string(),
                "println!(\"Goodbye\");".to_string(),
                "}".to_string(),
            ],
        };

        let result = UdiffApplier::apply_hunks(content, &[hunk]);
        match result {
            ApplyResult::Success { new_content } => {
                // Should preserve original indentation
                assert!(new_content.contains("  println!(\"Goodbye\");"));
            }
            _ => panic!(
                "Expected Success with normalized matching, got {:?}",
                result
            ),
        }
    }

    #[test]
    fn test_apply_partial_success() {
        let content = "fn first() {\n    let x = 1;\n}\nfn second() {\n    let y = 3;\n}";
        let hunks = vec![
            ParsedHunk {
                context_anchor: None,
                old_lines: vec![
                    "fn first() {".to_string(),
                    "    let x = 1;".to_string(),
                    "}".to_string(),
                ],
                new_lines: vec![
                    "fn first() {".to_string(),
                    "    let x = 2;".to_string(),
                    "}".to_string(),
                ],
            },
            ParsedHunk {
                context_anchor: None,
                old_lines: vec!["nonexistent".to_string()],
                new_lines: vec!["replacement".to_string()],
            },
        ];

        let result = UdiffApplier::apply_hunks(content, &hunks);
        match result {
            ApplyResult::PartialSuccess {
                applied,
                failed,
                new_content,
            } => {
                assert_eq!(applied, vec![0]);
                assert_eq!(failed.len(), 1);
                assert!(new_content.contains("let x = 2;"));
            }
            _ => panic!("Expected PartialSuccess, got {:?}", result),
        }
    }
}
