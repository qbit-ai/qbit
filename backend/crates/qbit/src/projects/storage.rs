//! Project storage operations - load, save, delete, list.

use anyhow::{Context, Result};
use std::path::PathBuf;

use super::schema::ProjectConfig;

/// Get the directory where project configs are stored.
pub fn projects_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".qbit")
        .join("projects")
}

/// Convert a project name to a valid filename slug.
///
/// - Converts to lowercase
/// - Replaces spaces and special chars with hyphens
/// - Removes consecutive hyphens
/// - Removes leading/trailing hyphens
pub fn slugify(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Remove consecutive hyphens and trim
    let mut result = String::new();
    let mut last_was_hyphen = true; // Start true to skip leading hyphens
    for c in slug.chars() {
        if c == '-' {
            if !last_was_hyphen {
                result.push(c);
                last_was_hyphen = true;
            }
        } else {
            result.push(c);
            last_was_hyphen = false;
        }
    }

    // Remove trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    // Fallback if empty
    if result.is_empty() {
        result = "project".to_string();
    }

    result
}

/// Get the path to a project's config file.
fn project_path(name: &str) -> PathBuf {
    projects_dir().join(format!("{}.toml", slugify(name)))
}

/// Load all projects from the projects directory.
pub async fn list_projects() -> Result<Vec<ProjectConfig>> {
    let dir = projects_dir();

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();
    let mut entries = tokio::fs::read_dir(&dir)
        .await
        .context("Failed to read projects directory")?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            match load_project_from_path(&path).await {
                Ok(project) => projects.push(project),
                Err(e) => {
                    tracing::warn!("Failed to load project from {:?}: {}", path, e);
                }
            }
        }
    }

    // Sort by name
    projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(projects)
}

/// Load a single project by name.
pub async fn load_project(name: &str) -> Result<Option<ProjectConfig>> {
    let path = project_path(name);

    if !path.exists() {
        return Ok(None);
    }

    let project = load_project_from_path(&path).await?;
    Ok(Some(project))
}

/// Load a project from a specific file path.
async fn load_project_from_path(path: &PathBuf) -> Result<ProjectConfig> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .context("Failed to read project file")?;

    let project: ProjectConfig =
        toml::from_str(&contents).context("Failed to parse project config")?;

    Ok(project)
}

/// Save a project configuration to disk.
///
/// Uses atomic write (temp file + rename) to prevent corruption.
pub async fn save_project(project: &ProjectConfig) -> Result<()> {
    let dir = projects_dir();

    // Ensure directory exists
    tokio::fs::create_dir_all(&dir)
        .await
        .context("Failed to create projects directory")?;

    let path = project_path(&project.name);
    let contents = toml::to_string_pretty(project).context("Failed to serialize project config")?;

    // Atomic write: write to temp file, then rename
    let temp_path = path.with_extension("toml.tmp");
    tokio::fs::write(&temp_path, &contents)
        .await
        .context("Failed to write temp project file")?;

    tokio::fs::rename(&temp_path, &path)
        .await
        .context("Failed to rename temp project file")?;

    tracing::info!("Saved project '{}' to {:?}", project.name, path);
    Ok(())
}

/// Delete a project configuration.
pub async fn delete_project(name: &str) -> Result<bool> {
    let path = project_path(name);

    if !path.exists() {
        return Ok(false);
    }

    tokio::fs::remove_file(&path)
        .await
        .context("Failed to delete project file")?;

    tracing::info!("Deleted project '{}' from {:?}", name, path);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("my-project"), "my-project");
        assert_eq!(slugify("My Project"), "my-project");
        assert_eq!(slugify("my_project"), "my-project");
        assert_eq!(slugify("My  Project!"), "my-project");
        assert_eq!(slugify("  leading spaces  "), "leading-spaces");
        assert_eq!(slugify("UPPERCASE"), "uppercase");
        assert_eq!(slugify("with--multiple---dashes"), "with-multiple-dashes");
        assert_eq!(slugify(""), "project");
        assert_eq!(slugify("---"), "project");
    }
}
