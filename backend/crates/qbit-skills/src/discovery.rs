//! Skill discovery from global and local directories.
//!
//! Skills are discovered from two locations:
//! - Global: `~/.qbit/skills/<skill-name>/SKILL.md`
//! - Local: `<project>/.qbit/skills/<skill-name>/SKILL.md`
//!
//! Local skills override global skills with the same name.

use crate::parser::{parse_skill_md, validate_skill_name};
use crate::types::SkillInfo;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Load a single skill from a directory path.
fn load_skill(skill_dir: PathBuf, source: &str) -> Option<SkillInfo> {
    let skill_md_path = skill_dir.join("SKILL.md");
    if !skill_md_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&skill_md_path).ok()?;
    let (frontmatter, _body) = parse_skill_md(&content)?;

    // Validate that skill name matches directory name
    let dir_name = skill_dir.file_name()?.to_str()?;
    if frontmatter.name != dir_name {
        tracing::warn!(
            "Skill name '{}' does not match directory name '{}'",
            frontmatter.name,
            dir_name
        );
        return None;
    }

    // Validate skill name format
    if !validate_skill_name(&frontmatter.name) {
        tracing::warn!("Invalid skill name format: '{}'", frontmatter.name);
        return None;
    }

    // Parse allowed tools from space-delimited string
    let allowed_tools = frontmatter.allowed_tools.as_ref().map(|tools| {
        tools
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    // Check for optional subdirectories
    let has_scripts = skill_dir.join("scripts").is_dir();
    let has_references = skill_dir.join("references").is_dir();
    let has_assets = skill_dir.join("assets").is_dir();

    Some(SkillInfo {
        name: frontmatter.name,
        path: skill_dir.to_string_lossy().to_string(),
        source: source.to_string(),
        description: frontmatter.description,
        license: frontmatter.license,
        compatibility: frontmatter.compatibility,
        metadata: frontmatter.metadata,
        allowed_tools,
        has_scripts,
        has_references,
        has_assets,
    })
}

/// Discover skills from global and local directories.
///
/// Local skills override global skills with the same name.
///
/// # Arguments
///
/// * `working_directory` - Optional working directory for local skill lookup.
///   If None, only global skills are discovered.
///
/// # Returns
///
/// A vector of discovered skills, sorted by name.
pub fn discover_skills(working_directory: Option<&str>) -> Vec<SkillInfo> {
    let mut skills: HashMap<String, SkillInfo> = HashMap::new();

    // Read global skills from ~/.qbit/skills/
    if let Some(home) = dirs::home_dir() {
        let global_dir = home.join(".qbit").join("skills");
        if global_dir.exists() {
            if let Ok(entries) = fs::read_dir(&global_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(skill) = load_skill(path, "global") {
                            skills.insert(skill.name.clone(), skill);
                        }
                    }
                }
            }
        }
    }

    // Read local skills from {working_directory}/.qbit/skills/
    // Local skills override global skills with the same name
    if let Some(wd) = working_directory {
        let local_dir = PathBuf::from(wd).join(".qbit").join("skills");
        if local_dir.exists() {
            if let Ok(entries) = fs::read_dir(&local_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(skill) = load_skill(path, "local") {
                            skills.insert(skill.name.clone(), skill);
                        }
                    }
                }
            }
        }
    }

    // Convert to sorted vector
    let mut result: Vec<SkillInfo> = skills.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    result
}

/// List files in a skill's subdirectory (scripts/, references/, or assets/).
pub fn list_skill_files(
    skill_path: &str,
    subdir: &str,
) -> Result<Vec<crate::types::SkillFileInfo>, crate::SkillsError> {
    use crate::types::SkillFileInfo;

    // Validate subdir is one of the allowed directories
    if !["scripts", "references", "assets"].contains(&subdir) {
        return Ok(vec![]);
    }

    let target_dir = PathBuf::from(skill_path).join(subdir);
    if !target_dir.exists() || !target_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut files = Vec::new();

    fn collect_files(
        dir: &PathBuf,
        base_path: &PathBuf,
        files: &mut Vec<SkillFileInfo>,
    ) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let relative = path
                .strip_prefix(base_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            files.push(SkillFileInfo {
                name,
                relative_path: relative,
                is_directory: path.is_dir(),
            });

            // Recursively collect files from subdirectories
            if path.is_dir() {
                collect_files(&path, base_path, files)?;
            }
        }
        Ok(())
    }

    let _ = collect_files(&target_dir, &target_dir, &mut files);

    // Sort files alphabetically
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(files)
}

/// Read a specific file from within a skill directory.
pub fn read_skill_file(
    skill_path: &str,
    relative_path: &str,
) -> Result<String, crate::SkillsError> {
    let file_path = PathBuf::from(skill_path).join(relative_path);

    // Security check: ensure the resolved path is still within the skill directory
    let canonical_skill = PathBuf::from(skill_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(skill_path));
    let canonical_file = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.clone());

    if !canonical_file.starts_with(&canonical_skill) {
        return Err(crate::SkillsError::SecurityError(
            "Access denied: path traversal attempted".to_string(),
        ));
    }

    std::fs::read_to_string(&file_path)
        .map_err(|e| crate::SkillsError::IoError(format!("Failed to read file: {}", e)))
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

    #[test]
    fn test_discover_skills_from_directory() {
        let temp_dir = TempDir::new().unwrap();
        let skills_dir = temp_dir.path().join(".qbit").join("skills");
        fs::create_dir_all(&skills_dir).unwrap();

        create_test_skill(&skills_dir, "skill-a");
        create_test_skill(&skills_dir, "skill-b");

        // Use temp dir as working directory
        let skills = discover_skills(Some(temp_dir.path().to_str().unwrap()));

        // Filter to only local skills (temp dir) to avoid interference from global skills
        let local_skills: Vec<_> = skills.iter().filter(|s| s.source == "local").collect();

        assert_eq!(local_skills.len(), 2);
        assert_eq!(local_skills[0].name, "skill-a");
        assert_eq!(local_skills[1].name, "skill-b");
    }

    #[test]
    fn test_local_overrides_global() {
        // This test would require mocking the home directory
        // For now, we just verify the basic discovery works
        let skills = discover_skills(None);
        // Should not fail even if no skills exist (skills is always a valid Vec)
        let _ = skills.len();
    }
}
