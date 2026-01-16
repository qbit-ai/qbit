//! Helper functions for finding memory files based on workspace and codebase settings.

use qbit_settings::schema::CodebaseConfig;
use std::path::{Path, PathBuf};

/// Standard memory file names to auto-detect, in priority order.
const AUTO_DETECT_FILES: &[&str] = &["CLAUDE.md", "AGENT.md"];

/// Find the memory file path for a workspace.
///
/// Resolution order:
/// 1. If the workspace matches an indexed codebase with explicit `memory_file`, use that
/// 2. Otherwise, auto-detect CLAUDE.md or AGENT.md in the workspace (CLAUDE.md takes priority)
pub fn find_memory_file_for_workspace(
    workspace_path: &Path,
    codebases: &[CodebaseConfig],
) -> Option<PathBuf> {
    // Helper to expand ~ to home directory
    fn expand_home_dir(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            dirs::home_dir()
                .map(|home| home.join(&path[2..]))
                .unwrap_or_else(|| PathBuf::from(path))
        } else {
            PathBuf::from(path)
        }
    }

    // First, check if workspace matches an indexed codebase with explicit memory_file
    if let Ok(workspace_canonical) = workspace_path.canonicalize() {
        for config in codebases {
            let codebase_path = expand_home_dir(&config.path);
            if let Ok(codebase_canonical) = codebase_path.canonicalize() {
                // Check if workspace is the codebase or a subdirectory
                if workspace_canonical == codebase_canonical
                    || workspace_canonical.starts_with(&codebase_canonical)
                {
                    // Found matching codebase - use explicit memory_file if configured
                    if let Some(ref memory_file) = config.memory_file {
                        return Some(PathBuf::from(memory_file));
                    }
                    // Codebase matched but no explicit memory_file - fall through to auto-detection
                    break;
                }
            }
        }
    }

    // Auto-detect memory files in priority order
    for filename in AUTO_DETECT_FILES {
        let path = workspace_path.join(filename);
        if path.exists() && path.is_file() {
            return Some(PathBuf::from(*filename));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_auto_detect_claude_md() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create CLAUDE.md
        fs::write(workspace.join("CLAUDE.md"), "# Instructions").unwrap();

        let result = find_memory_file_for_workspace(workspace, &[]);
        assert_eq!(result, Some(PathBuf::from("CLAUDE.md")));
    }

    #[test]
    fn test_auto_detect_agent_md() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create AGENT.md (no CLAUDE.md)
        fs::write(workspace.join("AGENT.md"), "# Instructions").unwrap();

        let result = find_memory_file_for_workspace(workspace, &[]);
        assert_eq!(result, Some(PathBuf::from("AGENT.md")));
    }

    #[test]
    fn test_claude_md_takes_priority_over_agent_md() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create both files
        fs::write(workspace.join("CLAUDE.md"), "# Claude").unwrap();
        fs::write(workspace.join("AGENT.md"), "# Agent").unwrap();

        let result = find_memory_file_for_workspace(workspace, &[]);
        assert_eq!(result, Some(PathBuf::from("CLAUDE.md")));
    }

    #[test]
    fn test_explicit_config_takes_priority() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create CLAUDE.md
        fs::write(workspace.join("CLAUDE.md"), "# Claude").unwrap();
        // Create custom file
        fs::write(workspace.join("CUSTOM.md"), "# Custom").unwrap();

        let codebases = vec![CodebaseConfig {
            path: workspace.display().to_string(),
            memory_file: Some("CUSTOM.md".to_string()),
        }];

        let result = find_memory_file_for_workspace(workspace, &codebases);
        assert_eq!(result, Some(PathBuf::from("CUSTOM.md")));
    }

    #[test]
    fn test_no_memory_file_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // No memory files created

        let result = find_memory_file_for_workspace(workspace, &[]);
        assert_eq!(result, None);
    }

    #[test]
    fn test_codebase_without_memory_file_falls_through_to_auto_detect() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path();

        // Create CLAUDE.md
        fs::write(workspace.join("CLAUDE.md"), "# Claude").unwrap();

        // Codebase configured but without memory_file
        let codebases = vec![CodebaseConfig {
            path: workspace.display().to_string(),
            memory_file: None,
        }];

        let result = find_memory_file_for_workspace(workspace, &codebases);
        assert_eq!(result, Some(PathBuf::from("CLAUDE.md")));
    }
}
