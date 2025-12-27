//! Refactoring evaluation scenario.
//!
//! Tests the agent's ability to extract logic into a separate function
//! while preserving behavior.

use async_trait::async_trait;

use crate::metrics::{CodeCorrectnessMetric, FileStateMetric, LlmScoreMetric, Metric};
use crate::scenarios::Scenario;

/// Scenario: Extract validation logic into a separate function.
pub struct RefactorScenario;

#[async_trait]
impl Scenario for RefactorScenario {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "Extract validation logic into a separate function"
    }

    fn testbed(&self) -> &str {
        "rust-refactor"
    }

    fn prompt(&self) -> &str {
        "Extract the validation logic from the `process` function into a separate \
         `validate` function. Keep the existing tests passing."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(CodeCorrectnessMetric::cargo_test()),
            Box::new(FileStateMetric::contains(
                "has_validate_fn",
                "src/lib.rs",
                "fn validate",
            )),
            Box::new(LlmScoreMetric::scale_10(
                "code_quality",
                "Rate the cleanliness and organization of the refactored code",
                7.0,
            )),
        ]
    }
}

/// Testbed files for the refactor scenario.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "refactor-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"use std::collections::HashMap;

/// Input data to process.
#[derive(Debug)]
pub struct Input {
    pub name: String,
    pub email: String,
    pub age: i32,
    pub data: HashMap<String, String>,
}

/// Processing result.
#[derive(Debug)]
pub struct Output {
    pub processed_name: String,
    pub summary: String,
}

/// Error type for processing.
#[derive(Debug, PartialEq)]
pub enum ProcessError {
    EmptyName,
    InvalidEmail,
    InvalidAge,
    MissingRequiredField(String),
}

/// Process input data and return output.
pub fn process(input: &Input) -> Result<Output, ProcessError> {
    // Validation logic (should be extracted)
    if input.name.is_empty() {
        return Err(ProcessError::EmptyName);
    }

    if !input.email.contains('@') {
        return Err(ProcessError::InvalidEmail);
    }

    if input.age < 0 || input.age > 150 {
        return Err(ProcessError::InvalidAge);
    }

    if !input.data.contains_key("required_field") {
        return Err(ProcessError::MissingRequiredField("required_field".to_string()));
    }

    // Processing logic
    let processed_name = input.name.trim().to_uppercase();
    let summary = format!(
        "{} ({}) - {} fields",
        processed_name,
        input.email,
        input.data.len()
    );

    Ok(Output {
        processed_name,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_input() -> Input {
        let mut data = HashMap::new();
        data.insert("required_field".to_string(), "value".to_string());
        Input {
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            age: 30,
            data,
        }
    }

    #[test]
    fn test_valid_input() {
        let input = valid_input();
        let result = process(&input).unwrap();
        assert_eq!(result.processed_name, "JOHN");
    }

    #[test]
    fn test_empty_name() {
        let mut input = valid_input();
        input.name = "".to_string();
        assert_eq!(process(&input), Err(ProcessError::EmptyName));
    }

    #[test]
    fn test_invalid_email() {
        let mut input = valid_input();
        input.email = "invalid".to_string();
        assert_eq!(process(&input), Err(ProcessError::InvalidEmail));
    }

    #[test]
    fn test_invalid_age() {
        let mut input = valid_input();
        input.age = -1;
        assert_eq!(process(&input), Err(ProcessError::InvalidAge));
    }

    #[test]
    fn test_missing_required_field() {
        let mut input = valid_input();
        input.data.clear();
        assert_eq!(
            process(&input),
            Err(ProcessError::MissingRequiredField("required_field".to_string()))
        );
    }
}
"#
            .to_string(),
        ),
    ]
}
