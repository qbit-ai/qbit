//! AST-grep based code search and replace for Qbit.
//!
//! This crate provides structural code search using AST patterns.
//! Unlike regex, it understands code structure and can match
//! syntactic patterns like function definitions, if statements, etc.
//!
//! # Examples
//!
//! ```ignore
//! use qbit_ast_grep::{search, SearchResult};
//!
//! // Search for all function definitions in a Rust file
//! let result = search(
//!     Path::new("/path/to/workspace"),
//!     "fn $NAME($$$ARGS)",
//!     Some("src/lib.rs"),
//!     Some("rust"),
//! )?;
//!
//! for m in result.matches {
//!     println!("Found function at {}:{}", m.file, m.line);
//! }
//! ```

pub mod language;
pub mod result;
pub mod tool;

// Re-export tool structs for easy use
pub use tool::{AstGrepReplaceTool, AstGrepTool};

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use ast_grep_language::{LanguageExt, SupportLang};
use walkdir::WalkDir;

pub use language::{detect_language, parse_language};
pub use result::{ReplaceResult, Replacement, SearchMatch, SearchResult};

/// Search for AST patterns in source code.
///
/// # Arguments
///
/// * `workspace` - The workspace root directory
/// * `pattern` - AST pattern to search for (e.g., "fn $NAME($$$ARGS)")
/// * `path` - Optional relative path to search (file or directory). Defaults to "."
/// * `language` - Optional language hint. Auto-detected from file extension if not provided.
///
/// # Returns
///
/// A `SearchResult` containing all matches found.
pub fn search(
    workspace: &Path,
    pattern: &str,
    path: Option<&str>,
    language: Option<&str>,
) -> Result<SearchResult> {
    let target_path = match path {
        Some(p) => workspace.join(p),
        None => workspace.to_path_buf(),
    };

    let lang = language.and_then(parse_language);
    let mut result = SearchResult::new();

    if target_path.is_file() {
        // Search single file
        search_file(&target_path, workspace, pattern, lang, &mut result)?;
        result.files_searched = 1;
    } else if target_path.is_dir() {
        // Search directory recursively
        for entry in WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            // Determine language for this file
            let file_lang = lang.or_else(|| {
                file_path
                    .to_str()
                    .and_then(detect_language)
            });

            if file_lang.is_some() {
                search_file(file_path, workspace, pattern, file_lang, &mut result)?;
                result.files_searched += 1;
            }
        }
    } else {
        anyhow::bail!("Path does not exist: {}", target_path.display());
    }

    Ok(result)
}

/// Search a single file for pattern matches.
fn search_file(
    file_path: &Path,
    workspace: &Path,
    pattern: &str,
    lang: Option<SupportLang>,
    result: &mut SearchResult,
) -> Result<()> {
    let lang = match lang {
        Some(l) => l,
        None => {
            // Try to detect from file path
            match file_path.to_str().and_then(detect_language) {
                Some(l) => l,
                None => return Ok(()), // Skip files with unknown language
            }
        }
    };

    let source = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let relative_path = file_path
        .strip_prefix(workspace)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    // Search the source using ast-grep
    search_source_impl(&source, pattern, lang, &relative_path, result);

    Ok(())
}

/// Search source code string for pattern matches.
fn search_source_impl(
    source: &str,
    pattern: &str,
    lang: SupportLang,
    file_path: &str,
    result: &mut SearchResult,
) {
    let grep = lang.ast_grep(source);

    // Use find_all with pattern string directly - ast-grep handles pattern parsing
    for node_match in grep.root().find_all(pattern) {
        let start = node_match.start_pos();
        let end = node_match.end_pos();
        let start_point = start.byte_point();
        let end_point = end.byte_point();

        result.matches.push(SearchMatch {
            file: file_path.to_string(),
            line: start_point.0 + 1, // Convert to 1-indexed
            column: start_point.1 + 1,
            text: node_match.text().to_string(),
            end_line: end_point.0 + 1,
            end_column: end_point.1 + 1,
        });
    }
}

/// Search source code and return matches.
///
/// This is a convenience function for testing that searches a source string directly.
pub fn search_source(source: &str, pattern: &str, lang: SupportLang) -> Vec<SearchMatch> {
    let mut result = SearchResult::new();
    search_source_impl(source, pattern, lang, "<source>", &mut result);
    result.matches
}

/// Replace AST patterns in source code.
///
/// # Arguments
///
/// * `workspace` - The workspace root directory
/// * `pattern` - AST pattern to match (e.g., "console.log($MSG)")
/// * `replacement` - Replacement template (e.g., "logger.info($MSG)")
/// * `path` - Relative path to modify (file or directory)
/// * `language` - Optional language hint. Auto-detected from file extension if not provided.
///
/// # Returns
///
/// A `ReplaceResult` containing information about the replacements made.
pub fn replace(
    workspace: &Path,
    pattern: &str,
    replacement: &str,
    path: &str,
    language: Option<&str>,
) -> Result<ReplaceResult> {
    let target_path = workspace.join(path);
    let lang = language.and_then(parse_language);
    let mut result = ReplaceResult::new();

    if target_path.is_file() {
        replace_file(&target_path, workspace, pattern, replacement, lang, &mut result)?;
    } else if target_path.is_dir() {
        for entry in WalkDir::new(&target_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            let file_lang = lang.or_else(|| {
                file_path
                    .to_str()
                    .and_then(detect_language)
            });

            if file_lang.is_some() {
                replace_file(file_path, workspace, pattern, replacement, file_lang, &mut result)?;
            }
        }
    } else {
        anyhow::bail!("Path does not exist: {}", target_path.display());
    }

    Ok(result)
}

/// Replace patterns in a single file.
fn replace_file(
    file_path: &Path,
    workspace: &Path,
    pattern: &str,
    replacement: &str,
    lang: Option<SupportLang>,
    result: &mut ReplaceResult,
) -> Result<()> {
    let lang = match lang {
        Some(l) => l,
        None => {
            match file_path.to_str().and_then(detect_language) {
                Some(l) => l,
                None => return Ok(()),
            }
        }
    };

    let source = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let relative_path = file_path
        .strip_prefix(workspace)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let (new_source, changes) = replace_source_impl(&source, pattern, replacement, lang, &relative_path);

    if !changes.is_empty() {
        fs::write(file_path, &new_source)
            .with_context(|| format!("Failed to write file: {}", file_path.display()))?;

        result.files_modified.push(relative_path);
        result.replacements_count += changes.len();
        result.changes.extend(changes);
    }

    Ok(())
}

/// Replace patterns in source code and return the new source and changes.
fn replace_source_impl(
    source: &str,
    pattern: &str,
    replacement: &str,
    lang: SupportLang,
    file_path: &str,
) -> (String, Vec<Replacement>) {
    let grep = lang.ast_grep(source);

    let mut changes = Vec::new();
    let mut new_source = source.to_string();

    // Collect all matches first (we need to apply from end to start to preserve positions)
    let mut matches: Vec<_> = grep
        .root()
        .find_all(pattern)
        .collect();

    // Sort by position (descending) to apply replacements from end to start
    matches.sort_by(|a, b| b.range().start.cmp(&a.range().start));

    for node_match in matches {
        let original = node_match.text().to_string();
        let start = node_match.start_pos();
        let start_point = start.byte_point();
        let range = node_match.range();

        // Generate replacement text by substituting meta-variables
        let replaced = generate_replacement(&node_match, replacement, lang);

        // Apply the replacement
        new_source.replace_range(range.start..range.end, &replaced);

        changes.push(Replacement {
            file: file_path.to_string(),
            line: start_point.0 + 1,
            original,
            replacement: replaced,
        });
    }

    // Reverse changes to match file order
    changes.reverse();

    (new_source, changes)
}

/// Generate replacement text by substituting captured meta-variables.
fn generate_replacement<D: ast_grep_core::Doc>(
    node_match: &ast_grep_core::NodeMatch<D>,
    replacement: &str,
    _lang: SupportLang,
) -> String {
    let env = node_match.get_env();

    // Find all $VAR and $$$VAR patterns in the replacement template
    // and substitute them with captured values
    let mut i = 0;
    let chars: Vec<char> = replacement.chars().collect();
    let mut new_result = String::new();

    while i < chars.len() {
        if chars[i] == '$' {
            // Check for $$$ (multi-capture)
            if i + 2 < chars.len() && chars[i + 1] == '$' && chars[i + 2] == '$' {
                // Find the variable name after $$$
                let start = i + 3;
                let end = find_var_end(&chars, start);
                if end > start {
                    let var_name: String = chars[start..end].iter().collect();
                    // Get multiple matches
                    let nodes = env.get_multiple_matches(&var_name);
                    let text: String = nodes.iter().map(|n| n.text().to_string()).collect::<Vec<_>>().join(", ");
                    new_result.push_str(&text);
                    i = end;
                    continue;
                }
            }
            // Check for $ (single capture)
            let start = i + 1;
            let end = find_var_end(&chars, start);
            if end > start {
                let var_name: String = chars[start..end].iter().collect();
                // Get single match
                if let Some(node) = env.get_match(&var_name) {
                    new_result.push_str(&node.text());
                    i = end;
                    continue;
                }
            }
        }
        new_result.push(chars[i]);
        i += 1;
    }

    new_result
}

/// Find the end of a variable name (alphanumeric + underscore)
fn find_var_end(chars: &[char], start: usize) -> usize {
    let mut end = start;
    while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    end
}

/// Replace patterns in source code and return the result.
///
/// This is a convenience function for testing.
pub fn replace_source(
    source: &str,
    pattern: &str,
    replacement: &str,
    lang: SupportLang,
) -> String {
    let (new_source, _) = replace_source_impl(source, pattern, replacement, lang, "<source>");
    new_source
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_search_rust_function() {
        let source = "fn foo(x: i32) -> i32 { x + 1 }";
        // Pattern needs to include the full function structure
        let results = search_source(source, "fn $NAME($$$ARGS) -> $RET { $$$BODY }", SupportLang::Rust);
        assert_eq!(results.len(), 1);
        assert!(results[0].text.contains("fn foo"));
    }

    #[test]
    fn test_search_multiple_functions() {
        let source = r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn sub(a: i32, b: i32) -> i32 { a - b }
fn mul(a: i32, b: i32) -> i32 { a * b }
"#;
        let results = search_source(source, "fn $NAME($$$ARGS) -> $RET { $$$BODY }", SupportLang::Rust);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_javascript_arrow_function() {
        let source = "const add = (a, b) => a + b;";
        let results = search_source(source, "($$$ARGS) => $BODY", SupportLang::JavaScript);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_python_function() {
        let source = r#"
def greet(name):
    return f'Hello, {name}'

def farewell(name):
    return f'Goodbye, {name}'
"#;
        // Python function definitions - match the return statement pattern
        let results = search_source(source, "return $EXPR", SupportLang::Python);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_replace_rust_function_call() {
        let source = "println!(\"hello\");";
        let result = replace_source(
            source,
            "println!($MSG)",
            "log::info!($MSG)",
            SupportLang::Rust,
        );
        assert_eq!(result, "log::info!(\"hello\");");
    }

    #[test]
    fn test_replace_javascript_console_log() {
        let source = "console.log('hello');";
        let result = replace_source(
            source,
            "console.log($MSG)",
            "logger.info($MSG)",
            SupportLang::JavaScript,
        );
        assert_eq!(result, "logger.info('hello');");
    }

    #[test]
    fn test_replace_multiple_occurrences() {
        let source = r#"
console.log('first');
console.log('second');
console.log('third');
"#;
        let result = replace_source(
            source,
            "console.log($MSG)",
            "logger.info($MSG)",
            SupportLang::JavaScript,
        );
        assert!(result.contains("logger.info('first')"));
        assert!(result.contains("logger.info('second')"));
        assert!(result.contains("logger.info('third')"));
        assert!(!result.contains("console.log"));
    }

    #[test]
    fn test_directory_search() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("lib.rs"), "fn helper() {}").unwrap();

        // Use pattern that matches empty function bodies
        let result = search(tmp.path(), "fn $NAME() {}", None, Some("rust")).unwrap();
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.files_searched, 2);
    }

    #[test]
    fn test_directory_search_with_subdirs() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("src/lib.rs"), "fn helper() {}").unwrap();

        // Use pattern that matches empty function bodies
        let result = search(tmp.path(), "fn $NAME() {}", None, Some("rust")).unwrap();
        assert_eq!(result.matches.len(), 2);
    }

    #[test]
    fn test_directory_replace() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.js"), "console.log('hello');").unwrap();

        let result = replace(
            tmp.path(),
            "console.log($MSG)",
            "logger.info($MSG)",
            "test.js",
            Some("javascript"),
        )
        .unwrap();

        assert_eq!(result.files_modified.len(), 1);
        assert_eq!(result.replacements_count, 1);

        let new_content = fs::read_to_string(tmp.path().join("test.js")).unwrap();
        assert_eq!(new_content, "logger.info('hello');");
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            matches: vec![SearchMatch {
                file: "test.rs".to_string(),
                line: 1,
                column: 1,
                text: "fn foo()".to_string(),
                end_line: 1,
                end_column: 9,
            }],
            files_searched: 1,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test.rs"));
        assert!(json.contains("fn foo()"));
    }
}
