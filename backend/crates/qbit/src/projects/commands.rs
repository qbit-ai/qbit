//! Tauri commands for project configuration management.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{
    delete_project as storage_delete, list_projects as storage_list, load_project as storage_load,
    save_project as storage_save, ProjectCommands, ProjectConfig, WorktreeConfig,
};

/// Project form data from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFormData {
    pub name: String,
    pub root_path: String,
    pub worktrees_dir: String,
    pub test_command: String,
    pub lint_command: String,
    pub build_command: String,
    pub start_command: String,
    pub worktree_init_script: String,
}

/// Project data returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectData {
    pub name: String,
    pub root_path: String,
    pub worktrees_dir: Option<String>,
    pub test_command: Option<String>,
    pub lint_command: Option<String>,
    pub build_command: Option<String>,
    pub start_command: Option<String>,
    pub worktree_init_script: Option<String>,
}

impl From<ProjectConfig> for ProjectData {
    fn from(config: ProjectConfig) -> Self {
        Self {
            name: config.name,
            root_path: config.root_path.to_string_lossy().to_string(),
            worktrees_dir: config
                .worktrees_dir
                .map(|p| p.to_string_lossy().to_string()),
            test_command: config.commands.test,
            lint_command: config.commands.lint,
            build_command: config.commands.build,
            start_command: config.commands.start,
            worktree_init_script: config.worktree.init_script,
        }
    }
}

impl From<ProjectFormData> for ProjectConfig {
    fn from(form: ProjectFormData) -> Self {
        let worktrees_dir = if form.worktrees_dir.is_empty() {
            None
        } else {
            Some(PathBuf::from(form.worktrees_dir))
        };

        ProjectConfig {
            name: form.name,
            root_path: PathBuf::from(form.root_path),
            worktrees_dir,
            commands: ProjectCommands {
                test: non_empty(form.test_command),
                lint: non_empty(form.lint_command),
                build: non_empty(form.build_command),
                start: non_empty(form.start_command),
            },
            worktree: WorktreeConfig {
                init_script: non_empty(form.worktree_init_script),
            },
        }
    }
}

/// Convert empty strings to None.
fn non_empty(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Save a new or updated project configuration.
#[tauri::command]
pub async fn save_project(form: ProjectFormData) -> Result<(), String> {
    let config: ProjectConfig = form.into();

    storage_save(&config)
        .await
        .map_err(|e| format!("Failed to save project: {}", e))
}

/// Delete a project configuration by name.
#[tauri::command]
pub async fn delete_project_config(name: String) -> Result<bool, String> {
    storage_delete(&name)
        .await
        .map_err(|e| format!("Failed to delete project: {}", e))
}

/// List all saved project configurations.
#[tauri::command]
pub async fn list_project_configs() -> Result<Vec<ProjectData>, String> {
    let projects = storage_list()
        .await
        .map_err(|e| format!("Failed to list projects: {}", e))?;

    Ok(projects.into_iter().map(ProjectData::from).collect())
}

/// Get a single project configuration by name.
#[tauri::command]
pub async fn get_project_config(name: String) -> Result<Option<ProjectData>, String> {
    let project = storage_load(&name)
        .await
        .map_err(|e| format!("Failed to load project: {}", e))?;

    Ok(project.map(ProjectData::from))
}
