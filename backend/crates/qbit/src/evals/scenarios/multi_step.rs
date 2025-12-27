//! Multi-step task evaluation scenario.
//!
//! Tests the agent's ability to complete a workflow requiring
//! multiple tools and steps.

use async_trait::async_trait;

use crate::evals::metrics::{CodeCorrectnessMetric, FileStateMetric, Metric};
use crate::evals::scenarios::Scenario;

/// Scenario: Complete a multi-step workflow.
pub struct MultiStepScenario;

#[async_trait]
impl Scenario for MultiStepScenario {
    fn name(&self) -> &str {
        "multi-step"
    }

    fn description(&self) -> &str {
        "Create a module, add a function, write tests, and verify they pass"
    }

    fn testbed(&self) -> &str {
        "rust-multi-step"
    }

    fn prompt(&self) -> &str {
        r#"Complete the following steps:

1. Create a new file `src/utils.rs` with a public function:
   ```rust
   pub fn is_palindrome(s: &str) -> bool { ... }
   ```
   The function should return true if the string reads the same forwards and backwards (case-insensitive, ignoring non-alphanumeric characters).

2. Add `pub mod utils;` to `src/lib.rs`

3. Create a test file `tests/utils_test.rs` that tests the is_palindrome function

4. Run `cargo test` to verify everything works

Make sure all files are created exactly as specified."#
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(FileStateMetric::exists("utils_module", "src/utils.rs")),
            Box::new(FileStateMetric::exists("test_file", "tests/utils_test.rs")),
            Box::new(FileStateMetric::contains(
                "mod_declaration",
                "src/lib.rs",
                "mod utils",
            )),
            Box::new(FileStateMetric::contains(
                "has_is_palindrome",
                "src/utils.rs",
                "fn is_palindrome",
            )),
            Box::new(CodeCorrectnessMetric::cargo_test()),
        ]
    }
}

/// Testbed files for the multi-step scenario.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "multi-step-testbed"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
            .to_string(),
        ),
        (
            "src/lib.rs".to_string(),
            r#"//! Multi-step testbed crate.

// Modules will be added here
"#
            .to_string(),
        ),
    ]
}
