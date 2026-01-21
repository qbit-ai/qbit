//! HumanEval problem type definitions.
//!
//! Represents the structure of HumanEval problems from the JSONL dataset.

use serde::{Deserialize, Serialize};

/// A HumanEval problem from the dataset.
///
/// Each problem contains:
/// - A task ID (e.g., "HumanEval/0")
/// - A prompt (function signature + docstring)
/// - The entry point function name
/// - A canonical solution (not used in evaluation)
/// - Test code for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanEvalProblem {
    /// Unique task identifier (e.g., "HumanEval/0")
    pub task_id: String,

    /// The prompt containing function signature and docstring
    /// This is what the agent sees
    pub prompt: String,

    /// The name of the function to implement
    pub entry_point: String,

    /// The canonical solution (not shown to agent)
    pub canonical_solution: String,

    /// Test code with assertions
    /// Contains a `check(candidate)` function that tests the implementation
    pub test: String,
}

impl HumanEvalProblem {
    /// Extract the numeric ID from the task_id (e.g., "HumanEval/42" -> 42).
    pub fn numeric_id(&self) -> Option<u32> {
        self.task_id
            .strip_prefix("HumanEval/")
            .and_then(|s| s.parse().ok())
    }

    /// Get a short name for the problem (e.g., "HumanEval/0" -> "humaneval-0").
    pub fn short_name(&self) -> String {
        format!(
            "humaneval-{}",
            self.numeric_id()
                .map(|n| n.to_string())
                .unwrap_or_else(|| self.task_id.clone())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_id_extraction() {
        let problem = HumanEvalProblem {
            task_id: "HumanEval/42".to_string(),
            prompt: String::new(),
            entry_point: String::new(),
            canonical_solution: String::new(),
            test: String::new(),
        };
        assert_eq!(problem.numeric_id(), Some(42));
    }

    #[test]
    fn test_short_name() {
        let problem = HumanEvalProblem {
            task_id: "HumanEval/0".to_string(),
            prompt: String::new(),
            entry_point: String::new(),
            canonical_solution: String::new(),
            test: String::new(),
        };
        assert_eq!(problem.short_name(), "humaneval-0");
    }

    #[test]
    fn test_parse_from_json() {
        let json = r#"{
            "task_id": "HumanEval/0",
            "prompt": "def test():\n    pass",
            "entry_point": "test",
            "canonical_solution": "    return True",
            "test": "def check(candidate):\n    assert candidate() == True"
        }"#;
        let problem: HumanEvalProblem = serde_json::from_str(json).unwrap();
        assert_eq!(problem.task_id, "HumanEval/0");
        assert_eq!(problem.entry_point, "test");
    }
}
