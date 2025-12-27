//! Bug fix evaluation scenario.
//!
//! Tests the agent's ability to fix a compile error in Rust code.

use async_trait::async_trait;

use crate::evals::metrics::{CodeCorrectnessMetric, FileStateMetric, LlmJudgeMetric, Metric};
use crate::evals::scenarios::Scenario;

/// Scenario: Fix a type error in Rust code.
pub struct BugFixScenario;

#[async_trait]
impl Scenario for BugFixScenario {
    fn name(&self) -> &str {
        "bug-fix"
    }

    fn description(&self) -> &str {
        "Fix a compile error in Rust code (type mismatch)"
    }

    fn testbed(&self) -> &str {
        "rust-bug-fix"
    }

    fn prompt(&self) -> &str {
        "Fix the compile error in src/lib.rs. The function should work correctly."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(CodeCorrectnessMetric::cargo_check()),
            Box::new(FileStateMetric::modified("lib_modified", "src/lib.rs")),
            Box::new(LlmJudgeMetric::new(
                "fix_quality",
                "The fix should be minimal and correct, not a workaround",
                0.7,
            )),
        ]
    }
}

/// Testbed files for the bug-fix scenario.
///
/// Creates a Rust project with a type error:
/// - Function returns i32 but declared to return String
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "bug-fix-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"/// Adds two numbers and returns the result as a string.
pub fn add_as_string(a: i32, b: i32) -> String {
    // BUG: Returns i32 instead of String
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_as_string() {
        assert_eq!(add_as_string(2, 3), "5");
    }
}
"#
            .to_string(),
        ),
    ]
}
