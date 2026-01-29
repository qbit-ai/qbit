//! Project configuration storage and management.
//!
//! Projects are stored as individual TOML files in `~/.qbit/projects/`.
//! Each project file contains configuration for a single codebase including
//! paths, commands, and worktree initialization scripts.

mod schema;
mod storage;

pub mod commands;

pub use schema::{ProjectCommands, ProjectConfig, WorktreeConfig};
pub use storage::{delete_project, list_projects, load_project, save_project};
