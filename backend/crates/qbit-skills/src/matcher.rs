//! Skill matching algorithm for automatic skill discovery.
//!
//! This module implements keyword-based matching to find relevant skills
//! based on user prompts. The matching is intentionally conservative to
//! minimize false positives.

use crate::types::SkillMetadata;

/// Common English stopwords to filter out during keyword extraction.
const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "he", "i", "in", "is",
    "it", "its", "of", "on", "or", "she", "that", "the", "they", "this", "to", "was", "we", "were",
    "will", "with", "you", "your", "can", "do", "does", "doing", "done", "did", "have", "having",
    "had", "may", "might", "must", "shall", "should", "would", "could", "all", "also", "any",
    "been", "being", "both", "but", "each", "few", "how", "if", "into", "just", "like", "make",
    "many", "more", "most", "my", "no", "not", "now", "only", "other", "our", "out", "over",
    "same", "so", "some", "such", "than", "then", "there", "these", "through", "too", "under",
    "up", "use", "using", "used", "very", "want", "what", "when", "where", "which", "while", "who",
    "why", "about", "after", "again", "against", "before", "between", "during", "help", "me",
    "please", "need", "get",
];

/// Skill matcher configuration.
#[derive(Debug, Clone)]
pub struct SkillMatcher {
    /// Minimum score threshold for a match (0.0 to 1.0)
    pub min_score: f32,
    /// Maximum number of skills to return
    pub max_skills: usize,
}

impl Default for SkillMatcher {
    fn default() -> Self {
        Self {
            min_score: 0.4, // Conservative threshold
            max_skills: 3,
        }
    }
}

impl SkillMatcher {
    /// Create a new skill matcher with custom settings.
    pub fn new(min_score: f32, max_skills: usize) -> Self {
        Self {
            min_score,
            max_skills,
        }
    }

    /// Match skills against a user prompt.
    ///
    /// Returns a vector of (SkillMetadata, score, reason) tuples for skills
    /// that match above the threshold, sorted by score descending.
    pub fn match_skills(
        &self,
        prompt: &str,
        skills: &[SkillMetadata],
    ) -> Vec<(SkillMetadata, f32, String)> {
        let prompt_lower = prompt.to_lowercase();
        let prompt_words: Vec<&str> = prompt_lower.split_whitespace().collect();

        let mut matches: Vec<(SkillMetadata, f32, String)> = skills
            .iter()
            .filter_map(|skill| {
                let (score, reason) = self.calculate_score(&prompt_lower, &prompt_words, skill);
                if score >= self.min_score {
                    Some((skill.clone(), score, reason))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score descending
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Limit to max_skills
        matches.truncate(self.max_skills);

        matches
    }

    /// Calculate the match score for a single skill.
    ///
    /// Returns (score, reason) where:
    /// - score is 0.0 to 1.0
    /// - reason is a human-readable explanation of the match
    fn calculate_score(
        &self,
        prompt_lower: &str,
        prompt_words: &[&str],
        skill: &SkillMetadata,
    ) -> (f32, String) {
        let mut score = 0.0f32;
        let mut reasons = Vec::new();

        // Check if prompt contains the skill name (0.5 score)
        if prompt_lower.contains(&skill.name) {
            score += 0.5;
            reasons.push(format!("prompt contains skill name '{}'", skill.name));
        }

        // Check keyword matches (0.15 per match, up to 0.45)
        let mut keyword_matches = 0;
        for keyword in &skill.keywords {
            if prompt_words.iter().any(|w| w == keyword) {
                keyword_matches += 1;
                if keyword_matches <= 3 {
                    score += 0.15;
                }
            }
        }

        if keyword_matches > 0 {
            reasons.push(format!("{} keyword matches", keyword_matches));
        }

        // Normalize score to max 1.0
        score = score.min(1.0);

        let reason = if reasons.is_empty() {
            "no significant matches".to_string()
        } else {
            reasons.join(", ")
        };

        (score, reason)
    }
}

/// Extract keywords from skill name and description.
///
/// This is used during skill metadata creation to pre-compute
/// keywords for efficient matching.
pub fn extract_keywords(name: &str, description: &str) -> Vec<String> {
    let mut keywords = Vec::new();

    // Split skill name by hyphens
    for part in name.split('-') {
        let part_lower = part.to_lowercase();
        if !part_lower.is_empty() && !is_stopword(&part_lower) {
            keywords.push(part_lower);
        }
    }

    // Extract significant words from description
    for word in description.split_whitespace() {
        let word_lower = word
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();

        if word_lower.len() >= 3 && !is_stopword(&word_lower) && !keywords.contains(&word_lower) {
            keywords.push(word_lower);
        }
    }

    keywords
}

/// Check if a word is a common stopword.
fn is_stopword(word: &str) -> bool {
    STOPWORDS.contains(&word)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_skill(name: &str, description: &str) -> SkillMetadata {
        let keywords = extract_keywords(name, description);
        SkillMetadata {
            name: name.to_string(),
            description: description.to_string(),
            path: format!("/path/to/{}", name),
            source: "test".to_string(),
            allowed_tools: None,
            keywords,
        }
    }

    #[test]
    fn test_extract_keywords() {
        let keywords =
            extract_keywords("git-commit", "Create git commits with conventional format");

        assert!(keywords.contains(&"git".to_string()));
        assert!(keywords.contains(&"commit".to_string()));
        assert!(keywords.contains(&"commits".to_string()));
        assert!(keywords.contains(&"conventional".to_string()));
        assert!(keywords.contains(&"format".to_string()));

        // Should not contain stopwords
        assert!(!keywords.contains(&"with".to_string()));
    }

    #[test]
    fn test_match_by_skill_name() {
        let matcher = SkillMatcher::default();
        let skills = vec![create_test_skill(
            "git-commit",
            "Create commits with conventional format",
        )];

        let matches = matcher.match_skills("use git-commit to create a commit", &skills);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].1 >= 0.5); // Name match should give at least 0.5
        assert!(matches[0].2.contains("skill name"));
    }

    #[test]
    fn test_match_by_keywords() {
        // Use a lower threshold to test keyword-only matching
        // (default threshold is 0.4, but keyword matches give 0.15 each)
        let matcher = SkillMatcher::new(0.25, 3);
        let skills = vec![create_test_skill(
            "code-review",
            "Review code for bugs and improvements",
        )];

        let matches = matcher.match_skills("please review this code", &skills);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].1 >= 0.25); // Keyword matches
        assert!(matches[0].2.contains("keyword"));
    }

    #[test]
    fn test_no_match_below_threshold() {
        let matcher = SkillMatcher::default();
        let skills = vec![create_test_skill(
            "database-migration",
            "Handle database schema migrations",
        )];

        let matches = matcher.match_skills("write some javascript code", &skills);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_max_skills_limit() {
        let matcher = SkillMatcher::new(0.1, 2);
        let skills = vec![
            create_test_skill("test-a", "Test skill A"),
            create_test_skill("test-b", "Test skill B"),
            create_test_skill("test-c", "Test skill C"),
        ];

        let matches = matcher.match_skills("run test for all skills", &skills);

        assert!(matches.len() <= 2);
    }

    #[test]
    fn test_score_normalization() {
        let matcher = SkillMatcher::default();
        let skills = vec![create_test_skill(
            "comprehensive-skill",
            "comprehensive comprehensive comprehensive comprehensive",
        )];

        let matches = matcher.match_skills("comprehensive-skill comprehensive", &skills);

        assert!(!matches.is_empty());
        assert!(matches[0].1 <= 1.0); // Score should be capped at 1.0
    }
}
