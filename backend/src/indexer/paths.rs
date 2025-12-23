//! Index path resolution utilities.
//!
//! Provides functions to compute index directory paths based on the configured
//! storage location (global or local).

use crate::settings::schema::IndexLocation;
use std::path::{Path, PathBuf};

/// Compute the index directory path for a given workspace.
///
/// # Arguments
/// * `workspace_path` - The absolute path to the workspace
/// * `location` - Whether to use global (~/.qbit) or local (.qbit) storage
///
/// # Returns
/// The path where the index should be stored.
pub fn compute_index_dir(workspace_path: &Path, location: IndexLocation) -> PathBuf {
    match location {
        IndexLocation::Global => compute_global_index_dir(workspace_path),
        IndexLocation::Local => compute_local_index_dir(workspace_path),
    }
}

/// Compute the local index directory: <workspace>/.qbit/index
fn compute_local_index_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(".qbit").join("index")
}

/// Compute the global index directory: ~/.qbit/codebases/<codebase-name>/index
///
/// The codebase name is derived from the directory name with a hash suffix
/// to ensure uniqueness across different paths with the same directory name.
fn compute_global_index_dir(workspace_path: &Path) -> PathBuf {
    let codebases_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".qbit")
        .join("codebases");

    let codebase_name = compute_codebase_name(workspace_path);
    codebases_dir.join(codebase_name).join("index")
}

/// Compute a unique codebase name from the workspace path.
///
/// Uses the last path component (directory name) as the base name.
/// Appends a short hash suffix (first 8 chars of path hash) to ensure uniqueness.
fn compute_codebase_name(workspace_path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Get the last component (directory name)
    let dir_name = workspace_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Compute a short hash of the full canonical path for uniqueness
    let canonical = workspace_path
        .canonicalize()
        .unwrap_or_else(|_| workspace_path.to_path_buf());
    let mut hasher = DefaultHasher::new();
    canonical.hash(&mut hasher);
    let hash = hasher.finish();
    let hash_suffix = format!("{:08x}", hash & 0xFFFFFFFF); // First 8 hex chars

    format!("{}-{}", dir_name, hash_suffix)
}

/// Find the existing index directory for a workspace.
///
/// Checks both global and local locations, prioritizing the configured location.
/// Returns the path if an index exists, or None.
pub fn find_existing_index_dir(workspace_path: &Path, preferred: IndexLocation) -> Option<PathBuf> {
    let preferred_path = compute_index_dir(workspace_path, preferred);
    if preferred_path.exists() {
        return Some(preferred_path);
    }

    // Check alternate location for backward compatibility
    let alternate = match preferred {
        IndexLocation::Global => IndexLocation::Local,
        IndexLocation::Local => IndexLocation::Global,
    };
    let alternate_path = compute_index_dir(workspace_path, alternate);
    if alternate_path.exists() {
        return Some(alternate_path);
    }

    None
}

/// Migrate an index from one location to another.
///
/// # Returns
/// * `Ok(Some(new_path))` if migration succeeded
/// * `Ok(None)` if no migration was needed (already at target or no index exists)
/// * `Err(...)` if migration failed
pub fn migrate_index(
    workspace_path: &Path,
    from: IndexLocation,
    to: IndexLocation,
) -> anyhow::Result<Option<PathBuf>> {
    if from == to {
        return Ok(None);
    }

    let source = compute_index_dir(workspace_path, from);
    if !source.exists() {
        return Ok(None);
    }

    let target = compute_index_dir(workspace_path, to);
    if target.exists() {
        anyhow::bail!("Target index directory already exists: {:?}", target);
    }

    // Create parent directories
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Move the index directory
    std::fs::rename(&source, &target)?;

    tracing::info!("Migrated index from {:?} to {:?}", source, target);
    Ok(Some(target))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_local_index_dir() {
        let path = PathBuf::from("/home/user/projects/myapp");
        let result = compute_local_index_dir(&path);
        assert_eq!(
            result,
            PathBuf::from("/home/user/projects/myapp/.qbit/index")
        );
    }

    #[test]
    fn test_compute_codebase_name_format() {
        let path = PathBuf::from("/home/user/projects/myapp");
        let name = compute_codebase_name(&path);

        // Should start with the directory name
        assert!(name.starts_with("myapp-"));
        // Should have 8 hex characters after the dash
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1].len(), 8);
    }

    #[test]
    fn test_compute_codebase_name_uniqueness() {
        let path1 = PathBuf::from("/home/user/projects/myapp");
        let path2 = PathBuf::from("/home/user/work/myapp");

        let name1 = compute_codebase_name(&path1);
        let name2 = compute_codebase_name(&path2);

        // Both start with "myapp-" but have different hash suffixes
        assert!(name1.starts_with("myapp-"));
        assert!(name2.starts_with("myapp-"));
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_compute_index_dir_global() {
        let path = PathBuf::from("/home/user/projects/myapp");
        let result = compute_index_dir(&path, IndexLocation::Global);

        // Should be under ~/.qbit/codebases/<name>/index
        let home = dirs::home_dir().unwrap();
        assert!(result.starts_with(home.join(".qbit").join("codebases")));
        assert!(result.ends_with("index"));
    }

    #[test]
    fn test_compute_index_dir_local() {
        let path = PathBuf::from("/home/user/projects/myapp");
        let result = compute_index_dir(&path, IndexLocation::Local);

        assert_eq!(
            result,
            PathBuf::from("/home/user/projects/myapp/.qbit/index")
        );
    }
}
