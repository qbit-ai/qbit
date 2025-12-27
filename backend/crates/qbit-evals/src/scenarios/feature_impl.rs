//! Feature implementation evaluation scenario.
//!
//! Tests the agent's ability to implement a new method.

use async_trait::async_trait;

use crate::metrics::{CodeCorrectnessMetric, FileStateMetric, LlmJudgeMetric, Metric};
use crate::scenarios::Scenario;

/// Scenario: Implement a new method on an existing struct.
pub struct FeatureImplScenario;

#[async_trait]
impl Scenario for FeatureImplScenario {
    fn name(&self) -> &str {
        "feature-impl"
    }

    fn description(&self) -> &str {
        "Implement a reverse method for StringUtils"
    }

    fn testbed(&self) -> &str {
        "rust-feature"
    }

    fn prompt(&self) -> &str {
        "Add a `reverse` method to StringUtils that reverses a string. \
         The test in tests/integration.rs should pass when you're done."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(CodeCorrectnessMetric::cargo_test()),
            Box::new(FileStateMetric::contains(
                "has_reverse_method",
                "src/lib.rs",
                "fn reverse",
            )),
            Box::new(LlmJudgeMetric::new(
                "implementation_quality",
                "Implementation should be idiomatic Rust",
                0.7,
            )),
        ]
    }
}

/// Testbed files for the feature-impl scenario.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "feature-impl-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"/// Utility struct for string operations.
pub struct StringUtils;

impl StringUtils {
    /// Convert a string to uppercase.
    pub fn uppercase(&self, s: &str) -> String {
        s.to_uppercase()
    }

    /// Convert a string to lowercase.
    pub fn lowercase(&self, s: &str) -> String {
        s.to_lowercase()
    }

    // TODO: Add reverse method
}
"#
            .to_string(),
        ),
        (
            "tests/integration.rs".to_string(),
            r#"use feature_impl_testbed::StringUtils;

#[test]
fn test_uppercase() {
    let utils = StringUtils;
    assert_eq!(utils.uppercase("hello"), "HELLO");
}

#[test]
fn test_lowercase() {
    let utils = StringUtils;
    assert_eq!(utils.lowercase("HELLO"), "hello");
}

#[test]
fn test_reverse() {
    let utils = StringUtils;
    assert_eq!(utils.reverse("hello"), "olleh");
    assert_eq!(utils.reverse(""), "");
    assert_eq!(utils.reverse("a"), "a");
}
"#
            .to_string(),
        ),
    ]
}
