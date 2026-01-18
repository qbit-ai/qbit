//! Skill types for the Agent Skills system.
//!
//! These types represent skills discovered from `~/.qbit/skills/` and `<project>/.qbit/skills/`
//! directories following the [agentskills.io](https://agentskills.io) specification.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Full skill information with all metadata.
///
/// This is the complete representation of a skill, including
/// information about optional subdirectories.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Lightweight metadata for matching (no body, pre-computed keywords).
///
/// This is used for efficient skill matching without loading
/// the full skill body until needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Skill name
    pub name: String,
    /// Short description
    pub description: String,
    /// Full path to the skill directory
    pub path: String,
    /// Source: "global" or "local"
    pub source: String,
    /// Allowed tools for this skill
    pub allowed_tools: Option<Vec<String>>,
    /// Pre-computed keywords for matching
    pub keywords: Vec<String>,
}

impl From<SkillInfo> for SkillMetadata {
    fn from(info: SkillInfo) -> Self {
        let keywords = crate::matcher::extract_keywords(&info.name, &info.description);
        Self {
            name: info.name,
            description: info.description,
            path: info.path,
            source: info.source,
            allowed_tools: info.allowed_tools,
            keywords,
        }
    }
}

impl From<&SkillInfo> for SkillMetadata {
    fn from(info: &SkillInfo) -> Self {
        let keywords = crate::matcher::extract_keywords(&info.name, &info.description);
        Self {
            name: info.name.clone(),
            description: info.description.clone(),
            path: info.path.clone(),
            source: info.source.clone(),
            allowed_tools: info.allowed_tools.clone(),
            keywords,
        }
    }
}

/// A skill matched to a user prompt with full body loaded.
///
/// This represents the result of matching a skill to user input,
/// including the full instruction body for injection into the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedSkill {
    /// The skill metadata
    pub metadata: SkillMetadata,
    /// The full skill body (markdown instructions)
    pub body: String,
    /// Match score (0.0 to 1.0)
    pub match_score: f32,
    /// Human-readable reason for the match
    pub match_reason: String,
}

/// Information about a file within a skill directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFileInfo {
    /// File name
    pub name: String,
    /// Relative path from skill directory
    pub relative_path: String,
    /// Whether this is a directory
    pub is_directory: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_info_to_metadata() {
        let info = SkillInfo {
            name: "test-skill".to_string(),
            path: "/path/to/skill".to_string(),
            source: "global".to_string(),
            description: "A test skill for demonstration".to_string(),
            license: Some("MIT".to_string()),
            compatibility: None,
            metadata: None,
            allowed_tools: Some(vec!["read_file".to_string()]),
            has_scripts: false,
            has_references: false,
            has_assets: false,
        };

        let metadata: SkillMetadata = info.into();
        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.source, "global");
        assert!(!metadata.keywords.is_empty());
    }
}
