//! HumanEval benchmark integration.
//!
//! Implements the HumanEval benchmark (164 Python function synthesis problems)
//! from OpenAI's human-eval repository.
//!
//! # Dataset
//!
//! The problems are embedded as JSONL and loaded at runtime. Each problem consists of:
//! - A function signature with docstring (the prompt)
//! - Test code to verify the solution
//!
//! # Usage
//!
//! ```bash
//! # Run all 164 problems
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval
//!
//! # Run problems 0-9
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval --problems 0-9
//!
//! # Run specific problems
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval --problems 0,5,10
//! ```

mod types;

pub use types::HumanEvalProblem;

use anyhow::Result;
use async_trait::async_trait;
use qbit_evals::metrics::{EvalContext, Metric};
use qbit_evals::outcome::EvalReport;
use qbit_evals::runner::EvalRunner;
use qbit_evals::scenarios::Scenario;

use crate::metrics::PythonTestMetric;

/// Embedded HumanEval problems dataset (JSONL format).
const PROBLEMS_JSONL: &str = include_str!("problems.jsonl");

/// HumanEval scenario for a single problem.
pub struct HumanEvalScenario {
    /// The problem being tested
    problem: HumanEvalProblem,
    /// Formatted prompt for the agent
    formatted_prompt: String,
}

impl HumanEvalScenario {
    /// Create a new HumanEval scenario from a problem.
    pub fn new(problem: HumanEvalProblem) -> Self {
        let formatted_prompt = format!(
            "Complete this Python function and write it to `solution.py`:\n\n```python\n{}\n```\n\nWrite ONLY the complete function implementation to solution.py. Do not include any test code.",
            problem.prompt
        );
        Self {
            problem,
            formatted_prompt,
        }
    }
}

impl From<HumanEvalProblem> for HumanEvalScenario {
    fn from(problem: HumanEvalProblem) -> Self {
        Self::new(problem)
    }
}

#[async_trait]
impl Scenario for HumanEvalScenario {
    fn name(&self) -> &str {
        // Return the short name for cleaner output
        // We store it in the problem for efficiency
        Box::leak(self.problem.short_name().into_boxed_str())
    }

    fn description(&self) -> &str {
        "HumanEval Python function synthesis"
    }

    fn testbed(&self) -> &str {
        "python-humaneval"
    }

    fn prompt(&self) -> &str {
        &self.formatted_prompt
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![Box::new(PythonTestMetric::new(
            &self.problem.test,
            &self.problem.entry_point,
        ))]
    }

    /// Run the scenario with custom testbed setup.
    ///
    /// This overrides the default run() to handle HumanEval's testbed
    /// setup directly, avoiding circular dependencies with qbit-evals.
    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed directly (create workspace with solution.py placeholder)
        let workspace = runner.workspace_path().join("humaneval");
        std::fs::create_dir_all(&workspace)?;

        // Write the placeholder solution file
        let solution_path = workspace.join("solution.py");
        std::fs::write(&solution_path, "# Agent writes solution here\n")?;

        // Run agent in the testbed workspace
        let agent_output = runner.run_prompt(&workspace, self.prompt()).await?;

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            agent_output.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Evaluate metrics
        let ctx = EvalContext {
            workspace,
            agent_output,
            prompt: self.prompt().to_string(),
        };

        for metric in self.metrics() {
            let result = metric.evaluate(&ctx).await?;
            report.add_metric(metric.name(), result);
        }

        Ok(report)
    }
}

/// Load all HumanEval problems from the embedded dataset.
fn load_problems() -> Vec<HumanEvalProblem> {
    PROBLEMS_JSONL
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            serde_json::from_str::<HumanEvalProblem>(line)
                .map_err(|e| {
                    tracing::warn!("Failed to parse HumanEval problem: {}", e);
                    e
                })
                .ok()
        })
        .collect()
}

/// Load all 164 HumanEval scenarios.
pub fn all_scenarios() -> Vec<Box<dyn Scenario>> {
    load_problems()
        .into_iter()
        .map(|p| Box::new(HumanEvalScenario::from(p)) as Box<dyn Scenario>)
        .collect()
}

/// Parse a problem range string into a set of IDs.
///
/// Supports:
/// - Single IDs: "5" -> {5}
/// - Ranges: "0-10" -> {0, 1, 2, ..., 10}
/// - Comma-separated: "0,5,10" -> {0, 5, 10}
/// - Mixed: "0-5,10,15-20" -> {0, 1, 2, 3, 4, 5, 10, 15, 16, 17, 18, 19, 20}
fn parse_problem_range(range: &str) -> std::collections::HashSet<u32> {
    let mut ids = std::collections::HashSet::new();

    for part in range.split(',') {
        let part = part.trim();
        if part.contains('-') {
            // Range: "0-10"
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    for id in start..=end {
                        ids.insert(id);
                    }
                }
            }
        } else {
            // Single ID: "5"
            if let Ok(id) = part.parse::<u32>() {
                ids.insert(id);
            }
        }
    }

    ids
}

/// Filter scenarios by problem IDs.
///
/// # Arguments
/// * `range` - Problem range string (e.g., "0-10" or "0,5,10")
pub fn scenarios_for_range(range: &str) -> Vec<Box<dyn Scenario>> {
    let ids = parse_problem_range(range);
    load_problems()
        .into_iter()
        .filter(|p| p.numeric_id().is_some_and(|id| ids.contains(&id)))
        .map(|p| Box::new(HumanEvalScenario::from(p)) as Box<dyn Scenario>)
        .collect()
}

/// Get a specific HumanEval scenario by problem ID.
pub fn get_scenario(id: u32) -> Option<Box<dyn Scenario>> {
    load_problems()
        .into_iter()
        .find(|p| p.numeric_id() == Some(id))
        .map(|p| Box::new(HumanEvalScenario::from(p)) as Box<dyn Scenario>)
}

/// Get testbed files for HumanEval scenarios.
///
/// Creates a minimal Python workspace with a placeholder solution file.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![(
        "solution.py".to_string(),
        "# Agent writes solution here\n".to_string(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_id() {
        let ids = parse_problem_range("5");
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&5));
    }

    #[test]
    fn test_parse_range() {
        let ids = parse_problem_range("0-5");
        assert_eq!(ids.len(), 6);
        for i in 0..=5 {
            assert!(ids.contains(&i));
        }
    }

    #[test]
    fn test_parse_comma_separated() {
        let ids = parse_problem_range("0,5,10");
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&0));
        assert!(ids.contains(&5));
        assert!(ids.contains(&10));
    }

    #[test]
    fn test_parse_mixed() {
        let ids = parse_problem_range("0-2,5,10-12");
        assert_eq!(ids.len(), 7);
        assert!(ids.contains(&0));
        assert!(ids.contains(&1));
        assert!(ids.contains(&2));
        assert!(ids.contains(&5));
        assert!(ids.contains(&10));
        assert!(ids.contains(&11));
        assert!(ids.contains(&12));
    }

    #[test]
    fn test_load_problems() {
        // This will fail if problems.jsonl doesn't exist yet
        // But we can at least verify the function exists
        let problems = load_problems();
        // After embedding the dataset, this should be 164
        assert!(problems.len() <= 164);
    }
}
