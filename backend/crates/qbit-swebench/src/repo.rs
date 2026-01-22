//! Repository management for SWE-bench.
//!
//! Handles cloning repositories and checking out specific commits.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use git2::{Oid, Repository};
use tracing::{debug, info, warn};

use crate::types::SWEBenchInstance;

/// Local cache directory for git repositories relative to ~/.qbit
const REPOS_CACHE_DIR: &str = "benchmarks/swebench/repos";

/// Manager for SWE-bench repository operations.
pub struct RepoManager {
    /// Path to the cache directory for bare repositories
    cache_dir: PathBuf,
}

impl RepoManager {
    /// Create a new repository manager with default cache location.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let cache_dir = home.join(".qbit").join(REPOS_CACHE_DIR);
        Self::with_cache_dir(cache_dir)
    }

    /// Create a new repository manager with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        Ok(Self { cache_dir })
    }

    /// Get the path to the cached bare repository.
    fn bare_repo_path(&self, repo: &str) -> PathBuf {
        // Replace / with __ for filesystem compatibility
        let safe_name = repo.replace('/', "__");
        self.cache_dir.join(&safe_name)
    }

    /// Get the GitHub URL for a repository.
    fn github_url(repo: &str) -> String {
        format!("https://github.com/{}.git", repo)
    }

    /// Clone or update the bare repository cache.
    pub fn ensure_bare_repo(&self, repo: &str) -> Result<PathBuf> {
        let bare_path = self.bare_repo_path(repo);

        if bare_path.exists() {
            debug!("Using cached bare repository at {}", bare_path.display());

            // Try to fetch updates
            if let Err(e) = self.fetch_updates(&bare_path) {
                warn!("Failed to fetch updates for {}: {}", repo, e);
            }

            return Ok(bare_path);
        }

        eprintln!("        Cloning {} (this may take a while)...", repo);
        let url = Self::github_url(repo);

        // Use system git command for cloning - more reliable across different systems
        // and respects system git configuration properly
        let output = std::process::Command::new("git")
            .args(["clone", "--bare", "--progress", &url])
            .arg(&bare_path)
            .stderr(std::process::Stdio::inherit()) // Show git progress
            .output()
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to clone {}: {}", url, stderr);
        }

        info!("Cloned {} to {}", repo, bare_path.display());
        Ok(bare_path)
    }

    /// Fetch updates for a bare repository.
    fn fetch_updates(&self, bare_path: &Path) -> Result<()> {
        // Use system git command for fetching - more reliable across different systems
        let output = std::process::Command::new("git")
            .current_dir(bare_path)
            .args(["fetch", "--all"])
            .output()
            .context("Failed to execute git fetch")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git fetch failed: {}", stderr);
        }

        debug!("Fetched updates for {}", bare_path.display());
        Ok(())
    }

    /// Setup a workspace for a SWE-bench instance.
    ///
    /// Creates a working copy of the repository at the specified commit.
    pub fn setup_workspace(
        &self,
        instance: &SWEBenchInstance,
        workspace_dir: &Path,
    ) -> Result<PathBuf> {
        // Ensure we have the bare repository
        let bare_path = self.ensure_bare_repo(&instance.repo)?;

        // Create workspace directory
        let repo_workspace = workspace_dir.join("repo");
        std::fs::create_dir_all(&repo_workspace)?;

        // Clone from bare repo to workspace
        debug!(
            "Cloning {} at {} to workspace",
            instance.repo, instance.base_commit
        );

        let repo =
            Repository::clone(bare_path.to_str().unwrap(), &repo_workspace).with_context(|| {
                format!("Failed to clone to workspace: {}", repo_workspace.display())
            })?;

        // Checkout the base commit
        self.checkout_commit(&repo, &instance.base_commit)?;

        info!(
            "Setup workspace for {} at commit {}",
            instance.instance_id,
            &instance.base_commit[..8.min(instance.base_commit.len())]
        );

        Ok(repo_workspace)
    }

    /// Checkout a specific commit in the repository.
    fn checkout_commit(&self, repo: &Repository, commit_hash: &str) -> Result<()> {
        // Parse the commit hash
        let oid = Oid::from_str(commit_hash)
            .with_context(|| format!("Invalid commit hash: {}", commit_hash))?;

        // Find the commit
        let commit = repo
            .find_commit(oid)
            .with_context(|| format!("Commit not found: {}", commit_hash))?;

        // Reset to the commit (hard reset)
        repo.reset(commit.as_object(), git2::ResetType::Hard, None)
            .with_context(|| format!("Failed to checkout commit: {}", commit_hash))?;

        debug!("Checked out commit {}", commit_hash);
        Ok(())
    }

    /// Apply a patch to the repository.
    pub fn apply_patch(&self, repo_path: &Path, patch: &str) -> Result<()> {
        if patch.is_empty() {
            return Ok(());
        }

        // Write patch to a temporary file
        let patch_file = repo_path.join(".swebench_patch.diff");
        std::fs::write(&patch_file, patch)?;

        // Apply using git apply
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(["apply", "--whitespace=nowarn", ".swebench_patch.diff"])
            .output()
            .context("Failed to execute git apply")?;

        // Clean up patch file
        let _ = std::fs::remove_file(&patch_file);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to apply patch: {}", stderr);
        }

        debug!("Applied patch to {}", repo_path.display());
        Ok(())
    }

    /// Get the current commit hash of a repository.
    pub fn current_commit(&self, repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?.target().context("HEAD has no target")?;
        Ok(head.to_string())
    }

    /// Protect test files by making them read-only.
    ///
    /// This prevents the agent from modifying test files, which is forbidden
    /// in SWE-bench evaluation. The agent should only modify source files.
    pub fn protect_test_files(&self, repo_path: &Path) -> Result<usize> {
        use std::os::unix::fs::PermissionsExt;

        let mut protected_count = 0;

        // Common test directory patterns
        let _test_patterns = ["tests/", "test/", "testing/", "**/tests/", "**/test/"];

        // Walk the directory and find test files
        for entry in walkdir::WalkDir::new(repo_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip if not a file
            if !path.is_file() {
                continue;
            }

            // Check if this is a test file
            let rel_path = path.strip_prefix(repo_path).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy();

            let is_test_file = rel_str.contains("/tests/")
                || rel_str.contains("/test/")
                || rel_str.starts_with("tests/")
                || rel_str.starts_with("test/")
                || rel_str.ends_with("_test.py")
                || rel_str.contains("/test_")
                || (rel_str.starts_with("test_") && rel_str.ends_with(".py"));

            if is_test_file {
                // Make file read-only (remove write permission)
                if let Ok(metadata) = path.metadata() {
                    let mut perms = metadata.permissions();
                    let mode = perms.mode();
                    // Remove write bits (owner, group, other)
                    let new_mode = mode & !0o222;
                    perms.set_mode(new_mode);
                    if std::fs::set_permissions(path, perms).is_ok() {
                        protected_count += 1;
                    }
                }
            }
        }

        debug!(
            "Protected {} test files in {}",
            protected_count,
            repo_path.display()
        );
        Ok(protected_count)
    }

    /// Get the list of modified files in the workspace.
    pub fn modified_files(&self, repo_path: &Path) -> Result<Vec<PathBuf>> {
        let repo = Repository::open(repo_path)?;

        let mut files = Vec::new();
        let statuses = repo.statuses(None)?;

        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                files.push(PathBuf::from(path));
            }
        }

        Ok(files)
    }

    /// Get the diff of all modifications in the workspace.
    pub fn workspace_diff(&self, repo_path: &Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .current_dir(repo_path)
            .args(["diff", "HEAD"])
            .output()
            .context("Failed to execute git diff")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git diff failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Clean up a workspace directory.
    pub fn cleanup_workspace(&self, workspace_dir: &Path) -> Result<()> {
        if workspace_dir.exists() {
            std::fs::remove_dir_all(workspace_dir).with_context(|| {
                format!("Failed to remove workspace: {}", workspace_dir.display())
            })?;
        }
        Ok(())
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> Result<CacheStats> {
        let mut repos = Vec::new();
        let mut total_size = 0u64;

        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().replace("__", "/"))
                    .unwrap_or_default();

                let size = dir_size(&path).unwrap_or(0);
                total_size += size;
                repos.push((name, size));
            }
        }

        Ok(CacheStats { repos, total_size })
    }
}

impl Default for RepoManager {
    fn default() -> Self {
        Self::new().expect("Failed to create default RepoManager")
    }
}

/// Statistics about the repository cache.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Cached repositories with their sizes in bytes
    pub repos: Vec<(String, u64)>,
    /// Total cache size in bytes
    pub total_size: u64,
}

impl CacheStats {
    /// Print a summary of the cache.
    pub fn print_summary(&self) {
        println!("SWE-bench Repository Cache");
        println!("==========================");
        println!("Total size: {}", format_size(self.total_size));
        println!();
        println!("Cached repositories:");

        let mut repos = self.repos.clone();
        repos.sort_by(|a, b| b.1.cmp(&a.1));

        for (repo, size) in repos {
            println!("  {}: {}", repo, format_size(size));
        }
    }
}

/// Calculate the size of a directory recursively.
fn dir_size(path: &Path) -> Result<u64> {
    let mut size = 0;
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

/// Format a byte size as a human-readable string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_url() {
        assert_eq!(
            RepoManager::github_url("django/django"),
            "https://github.com/django/django.git"
        );
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
    }
}
