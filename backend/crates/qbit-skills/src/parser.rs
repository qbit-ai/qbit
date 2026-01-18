//! SKILL.md parsing and validation.
//!
//! This module handles parsing SKILL.md files according to the
//! [agentskills.io](https://agentskills.io) specification.

use crate::types::SkillFrontmatter;
use crate::SkillsError;
use std::path::Path;

/// Validate skill name according to agentskills.io specification.
///
/// Rules:
/// - 1-64 chars
/// - Lowercase alphanumeric + hyphens
/// - No consecutive hyphens
/// - No leading/trailing hyphens
pub fn validate_skill_name(name: &str) -> bool {
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
/// Returns `Some((frontmatter, body))` where frontmatter is the parsed YAML
/// and body is the markdown content after the frontmatter.
///
/// Returns `None` if the content is not a valid SKILL.md file.
pub fn parse_skill_md(content: &str) -> Option<(SkillFrontmatter, String)> {
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

/// Load only the body (markdown instructions) from a SKILL.md file.
///
/// This is what gets sent to the AI as the skill instructions.
pub fn load_skill_body(skill_path: &str) -> Result<String, SkillsError> {
    let skill_md_path = Path::new(skill_path).join("SKILL.md");
    let content = std::fs::read_to_string(&skill_md_path).map_err(|e| {
        SkillsError::IoError(format!("Failed to read {}: {}", skill_md_path.display(), e))
    })?;

    // Parse and return only the body
    if let Some((_frontmatter, body)) = parse_skill_md(&content) {
        Ok(body)
    } else {
        // If parsing fails, return the whole content as body
        Ok(content)
    }
}

/// Load the full content of a SKILL.md file (frontmatter + body).
pub fn load_skill_content(skill_path: &str) -> Result<String, SkillsError> {
    let skill_md_path = Path::new(skill_path).join("SKILL.md");
    std::fs::read_to_string(&skill_md_path).map_err(|e| {
        SkillsError::IoError(format!("Failed to read {}: {}", skill_md_path.display(), e))
    })
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
