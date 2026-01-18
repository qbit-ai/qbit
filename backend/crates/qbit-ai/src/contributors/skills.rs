//! Skills prompt contributor.
//!
//! Automatically injects relevant skill instructions based on
//! matched skills in the PromptContext.

use qbit_core::{PromptContext, PromptContributor, PromptPriority, PromptSection};

/// Contributor that generates skill documentation and injects matched skill bodies.
///
/// This contributor:
/// 1. Adds a summary of available skills (if any exist)
/// 2. Injects full instruction bodies for skills matched to the user's prompt
pub struct SkillsPromptContributor;

impl SkillsPromptContributor {
    /// Create a new skills prompt contributor.
    pub fn new() -> Self {
        Self
    }

    /// Generate a summary of available skills.
    fn generate_skills_summary(ctx: &PromptContext) -> Option<String> {
        if ctx.available_skills.is_empty() {
            return None;
        }

        let mut content = String::from("## Available Skills\n\n");
        content.push_str(
            "Skills provide specialized capabilities. They can be invoked via `/skill-name`.\n\n",
        );

        for skill in &ctx.available_skills {
            content.push_str(&format!("- **{}**: {}\n", skill.name, skill.description));
        }

        Some(content)
    }

    /// Generate matched skill bodies for injection.
    fn generate_matched_skills(ctx: &PromptContext) -> Option<String> {
        if ctx.matched_skills.is_empty() {
            return None;
        }

        let mut content = String::from("## Active Skills\n\n");
        content.push_str(
            "The following skills have been automatically activated based on your request:\n\n",
        );

        for skill in &ctx.matched_skills {
            content.push_str(&format!(
                "### {} (score: {:.2})\n\n",
                skill.name, skill.match_score
            ));
            content.push_str(&skill.body);
            content.push_str("\n\n");
        }

        Some(content)
    }
}

impl Default for SkillsPromptContributor {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptContributor for SkillsPromptContributor {
    fn contribute(&self, ctx: &PromptContext) -> Option<Vec<PromptSection>> {
        let mut sections = Vec::new();

        // Add skills summary (lower priority - informational)
        if let Some(summary) = Self::generate_skills_summary(ctx) {
            sections.push(PromptSection::new(
                "skills_summary",
                PromptPriority::Features,
                summary,
            ));
        }

        // Add matched skill bodies (higher priority - active instructions)
        if let Some(matched) = Self::generate_matched_skills(ctx) {
            sections.push(PromptSection::new(
                "skills_matched",
                PromptPriority::Features,
                matched,
            ));
        }

        if sections.is_empty() {
            None
        } else {
            Some(sections)
        }
    }

    fn name(&self) -> &str {
        "SkillsPromptContributor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qbit_core::{PromptMatchedSkill, PromptSkillInfo};

    #[test]
    fn test_no_skills_returns_none() {
        let contributor = SkillsPromptContributor::new();
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4");

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_none());
    }

    #[test]
    fn test_matched_skill_body_preserved_exactly() {
        let contributor = SkillsPromptContributor::new();
        let skill_body = r#"# Code Review Guidelines

You are a code review expert. Follow these rules:

1. Check for security vulnerabilities
2. Review for performance issues
3. Ensure code style consistency

## Important Notes

- Be constructive in feedback
- Suggest improvements, don't just criticize"#;

        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_matched_skills(vec![
            PromptMatchedSkill {
                name: "code-review".to_string(),
                description: "Review code for quality".to_string(),
                body: skill_body.to_string(),
                match_score: 0.75,
                match_reason: "prompt contains skill name".to_string(),
            },
        ]);

        let sections = contributor.contribute(&ctx).unwrap();
        let matched_section = sections.iter().find(|s| s.id == "skills_matched").unwrap();

        // Verify the skill body is included in its entirety
        assert!(matched_section.content.contains("# Code Review Guidelines"));
        assert!(matched_section
            .content
            .contains("1. Check for security vulnerabilities"));
        assert!(matched_section.content.contains("## Important Notes"));
        assert!(matched_section
            .content
            .contains("Suggest improvements, don't just criticize"));
    }

    #[test]
    fn test_multiple_matched_skills_all_injected() {
        let contributor = SkillsPromptContributor::new();
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_matched_skills(vec![
            PromptMatchedSkill {
                name: "skill-one".to_string(),
                description: "First skill".to_string(),
                body: "Body of skill one".to_string(),
                match_score: 0.9,
                match_reason: "name match".to_string(),
            },
            PromptMatchedSkill {
                name: "skill-two".to_string(),
                description: "Second skill".to_string(),
                body: "Body of skill two".to_string(),
                match_score: 0.7,
                match_reason: "keyword match".to_string(),
            },
        ]);

        let sections = contributor.contribute(&ctx).unwrap();
        let matched_section = sections.iter().find(|s| s.id == "skills_matched").unwrap();

        // Both skills should be present
        assert!(matched_section.content.contains("skill-one (score: 0.90)"));
        assert!(matched_section.content.contains("Body of skill one"));
        assert!(matched_section.content.contains("skill-two (score: 0.70)"));
        assert!(matched_section.content.contains("Body of skill two"));
    }

    #[test]
    fn test_available_skills_summary() {
        let contributor = SkillsPromptContributor::new();
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_available_skills(vec![
            PromptSkillInfo {
                name: "git-commit".to_string(),
                description: "Create conventional commits".to_string(),
            },
            PromptSkillInfo {
                name: "code-review".to_string(),
                description: "Review code for issues".to_string(),
            },
        ]);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "skills_summary");

        let content = &sections[0].content;
        assert!(content.contains("## Available Skills"));
        assert!(content.contains("git-commit"));
        assert!(content.contains("code-review"));
    }

    #[test]
    fn test_matched_skills_injection() {
        let contributor = SkillsPromptContributor::new();
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4").with_matched_skills(vec![
            PromptMatchedSkill {
                name: "git-commit".to_string(),
                description: "Create conventional commits".to_string(),
                body: "You are a git commit assistant.\n\nFollow conventional commit format."
                    .to_string(),
                match_score: 0.8,
                match_reason: "prompt contains skill name".to_string(),
            },
        ]);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "skills_matched");

        let content = &sections[0].content;
        assert!(content.contains("## Active Skills"));
        assert!(content.contains("git-commit (score: 0.80)"));
        assert!(content.contains("You are a git commit assistant"));
    }

    #[test]
    fn test_both_summary_and_matched() {
        let contributor = SkillsPromptContributor::new();
        let ctx = PromptContext::new("anthropic", "claude-sonnet-4")
            .with_available_skills(vec![PromptSkillInfo {
                name: "git-commit".to_string(),
                description: "Create conventional commits".to_string(),
            }])
            .with_matched_skills(vec![PromptMatchedSkill {
                name: "git-commit".to_string(),
                description: "Create conventional commits".to_string(),
                body: "Instructions here.".to_string(),
                match_score: 0.8,
                match_reason: "matched".to_string(),
            }]);

        let sections = contributor.contribute(&ctx);
        assert!(sections.is_some());

        let sections = sections.unwrap();
        assert_eq!(sections.len(), 2);
    }
}
