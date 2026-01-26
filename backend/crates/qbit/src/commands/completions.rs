//! Tab completion support for path navigation.
//!
//! This module provides the `list_path_completions` command that returns
//! file/directory completions for a given partial path, enabling tab completion
//! in the terminal input.

use crate::error::Result;
use crate::state::AppState;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::State;

/// Type of filesystem entry for path completions.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PathEntryType {
    File,
    Directory,
    Symlink,
}

/// A single path completion suggestion.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PathCompletion {
    /// Display name (e.g., "Documents/" for directories)
    pub name: String,
    /// Text to insert when this completion is selected
    pub insert_text: String,
    /// Type of filesystem entry
    pub entry_type: PathEntryType,
    /// Fuzzy match score (higher = better match)
    pub score: u32,
    /// Indices of matched characters for highlighting
    pub match_indices: Vec<usize>,
}

/// Response wrapper containing completions and total count.
#[derive(Debug, Clone, Serialize)]
pub struct PathCompletionResponse {
    /// The completions (limited by the limit parameter)
    pub completions: Vec<PathCompletion>,
    /// Total number of matches before limit was applied
    pub total_count: usize,
}

/// Default number of completions to return if no limit is specified.
const DEFAULT_LIMIT: usize = 20;

/// List path completions for a partial path input.
///
/// This command supports:
/// - Empty input (lists current directory)
/// - Tilde expansion (`~/` -> home directory)
/// - Absolute paths (`/`)
/// - Relative paths (`./`, `../`)
/// - Fuzzy matching with scoring and match highlighting
///
/// # Arguments
/// * `state` - Application state containing PTY manager
/// * `session_id` - PTY session ID (used to get working directory)
/// * `partial_path` - The partial path to complete
/// * `limit` - Maximum number of completions to return (default: 20)
///
/// # Returns
/// A `PathCompletionResponse` containing completions and total count.
#[tauri::command]
pub async fn list_path_completions(
    state: State<'_, AppState>,
    session_id: String,
    partial_path: String,
    limit: Option<usize>,
) -> Result<PathCompletionResponse> {
    // Get working directory from PTY session
    let session = state.pty_manager.get_session(&session_id)?;
    let working_dir = PathBuf::from(&session.working_directory);

    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let response = compute_path_completions(&partial_path, &working_dir, limit);

    Ok(response)
}

/// Compute path completions for a partial path.
///
/// This is the core completion logic, separated from the Tauri command for easier testing.
pub fn compute_path_completions(
    partial_path: &str,
    working_dir: &Path,
    limit: usize,
) -> PathCompletionResponse {
    let (search_dir, prefix) = parse_path_input(partial_path, working_dir);

    // Read directory entries
    let entries = match std::fs::read_dir(&search_dir) {
        Ok(entries) => entries,
        Err(_) => {
            return PathCompletionResponse {
                completions: Vec::new(),
                total_count: 0,
            }
        }
    };

    // Check if we should include hidden files
    let show_hidden = prefix.starts_with('.');

    // Collect raw entries first
    let raw_entries: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy().to_string();

            // Skip hidden files unless prefix starts with '.'
            if name.starts_with('.') && !show_hidden {
                return None;
            }

            // Determine entry type
            let metadata = entry.metadata().ok()?;
            let file_type = entry.file_type().ok()?;

            let entry_type = if file_type.is_symlink() {
                PathEntryType::Symlink
            } else if metadata.is_dir() {
                PathEntryType::Directory
            } else {
                PathEntryType::File
            };

            Some((name, entry_type))
        })
        .collect();

    // If no prefix, return all entries with score=0 (no fuzzy matching needed)
    let mut completions: Vec<PathCompletion> = if prefix.is_empty() {
        raw_entries
            .into_iter()
            .map(|(name, entry_type)| {
                let (display_name, insert_text) =
                    build_completion_text(&name, &entry_type, partial_path, &prefix);
                PathCompletion {
                    name: display_name,
                    insert_text,
                    entry_type,
                    score: 0,
                    match_indices: Vec::new(),
                }
            })
            .collect()
    } else {
        // Use fuzzy matching with nucleo
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pattern = Pattern::parse(&prefix, CaseMatching::Smart, Normalization::Smart);

        raw_entries
            .into_iter()
            .filter_map(|(name, entry_type)| {
                let mut indices = Vec::new();
                let mut haystack_buf = Vec::new();
                let haystack = Utf32Str::new(&name, &mut haystack_buf);

                let score = pattern.indices(haystack.slice(..), &mut matcher, &mut indices)?;

                let (display_name, insert_text) =
                    build_completion_text(&name, &entry_type, partial_path, &prefix);

                Some(PathCompletion {
                    name: display_name,
                    insert_text,
                    entry_type,
                    score,
                    match_indices: indices.iter().map(|&i| i as usize).collect(),
                })
            })
            .collect()
    };

    let total_count = completions.len();

    // Sort: by score descending, then directories first, then alphabetically by name
    completions.sort_by(|a, b| {
        // Sort by score descending first (higher score = better match)
        b.score
            .cmp(&a.score)
            .then_with(|| {
                // Then directories first
                let a_is_dir = matches!(a.entry_type, PathEntryType::Directory);
                let b_is_dir = matches!(b.entry_type, PathEntryType::Directory);
                b_is_dir.cmp(&a_is_dir)
            })
            .then_with(|| {
                // Then alphabetically
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            })
    });

    // Apply limit
    completions.truncate(limit);

    PathCompletionResponse {
        completions,
        total_count,
    }
}

/// Parse the partial path input and return (search_directory, prefix_to_match).
fn parse_path_input(partial_path: &str, working_dir: &Path) -> (PathBuf, String) {
    if partial_path.is_empty() {
        // Empty input: list current directory
        return (working_dir.to_path_buf(), String::new());
    }

    // Expand tilde
    let expanded = expand_tilde(partial_path);
    let path = Path::new(&expanded);

    if expanded.ends_with('/') || expanded.ends_with(std::path::MAIN_SEPARATOR) {
        // Path ends with separator: search inside this directory
        let search_dir = if path.is_absolute() {
            path.to_path_buf()
        } else {
            working_dir.join(path)
        };
        (search_dir, String::new())
    } else if let Some(parent) = path.parent() {
        // Path has components: search in parent, match against file name
        let search_dir = if parent.as_os_str().is_empty() {
            if path.is_absolute() {
                PathBuf::from("/")
            } else {
                working_dir.to_path_buf()
            }
        } else if path.is_absolute() || expanded.starts_with('/') {
            parent.to_path_buf()
        } else {
            working_dir.join(parent)
        };

        // Note: path.file_name() returns None for "." and ".." special paths.
        // In that case, treat the entire expanded string as the prefix to match
        // hidden files (e.g., "." matches ".hidden", ".." matches "..foo").
        let prefix = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| expanded.clone());

        (search_dir, prefix)
    } else {
        // Just a prefix (e.g., "Doc")
        (working_dir.to_path_buf(), expanded)
    }
}

/// Expand tilde to home directory.
fn expand_tilde(path: &str) -> String {
    if path == "~" {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        dirs::home_dir()
            .map(|home| home.join(rest).to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string())
    } else {
        path.to_string()
    }
}

/// Build the display name and insert text for a completion.
fn build_completion_text(
    name: &str,
    entry_type: &PathEntryType,
    original_input: &str,
    _prefix: &str,
) -> (String, String) {
    // Display name: append "/" for directories
    let display_name = match entry_type {
        PathEntryType::Directory => format!("{}/", name),
        _ => name.to_string(),
    };

    // Insert text: replace the last component of the original input with the full name
    let insert_text = if original_input.is_empty() {
        display_name.clone()
    } else if original_input.ends_with('/') || original_input.ends_with(std::path::MAIN_SEPARATOR) {
        format!("{}{}", original_input, display_name)
    } else if let Some(last_sep_pos) = original_input.rfind(['/', std::path::MAIN_SEPARATOR]) {
        format!("{}{}", &original_input[..=last_sep_pos], display_name)
    } else {
        display_name.clone()
    };

    (display_name, insert_text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    /// Helper to create a test directory structure.
    fn setup_test_dir() -> TempDir {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create directories
        fs::create_dir(root.join("Documents")).unwrap();
        fs::create_dir(root.join("Downloads")).unwrap();
        fs::create_dir(root.join("Desktop")).unwrap();
        fs::create_dir(root.join(".hidden_dir")).unwrap();

        // Create files
        File::create(root.join("file.txt")).unwrap();
        File::create(root.join("data.json")).unwrap();
        File::create(root.join(".hidden_file")).unwrap();

        // Create nested structure
        fs::create_dir_all(root.join("Documents/work")).unwrap();
        File::create(root.join("Documents/notes.md")).unwrap();

        temp
    }

    mod path_parsing {
        use super::*;

        #[test]
        fn empty_input_returns_working_dir() {
            let working_dir = PathBuf::from("/home/user");
            let (search_dir, prefix) = parse_path_input("", &working_dir);

            assert_eq!(search_dir, PathBuf::from("/home/user"));
            assert_eq!(prefix, "");
        }

        #[test]
        fn tilde_expands_to_home() {
            let expanded = expand_tilde("~/Documents");
            let home = dirs::home_dir().unwrap();
            let expected = home.join("Documents").to_string_lossy().to_string();

            assert_eq!(expanded, expected);
        }

        #[test]
        fn tilde_alone_expands_to_home() {
            let expanded = expand_tilde("~");
            let home = dirs::home_dir().unwrap();

            assert_eq!(expanded, home.to_string_lossy().to_string());
        }

        #[test]
        fn absolute_path_is_preserved() {
            let working_dir = PathBuf::from("/home/user");
            let (search_dir, prefix) = parse_path_input("/usr/loc", &working_dir);

            assert_eq!(search_dir, PathBuf::from("/usr"));
            assert_eq!(prefix, "loc");
        }

        #[test]
        fn relative_path_is_joined_with_working_dir() {
            let working_dir = PathBuf::from("/home/user");
            let (search_dir, prefix) = parse_path_input("Documents/wo", &working_dir);

            assert_eq!(search_dir, PathBuf::from("/home/user/Documents"));
            assert_eq!(prefix, "wo");
        }

        #[test]
        fn path_ending_with_slash_searches_inside() {
            let working_dir = PathBuf::from("/home/user");
            let (search_dir, prefix) = parse_path_input("Documents/", &working_dir);

            assert_eq!(search_dir, PathBuf::from("/home/user/Documents"));
            assert_eq!(prefix, "");
        }

        #[test]
        fn simple_prefix_searches_current_dir() {
            let working_dir = PathBuf::from("/home/user");
            let (search_dir, prefix) = parse_path_input("Doc", &working_dir);

            assert_eq!(search_dir, PathBuf::from("/home/user"));
            assert_eq!(prefix, "Doc");
        }
    }

    mod filtering {
        use super::*;

        #[test]
        fn hidden_files_excluded_by_default() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 100);

            // Should not contain hidden files/dirs
            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(!names.contains(&".hidden_dir/"));
            assert!(!names.contains(&".hidden_file"));
        }

        #[test]
        fn hidden_files_included_when_prefix_starts_with_dot() {
            let temp = setup_test_dir();
            let response = compute_path_completions(".", temp.path(), 100);

            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(names.contains(&".hidden_dir/"));
            assert!(names.contains(&".hidden_file"));
        }

        #[test]
        fn fuzzy_matching_filters_results() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Do", temp.path(), 100);

            // Should match Documents, Downloads (both start with "Do")
            // Note: With fuzzy matching, Desktop could also match (D...o) but with lower score
            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(names.contains(&"Documents/"));
            assert!(names.contains(&"Downloads/"));
            // Documents and Downloads should rank higher than Desktop due to closer character positions
            if names.contains(&"Desktop/") {
                let docs_idx = names.iter().position(|n| *n == "Documents/").unwrap();
                let desk_idx = names.iter().position(|n| *n == "Desktop/").unwrap();
                assert!(
                    docs_idx < desk_idx,
                    "Documents should rank higher than Desktop for 'Do' query"
                );
            }
        }

        #[test]
        #[cfg(target_os = "macos")]
        fn case_insensitive_matching_on_macos() {
            let temp = setup_test_dir();
            let response = compute_path_completions("do", temp.path(), 100);

            // On macOS, should match Documents/Downloads even with lowercase prefix
            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(names.contains(&"Documents/"));
            assert!(names.contains(&"Downloads/"));
        }
    }

    mod sorting {
        use super::*;

        #[test]
        fn directories_come_first() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 100);

            // Find first file and first directory
            let first_file_idx = response
                .completions
                .iter()
                .position(|c| matches!(c.entry_type, PathEntryType::File));
            let last_dir_idx = response
                .completions
                .iter()
                .rposition(|c| matches!(c.entry_type, PathEntryType::Directory));

            if let (Some(file_idx), Some(dir_idx)) = (first_file_idx, last_dir_idx) {
                assert!(
                    dir_idx < file_idx,
                    "All directories should come before files"
                );
            }
        }

        #[test]
        fn alphabetical_within_type() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 100);

            // Get just directory names
            let dir_names: Vec<&str> = response
                .completions
                .iter()
                .filter(|c| matches!(c.entry_type, PathEntryType::Directory))
                .map(|c| c.name.as_str())
                .collect();

            // Should be alphabetically sorted (case-insensitive)
            let mut sorted = dir_names.clone();
            sorted.sort_by_key(|a| a.to_lowercase());
            assert_eq!(dir_names, sorted);
        }
    }

    mod entry_types {
        use super::*;

        #[test]
        fn directories_have_trailing_slash() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Doc", temp.path(), 100);

            let docs = response
                .completions
                .iter()
                .find(|c| c.name.starts_with("Documents"));
            assert!(docs.is_some());
            assert_eq!(docs.unwrap().name, "Documents/");
            assert_eq!(docs.unwrap().entry_type, PathEntryType::Directory);
        }

        #[test]
        fn files_have_no_trailing_slash() {
            let temp = setup_test_dir();
            let response = compute_path_completions("file", temp.path(), 100);

            let file = response
                .completions
                .iter()
                .find(|c| c.name.starts_with("file"));
            assert!(file.is_some());
            assert_eq!(file.unwrap().name, "file.txt");
            assert_eq!(file.unwrap().entry_type, PathEntryType::File);
        }
    }

    mod insert_text {
        use super::*;

        #[test]
        fn empty_input_inserts_name() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 100);

            let docs = response
                .completions
                .iter()
                .find(|c| c.name == "Documents/")
                .unwrap();
            assert_eq!(docs.insert_text, "Documents/");
        }

        #[test]
        fn prefix_input_inserts_name() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Doc", temp.path(), 100);

            let docs = response
                .completions
                .iter()
                .find(|c| c.name == "Documents/")
                .unwrap();
            assert_eq!(docs.insert_text, "Documents/");
        }

        #[test]
        fn path_with_slash_preserves_prefix() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Documents/", temp.path(), 100);

            let work = response
                .completions
                .iter()
                .find(|c| c.name == "work/")
                .unwrap();
            assert_eq!(work.insert_text, "Documents/work/");
        }

        #[test]
        fn partial_path_replaces_last_component() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Documents/wo", temp.path(), 100);

            let work = response
                .completions
                .iter()
                .find(|c| c.name == "work/")
                .unwrap();
            assert_eq!(work.insert_text, "Documents/work/");
        }
    }

    mod limits {
        use super::*;

        #[test]
        fn respects_limit_parameter() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 2);

            assert!(response.completions.len() <= 2);
            // total_count should reflect the actual number of matches
            assert!(response.total_count >= response.completions.len());
        }

        #[test]
        fn returns_all_if_under_limit() {
            let temp = setup_test_dir();
            // We have: Documents/, Downloads/, Desktop/, file.txt, data.json = 5 visible items
            let response = compute_path_completions("", temp.path(), 100);

            // Should have all non-hidden items
            assert!(response.completions.len() >= 5);
            assert_eq!(response.total_count, response.completions.len());
        }

        #[test]
        fn total_count_reflects_all_matches() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 2);

            // total_count should be the actual number of matches, not limited
            assert!(response.total_count >= 5); // We have at least 5 visible items
            assert_eq!(response.completions.len(), 2); // But only 2 returned due to limit
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn nonexistent_directory_returns_empty() {
            let temp = setup_test_dir();
            let response = compute_path_completions("nonexistent/", temp.path(), 100);

            assert!(response.completions.is_empty());
            assert_eq!(response.total_count, 0);
        }

        #[test]
        fn no_matches_returns_empty() {
            let temp = setup_test_dir();
            let response = compute_path_completions("xyz", temp.path(), 100);

            assert!(response.completions.is_empty());
            assert_eq!(response.total_count, 0);
        }

        #[test]
        fn dot_dot_navigates_up() {
            let temp = setup_test_dir();
            let nested_dir = temp.path().join("Documents");

            // From Documents/, "../Do" should list the temp root
            let response = compute_path_completions("../Do", &nested_dir, 100);

            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(names.contains(&"Documents/"));
            assert!(names.contains(&"Downloads/"));
        }
    }

    mod fuzzy_matching {
        use super::*;

        #[test]
        fn fuzzy_match_returns_score_and_indices() {
            let temp = setup_test_dir();
            let response = compute_path_completions("Doc", temp.path(), 100);

            // Should have matches with scores > 0
            let docs = response.completions.iter().find(|c| c.name == "Documents/");
            assert!(docs.is_some());
            let docs = docs.unwrap();
            assert!(docs.score > 0);
            assert!(!docs.match_indices.is_empty());
        }

        #[test]
        fn fuzzy_match_handles_abbreviations() {
            let temp = setup_test_dir();
            // "Dcmts" should fuzzy match "Documents"
            let response = compute_path_completions("Dcmt", temp.path(), 100);

            let names: Vec<&str> = response
                .completions
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert!(names.contains(&"Documents/"));
        }

        #[test]
        fn empty_prefix_returns_all_with_zero_score() {
            let temp = setup_test_dir();
            let response = compute_path_completions("", temp.path(), 100);

            // All completions should have score 0 and empty match_indices
            for completion in &response.completions {
                assert_eq!(completion.score, 0);
                assert!(completion.match_indices.is_empty());
            }
        }

        #[test]
        fn higher_score_sorted_first() {
            let temp = setup_test_dir();
            let response = compute_path_completions("doc", temp.path(), 100);

            // If there are multiple matches, higher scores should come first
            if response.completions.len() >= 2 {
                for window in response.completions.windows(2) {
                    // Higher score or equal should come first (with dirs before files as tiebreaker)
                    assert!(
                        window[0].score >= window[1].score
                            || (window[0].score == window[1].score
                                && matches!(window[0].entry_type, PathEntryType::Directory))
                    );
                }
            }
        }
    }

    /// Property-based tests for path completion invariants.
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        /// Strategy for generating valid filesystem-safe names.
        fn valid_name_strategy() -> impl Strategy<Value = String> {
            "[a-zA-Z][a-zA-Z0-9_-]{0,15}".prop_map(|s| s)
        }

        /// Strategy for generating a list of directory/file names.
        fn name_list_strategy() -> impl Strategy<Value = Vec<(String, bool)>> {
            prop::collection::vec((valid_name_strategy(), any::<bool>()), 0..10)
        }

        proptest! {
            /// Property: The number of completions never exceeds the limit.
            #[test]
            fn completions_respect_limit(
                limit in 1usize..50,
                names in name_list_strategy(),
            ) {
                let temp = TempDir::new().unwrap();

                // Create the directory structure
                for (name, is_dir) in &names {
                    let path = temp.path().join(name);
                    if *is_dir {
                        let _ = fs::create_dir(&path);
                    } else {
                        let _ = File::create(&path);
                    }
                }

                let response = compute_path_completions("", temp.path(), limit);

                prop_assert!(response.completions.len() <= limit,
                    "Got {} completions but limit was {}", response.completions.len(), limit);
            }

            /// Property: Directories always have trailing slash in name.
            #[test]
            fn directories_always_have_trailing_slash(
                names in name_list_strategy(),
            ) {
                let temp = TempDir::new().unwrap();

                for (name, is_dir) in &names {
                    let path = temp.path().join(name);
                    if *is_dir {
                        let _ = fs::create_dir(&path);
                    } else {
                        let _ = File::create(&path);
                    }
                }

                let response = compute_path_completions("", temp.path(), 100);

                for completion in &response.completions {
                    match completion.entry_type {
                        PathEntryType::Directory => {
                            prop_assert!(completion.name.ends_with('/'),
                                "Directory '{}' should end with /", completion.name);
                        }
                        PathEntryType::File | PathEntryType::Symlink => {
                            prop_assert!(!completion.name.ends_with('/'),
                                "File/symlink '{}' should not end with /", completion.name);
                        }
                    }
                }
            }

            /// Property: Completions are sorted (directories first, then alphabetical when scores are equal).
            #[test]
            fn completions_are_properly_sorted(
                names in name_list_strategy(),
            ) {
                let temp = TempDir::new().unwrap();

                for (name, is_dir) in &names {
                    let path = temp.path().join(name);
                    if *is_dir {
                        let _ = fs::create_dir(&path);
                    } else {
                        let _ = File::create(&path);
                    }
                }

                let response = compute_path_completions("", temp.path(), 100);

                // When no prefix (empty query), all scores are 0, so directories come first
                let mut seen_file = false;
                for completion in &response.completions {
                    if matches!(completion.entry_type, PathEntryType::File | PathEntryType::Symlink) {
                        seen_file = true;
                    } else if seen_file {
                        prop_assert!(false,
                            "Directory '{}' found after file", completion.name);
                    }
                }

                // Check alphabetical within each type (when scores are equal)
                let dirs: Vec<_> = response.completions.iter()
                    .filter(|c| matches!(c.entry_type, PathEntryType::Directory))
                    .collect();
                let files: Vec<_> = response.completions.iter()
                    .filter(|c| !matches!(c.entry_type, PathEntryType::Directory))
                    .collect();

                for window in dirs.windows(2) {
                    prop_assert!(window[0].name.to_lowercase() <= window[1].name.to_lowercase(),
                        "Directories not sorted: '{}' should come before '{}'",
                        window[0].name, window[1].name);
                }

                for window in files.windows(2) {
                    prop_assert!(window[0].name.to_lowercase() <= window[1].name.to_lowercase(),
                        "Files not sorted: '{}' should come before '{}'",
                        window[0].name, window[1].name);
                }
            }

            /// Property: Hidden files only appear when prefix starts with dot.
            #[test]
            fn hidden_files_visibility(
                prefix in prop::option::of("[.a-zA-Z][a-zA-Z0-9]*"),
            ) {
                let temp = TempDir::new().unwrap();

                // Create both hidden and visible items
                fs::create_dir(temp.path().join(".hidden")).unwrap();
                fs::create_dir(temp.path().join("visible")).unwrap();
                File::create(temp.path().join(".hidden_file")).unwrap();
                File::create(temp.path().join("visible_file")).unwrap();

                let prefix_str = prefix.unwrap_or_default();
                let response = compute_path_completions(&prefix_str, temp.path(), 100);

                let has_hidden = response.completions.iter().any(|c|
                    c.name.starts_with('.') || c.name.starts_with("./"));
                let prefix_starts_with_dot = prefix_str.starts_with('.');

                if prefix_starts_with_dot {
                    // Hidden files may or may not be present depending on prefix match
                    // but if they match the prefix, they should be included
                } else {
                    prop_assert!(!has_hidden,
                        "Hidden files should not appear without dot prefix, got: {:?}",
                        response.completions.iter().map(|c| &c.name).collect::<Vec<_>>());
                }
            }

            /// Property: Tilde expansion produces valid paths.
            #[test]
            fn tilde_expansion_produces_valid_path(
                // Avoid starting with / which would make it an absolute path
                suffix in "[a-zA-Z][a-zA-Z0-9_-]{0,15}",
            ) {
                let input = format!("~/{}", suffix);
                let expanded = expand_tilde(&input);

                if let Some(home) = dirs::home_dir() {
                    prop_assert!(expanded.starts_with(&home.to_string_lossy().to_string()),
                        "Expanded path '{}' should start with home dir", expanded);
                }
            }

            /// Property: Insert text is consistent with input patterns.
            #[test]
            fn insert_text_consistency(
                input_has_slash in any::<bool>(),
            ) {
                let temp = TempDir::new().unwrap();
                fs::create_dir(temp.path().join("test_dir")).unwrap();
                File::create(temp.path().join("test_file")).unwrap();

                let input = if input_has_slash { "test_dir/" } else { "" };
                let response = compute_path_completions(input, temp.path(), 100);

                for completion in &response.completions {
                    // insert_text should either:
                    // 1. Be just the name (when input is empty or just a prefix)
                    // 2. Preserve the path structure (when input has slashes)

                    if input_has_slash && !input.is_empty() {
                        // Should preserve the directory prefix
                        prop_assert!(completion.insert_text.contains('/'),
                            "Insert text '{}' should contain slash when input has directory",
                            completion.insert_text);
                    }

                    // Insert text should contain the display name
                    prop_assert!(completion.insert_text.ends_with(&completion.name) ||
                                 completion.insert_text.contains(&completion.name),
                        "Insert text '{}' should contain name '{}'",
                        completion.insert_text, completion.name);
                }
            }

            /// Property: Total count is always >= completions length.
            #[test]
            fn total_count_gte_completions_length(
                limit in 1usize..50,
                names in name_list_strategy(),
            ) {
                let temp = TempDir::new().unwrap();

                for (name, is_dir) in &names {
                    let path = temp.path().join(name);
                    if *is_dir {
                        let _ = fs::create_dir(&path);
                    } else {
                        let _ = File::create(&path);
                    }
                }

                let response = compute_path_completions("", temp.path(), limit);

                prop_assert!(response.total_count >= response.completions.len(),
                    "total_count {} should be >= completions.len() {}",
                    response.total_count, response.completions.len());
            }
        }
    }
}
