//! Parser for unified diff format from LLM output.

use std::path::PathBuf;

/// A parsed hunk from a unified diff
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedHunk {
    /// Optional context anchor from @@ line (text after @@)
    pub context_anchor: Option<String>,
    /// Lines from the original file (prefixed with - or space)
    pub old_lines: Vec<String>,
    /// Lines for the new file (prefixed with + or space)
    pub new_lines: Vec<String>,
}

/// A parsed diff for a single file
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDiff {
    /// Path to the file being modified
    pub file_path: PathBuf,
    /// Hunks to apply to this file
    pub hunks: Vec<ParsedHunk>,
}

/// Parser for unified diff format
pub struct UdiffParser;

impl UdiffParser {
    /// Parse LLM output for ```diff code blocks and extract diffs
    ///
    /// This function scans for fenced diff blocks in the format:
    /// ```diff
    /// --- a/path/to/file
    /// +++ b/path/to/file
    /// @@ context @@
    ///  unchanged line
    /// -removed line
    /// +added line
    /// ```
    pub fn parse(content: &str) -> Vec<ParsedDiff> {
        let mut diffs = Vec::new();
        let mut in_diff_block = false;
        let mut current_diff_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for diff block start
            if trimmed == "```diff" || trimmed.starts_with("```diff ") {
                in_diff_block = true;
                current_diff_lines.clear();
                continue;
            }

            // Check for diff block end
            if in_diff_block && trimmed == "```" {
                in_diff_block = false;
                if !current_diff_lines.is_empty() {
                    if let Some(diff) = Self::parse_diff_block(&current_diff_lines) {
                        diffs.push(diff);
                    }
                }
                current_diff_lines.clear();
                continue;
            }

            // Collect lines inside diff block
            if in_diff_block {
                current_diff_lines.push(line.to_string());
            }
        }

        diffs
    }

    /// Parse a single diff block
    fn parse_diff_block(lines: &[String]) -> Option<ParsedDiff> {
        let mut file_path: Option<PathBuf> = None;
        let mut hunks = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = &lines[i];

            // Parse file headers
            if line.starts_with("--- ") {
                // Extract path from "--- a/path" or "--- path"
                let path_part = line.strip_prefix("--- ")?.trim();
                let path = if let Some(p) = path_part.strip_prefix("a/") {
                    p
                } else {
                    path_part
                };
                file_path = Some(PathBuf::from(path));
                i += 1;
                continue;
            }

            if line.starts_with("+++ ") {
                // Skip +++ line, we already got the path from ---
                i += 1;
                continue;
            }

            // Parse hunk
            if line.starts_with("@@ ") {
                let context_anchor = Self::extract_context_anchor(line);
                let hunk_lines = Self::collect_hunk_lines(&lines[i + 1..]);
                let hunk = Self::parse_hunk(&hunk_lines, context_anchor);
                hunks.push(hunk);

                // Skip past the hunk lines we just processed
                i += 1 + hunk_lines.len();
                continue;
            }

            i += 1;
        }

        file_path.map(|path| ParsedDiff {
            file_path: path,
            hunks,
        })
    }

    /// Extract context anchor from @@ line
    fn extract_context_anchor(line: &str) -> Option<String> {
        // Format: "@@ -old_start,old_count +new_start,new_count @@ optional context"
        // We want the "optional context" part after the second @@
        if let Some(second_at) = line.rfind("@@") {
            let context = line[second_at + 2..].trim();
            if !context.is_empty() {
                return Some(context.to_string());
            }
        }
        None
    }

    /// Collect all lines belonging to a hunk (until next @@ or end)
    fn collect_hunk_lines(lines: &[String]) -> Vec<String> {
        let mut hunk_lines = Vec::new();

        for line in lines {
            // Stop at next hunk or header
            if line.starts_with("@@") || line.starts_with("---") || line.starts_with("+++") {
                break;
            }

            // Include lines that start with space, -, or +
            if line.starts_with(' ') || line.starts_with('-') || line.starts_with('+') {
                hunk_lines.push(line.clone());
            }
        }

        hunk_lines
    }

    /// Parse hunk lines into old and new line sets
    fn parse_hunk(lines: &[String], context_anchor: Option<String>) -> ParsedHunk {
        let mut old_lines = Vec::new();
        let mut new_lines = Vec::new();

        for line in lines {
            if let Some(content) = line.strip_prefix(' ') {
                // Context line - appears in both old and new
                old_lines.push(content.to_string());
                new_lines.push(content.to_string());
            } else if let Some(content) = line.strip_prefix('-') {
                // Deletion - only in old
                old_lines.push(content.to_string());
            } else if let Some(content) = line.strip_prefix('+') {
                // Addition - only in new
                new_lines.push(content.to_string());
            }
        }

        ParsedHunk {
            context_anchor,
            old_lines,
            new_lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let input = r#"
Here's the change:

```diff
--- a/src/main.rs
+++ b/src/main.rs
@@ main function @@
 fn main() {
-    println!("Hello");
+    println!("Hello, world!");
 }
```
"#;

        let diffs = UdiffParser::parse(input);
        assert_eq!(diffs.len(), 1);

        let diff = &diffs[0];
        assert_eq!(diff.file_path, PathBuf::from("src/main.rs"));
        assert_eq!(diff.hunks.len(), 1);

        let hunk = &diff.hunks[0];
        assert_eq!(hunk.context_anchor, Some("main function".to_string()));
        assert_eq!(hunk.old_lines, vec!["fn main() {", "    println!(\"Hello\");", "}"]);
        assert_eq!(
            hunk.new_lines,
            vec!["fn main() {", "    println!(\"Hello, world!\");", "}"]
        );
    }

    #[test]
    fn test_parse_multiple_hunks() {
        let input = r#"
```diff
--- a/test.rs
+++ b/test.rs
@@ first function @@
 fn first() {
-    let x = 1;
+    let x = 2;
 }
@@ second function @@
 fn second() {
-    let y = 3;
+    let y = 4;
 }
```
"#;

        let diffs = UdiffParser::parse(input);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].hunks.len(), 2);
    }

    #[test]
    fn test_parse_no_context_anchor() {
        let input = r#"
```diff
--- a/file.rs
+++ b/file.rs
@@
 context line
-old line
+new line
```
"#;

        let diffs = UdiffParser::parse(input);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].hunks[0].context_anchor, None);
    }

    #[test]
    fn test_parse_multiple_diff_blocks() {
        let input = r#"
First change:
```diff
--- a/file1.rs
+++ b/file1.rs
@@ @@
-old
+new
```

Second change:
```diff
--- a/file2.rs
+++ b/file2.rs
@@ @@
-old2
+new2
```
"#;

        let diffs = UdiffParser::parse(input);
        assert_eq!(diffs.len(), 2);
        assert_eq!(diffs[0].file_path, PathBuf::from("file1.rs"));
        assert_eq!(diffs[1].file_path, PathBuf::from("file2.rs"));
    }

    #[test]
    fn test_parse_empty_input() {
        let diffs = UdiffParser::parse("");
        assert_eq!(diffs.len(), 0);
    }

    #[test]
    fn test_parse_no_diff_blocks() {
        let input = "Just some regular text without any diff blocks";
        let diffs = UdiffParser::parse(input);
        assert_eq!(diffs.len(), 0);
    }
}
