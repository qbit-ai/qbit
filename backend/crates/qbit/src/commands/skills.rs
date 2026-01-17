//! Skill discovery and loading for Agent Skills support.
//!
//! Skills are directory-based extensions following the [agentskills.io](https://agentskills.io) specification.
//! Each skill is a directory containing a `SKILL.md` file with YAML frontmatter.
//!
//! Skills are loaded from two locations (local takes precedence over global):
//! - Global: `~/.qbit/skills/<skill-name>/SKILL.md`
//! - Local: `<project>/.qbit/skills/<skill-name>/SKILL.md`

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// YAML frontmatter parsed from SKILL.md files.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    /// Required: 1-64 chars, lowercase alphanumeric + hyphens
    pub name: String,
    /// Required: 1-1024 chars
    pub description: String,
    /// Optional license identifier
    pub license: Option<String>,
    /// Optional compatibility info (1-500 chars if present)
    pub compatibility: Option<String>,
    /// Optional arbitrary metadata
    pub metadata: Option<HashMap<String, String>>,
    /// Space-delimited tool names that this skill is allowed to use
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
}

/// Information about a discovered skill.
#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    /// Skill name (from frontmatter)
    pub name: String,
    /// Full path to the skill directory
    pub path: String,
    /// Source: "global" or "local"
    pub source: String,
    /// Description from frontmatter
    pub description: String,
    /// Optional license
    pub license: Option<String>,
    /// Optional compatibility info
    pub compatibility: Option<String>,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
    /// Parsed allowed tools (from space-delimited string)
    pub allowed_tools: Option<Vec<String>>,
    /// Whether the skill has a scripts/ subdirectory
    pub has_scripts: bool,
    /// Whether the skill has a references/ subdirectory
    pub has_references: bool,
    /// Whether the skill has an assets/ subdirectory
    pub has_assets: bool,
}

/// Information about a file within a skill directory.
#[derive(Debug, Clone, Serialize)]
pub struct SkillFileInfo {
    /// File name
    pub name: String,
    /// Relative path from skill directory
    pub relative_path: String,
    /// Whether this is a directory
    pub is_directory: bool,
}

/// Validate skill name according to agentskills.io specification.
///
/// Rules:
/// - 1-64 chars
/// - Lowercase alphanumeric + hyphens
/// - No consecutive hyphens
/// - No leading/trailing hyphens
fn validate_skill_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }

    // Check for leading/trailing hyphens
    if name.starts_with('-') || name.ends_with('-') {
        return false;
    }

    // Check for consecutive hyphens
    if name.contains("--") {
        return false;
    }

    // Check all characters are valid
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Parse SKILL.md content into frontmatter and body.
///
/// Returns (frontmatter, body) where frontmatter is the parsed YAML and body is the markdown content.
fn parse_skill_md(content: &str) -> Option<(SkillFrontmatter, String)> {
    // Check for YAML frontmatter delimiters
    if !content.starts_with("---") {
        return None;
    }

    // Find the closing delimiter
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---")?;
    let yaml_content = &after_first[..end_pos];

    // Parse YAML frontmatter
    let frontmatter: SkillFrontmatter = serde_yaml::from_str(yaml_content.trim()).ok()?;

    // Validate frontmatter
    if frontmatter.name.is_empty() || frontmatter.name.len() > 64 {
        return None;
    }
    if frontmatter.description.is_empty() || frontmatter.description.len() > 1024 {
        return None;
    }
    if let Some(ref compat) = frontmatter.compatibility {
        if compat.is_empty() || compat.len() > 500 {
            return None;
        }
    }

    // Extract body (everything after closing delimiter and newline)
    let body_start = 3 + end_pos + 4; // "---" + yaml + "\n---"
    let body = if body_start < content.len() {
        content[body_start..].trim_start_matches('\n').to_string()
    } else {
        String::new()
    };

    Some((frontmatter, body))
}

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

/// List available skills from global (~/.qbit/skills/) and local (.qbit/skills/) directories.
/// Local skills override global skills with the same name.
#[tauri::command]
pub async fn list_skills(working_directory: Option<String>) -> Result<Vec<SkillInfo>> {
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
        let local_dir = PathBuf::from(&wd).join(".qbit").join("skills");
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

    Ok(result)
}

/// Read the full content of a SKILL.md file (frontmatter + body).
#[tauri::command]
pub async fn read_skill(path: String) -> Result<String> {
    let skill_md_path = PathBuf::from(&path).join("SKILL.md");
    let content = fs::read_to_string(&skill_md_path)?;
    Ok(content)
}

/// Read only the body (markdown instructions) from a SKILL.md file.
/// This is what gets sent to the AI as the skill instructions.
#[tauri::command]
pub async fn read_skill_body(path: String) -> Result<String> {
    let skill_md_path = PathBuf::from(&path).join("SKILL.md");
    let content = fs::read_to_string(&skill_md_path)?;

    // Parse and return only the body
    if let Some((_frontmatter, body)) = parse_skill_md(&content) {
        Ok(body)
    } else {
        // If parsing fails, return the whole content as body
        Ok(content)
    }
}

/// List files in a skill's subdirectory (scripts/, references/, or assets/).
#[tauri::command]
pub async fn list_skill_files(skill_path: String, subdir: String) -> Result<Vec<SkillFileInfo>> {
    // Validate subdir is one of the allowed directories
    if !["scripts", "references", "assets"].contains(&subdir.as_str()) {
        return Ok(vec![]);
    }

    let target_dir = PathBuf::from(&skill_path).join(&subdir);
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
#[tauri::command]
pub async fn read_skill_file(skill_path: String, relative_path: String) -> Result<String> {
    let file_path = PathBuf::from(&skill_path).join(&relative_path);

    // Security check: ensure the resolved path is still within the skill directory
    let canonical_skill = PathBuf::from(&skill_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&skill_path));
    let canonical_file = file_path
        .canonicalize()
        .unwrap_or_else(|_| file_path.clone());

    if !canonical_file.starts_with(&canonical_skill) {
        return Err(crate::error::QbitError::Internal(
            "Access denied: path traversal attempted".to_string(),
        ));
    }

    let content = fs::read_to_string(&file_path)?;
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_skill_name_valid() {
        assert!(validate_skill_name("test-skill"));
        assert!(validate_skill_name("my-skill-123"));
        assert!(validate_skill_name("a"));
        assert!(validate_skill_name("abc123"));
    }

    #[test]
    fn test_validate_skill_name_invalid() {
        assert!(!validate_skill_name("")); // Empty
        assert!(!validate_skill_name("-test")); // Leading hyphen
        assert!(!validate_skill_name("test-")); // Trailing hyphen
        assert!(!validate_skill_name("test--skill")); // Consecutive hyphens
        assert!(!validate_skill_name("Test-Skill")); // Uppercase
        assert!(!validate_skill_name("test_skill")); // Underscore
        assert!(!validate_skill_name(&"a".repeat(65))); // Too long
    }

    #[test]
    fn test_parse_skill_md_valid() {
        let content = r#"---
name: test-skill
description: A test skill
---

You are a testing assistant.
"#;
        let result = parse_skill_md(content);
        assert!(result.is_some());
        let (frontmatter, body) = result.unwrap();
        assert_eq!(frontmatter.name, "test-skill");
        assert_eq!(frontmatter.description, "A test skill");
        assert!(body.contains("You are a testing assistant"));
    }

    #[test]
    fn test_parse_skill_md_with_optional_fields() {
        let content = r#"---
name: advanced-skill
description: An advanced skill with all fields
license: MIT
compatibility: Claude 3.5+
allowed-tools: read write bash
metadata:
  author: test
  version: 1.0.0
---

Skill instructions here.
"#;
        let result = parse_skill_md(content);
        assert!(result.is_some());
        let (frontmatter, _body) = result.unwrap();
        assert_eq!(frontmatter.name, "advanced-skill");
        assert_eq!(frontmatter.license, Some("MIT".to_string()));
        assert_eq!(frontmatter.compatibility, Some("Claude 3.5+".to_string()));
        assert_eq!(
            frontmatter.allowed_tools,
            Some("read write bash".to_string())
        );
        assert!(frontmatter.metadata.is_some());
    }

    #[test]
    fn test_parse_skill_md_no_frontmatter() {
        let content = "Just some markdown without frontmatter";
        assert!(parse_skill_md(content).is_none());
    }

    #[test]
    fn test_parse_skill_md_invalid_frontmatter() {
        let content = r#"---
name: ""
description: Missing name
---

Body here.
"#;
        assert!(parse_skill_md(content).is_none());
    }
}
