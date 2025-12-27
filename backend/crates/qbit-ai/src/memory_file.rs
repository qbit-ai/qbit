//! Helper functions for finding memory files based on workspace and codebase settings.

use qbit_settings::schema::CodebaseConfig;
use std::path::{Path, PathBuf};

/// Find the memory file path for a workspace by matching against indexed codebases.
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

    // Canonicalize workspace path for comparison
    let workspace_canonical = workspace_path.canonicalize().ok()?;

    // Find matching codebase
    for config in codebases {
        let codebase_path = expand_home_dir(&config.path);
        if let Ok(codebase_canonical) = codebase_path.canonicalize() {
            // Check if workspace is the codebase or a subdirectory
            if workspace_canonical == codebase_canonical
                || workspace_canonical.starts_with(&codebase_canonical)
            {
                // Found matching codebase
                if let Some(ref memory_file) = config.memory_file {
                    // Return just the filename - it will be resolved relative to workspace
                    return Some(PathBuf::from(memory_file));
                }
                // Codebase found but no memory file configured
                return None;
            }
        }
    }

    // No matching codebase found
    None
}
