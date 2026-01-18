//! Agent Skills discovery, parsing, and matching for Qbit.
//!
//! This crate implements the [agentskills.io](https://agentskills.io) specification
//! for discovering and loading skill-based extensions that provide specialized
//! instructions to the AI agent.
//!
//! # Skill Discovery
//!
//! Skills are discovered from two locations:
//! - Global: `~/.qbit/skills/<skill-name>/SKILL.md`
//! - Local: `<project>/.qbit/skills/<skill-name>/SKILL.md`
//!
//! Local skills override global skills with the same name.
//!
//! # Usage
//!
//! ```ignore
//! use qbit_skills::{discover_skills, SkillMatcher, load_skill_body};
//!
//! // Discover all available skills
//! let skills = discover_skills(Some("/path/to/project"));
//!
//! // Convert to metadata for matching
//! let metadata: Vec<SkillMetadata> = skills.into_iter().map(Into::into).collect();
//!
//! // Match against user prompt
//! let matcher = SkillMatcher::default();
//! let matches = matcher.match_skills("help me with git commits", &metadata);
//!
//! // Load full body for matched skills
//! for (skill, score, reason) in matches {
//!     let body = load_skill_body(&skill.path)?;
//!     // Inject body into system prompt...
//! }
//! ```

mod discovery;
mod matcher;
mod parser;
mod types;

pub use discovery::{discover_skills, list_skill_files, read_skill_file};
pub use matcher::{extract_keywords, SkillMatcher};
pub use parser::{load_skill_body, load_skill_content, parse_skill_md, validate_skill_name};
pub use types::{MatchedSkill, SkillFileInfo, SkillFrontmatter, SkillInfo, SkillMetadata};

/// Errors that can occur during skill operations.
#[derive(Debug, thiserror::Error)]
pub enum SkillsError {
    /// I/O error reading skill files
    #[error("I/O error: {0}")]
    IoError(String),

    /// Security error (e.g., path traversal)
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Parse error for SKILL.md
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_lifecycle() {
        // Create a test skill in memory
        let content = r#"---
name: test-skill
description: A test skill for unit testing
---

You are a test assistant.
"#;

        // Parse the skill
        let (frontmatter, body) = parse_skill_md(content).expect("Should parse valid SKILL.md");
        assert_eq!(frontmatter.name, "test-skill");
        assert!(body.contains("test assistant"));

        // Validate the name
        assert!(validate_skill_name(&frontmatter.name));

        // Create skill info
        let info = SkillInfo {
            name: frontmatter.name,
            path: "/test/path".to_string(),
            source: "test".to_string(),
            description: frontmatter.description,
            license: frontmatter.license,
            compatibility: frontmatter.compatibility,
            metadata: frontmatter.metadata,
            allowed_tools: None,
            has_scripts: false,
            has_references: false,
            has_assets: false,
        };

        // Convert to metadata
        let metadata: SkillMetadata = info.into();
        assert_eq!(metadata.name, "test-skill");
        assert!(!metadata.keywords.is_empty());

        // Match against prompt
        let matcher = SkillMatcher::default();
        let matches = matcher.match_skills("use test-skill", &[metadata]);
        assert!(!matches.is_empty());
    }
}
