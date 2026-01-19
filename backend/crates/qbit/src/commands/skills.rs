//! Skill discovery and loading for Agent Skills support.
//!
//! Skills are directory-based extensions following the [agentskills.io](https://agentskills.io) specification.
//! Each skill is a directory containing a `SKILL.md` file with YAML frontmatter.
//!
//! Skills are loaded from two locations (local takes precedence over global):
//! - Global: `~/.qbit/skills/<skill-name>/SKILL.md`
//! - Local: `<project>/.qbit/skills/<skill-name>/SKILL.md`
//!
//! This module provides thin Tauri command wrappers around the `qbit-skills` crate.

use crate::error::Result;

// Re-export types from qbit-skills for use by other modules
pub use qbit_skills::{SkillFileInfo, SkillInfo};

/// List available skills from global (~/.qbit/skills/) and local (.qbit/skills/) directories.
/// Local skills override global skills with the same name.
#[tauri::command]
pub async fn list_skills(working_directory: Option<String>) -> Result<Vec<SkillInfo>> {
    Ok(qbit_skills::discover_skills(working_directory.as_deref()))
}

/// Read the full content of a SKILL.md file (frontmatter + body).
#[tauri::command]
pub async fn read_skill(path: String) -> Result<String> {
    qbit_skills::load_skill_content(&path).map_err(Into::into)
}

/// Read only the body (markdown instructions) from a SKILL.md file.
/// This is what gets sent to the AI as the skill instructions.
#[tauri::command]
pub async fn read_skill_body(path: String) -> Result<String> {
    qbit_skills::load_skill_body(&path).map_err(Into::into)
}

/// List files in a skill's subdirectory (scripts/, references/, or assets/).
#[tauri::command]
pub async fn list_skill_files(skill_path: String, subdir: String) -> Result<Vec<SkillFileInfo>> {
    qbit_skills::list_skill_files(&skill_path, &subdir).map_err(Into::into)
}

/// Read a specific file from within a skill directory.
#[tauri::command]
pub async fn read_skill_file(skill_path: String, relative_path: String) -> Result<String> {
    qbit_skills::read_skill_file(&skill_path, &relative_path).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skill(dir: &std::path::Path, name: &str) {
        let skill_dir = dir.join(name);
        fs::create_dir_all(&skill_dir).unwrap();

        let content = format!(
            r#"---
name: {}
description: Test skill {}
---

Test instructions for {}.
"#,
            name, name, name
        );

        fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    }

    #[tokio::test]
    async fn test_list_skills() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".qbit").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        create_test_skill(&skills_dir, "test-skill");

        let skills = list_skills(Some(temp_dir.path().to_string_lossy().to_string()))
            .await
            .unwrap();

        // Filter to only local skills to avoid interference from global skills
        let local_skills: Vec<_> = skills.iter().filter(|s| s.source == "local").collect();

        assert_eq!(local_skills.len(), 1);
        assert_eq!(local_skills[0].name, "test-skill");
    }

    #[tokio::test]
    async fn test_read_skill_body() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".qbit").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        create_test_skill(&skills_dir, "test-skill");

        let skill_path = skills_dir.join("test-skill");
        let body = read_skill_body(skill_path.to_string_lossy().to_string())
            .await
            .unwrap();

        assert!(body.contains("Test instructions for test-skill"));
    }
}
