//! Benchmark integrations for Qbit AI agent evaluation.
//!
//! This crate provides standard benchmark suites for evaluating
//! the Qbit agent's capabilities, starting with HumanEval for
//! Python function synthesis.
//!
//! # Benchmarks
//!
//! - **HumanEval**: 164 Python function synthesis problems from OpenAI
//!
//! # Usage
//!
//! ```bash
//! # Run full HumanEval benchmark
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval
//!
//! # Run a subset of problems
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval --problems 0-9
//!
//! # Run specific problems
//! cargo run --features evals,cli --bin qbit-cli -- --benchmark humaneval --problems 0,5,10
//! ```

pub mod humaneval;
pub mod metrics;

pub use humaneval::{all_scenarios as humaneval_scenarios, scenarios_for_range, HumanEvalScenario};
pub use metrics::PythonTestMetric;

/// Get scenarios for a benchmark by name.
///
/// # Arguments
/// * `benchmark` - Name of the benchmark (e.g., "humaneval")
/// * `problems` - Optional problem filter (e.g., "0-10" or "0,5,10")
///
/// # Returns
/// A vector of scenarios for the benchmark, or an error if the benchmark is unknown.
pub fn get_benchmark_scenarios(
    benchmark: &str,
    problems: Option<&str>,
) -> anyhow::Result<Vec<Box<dyn qbit_evals::scenarios::Scenario>>> {
    match benchmark {
        "humaneval" => {
            if let Some(range) = problems {
                Ok(scenarios_for_range(range))
            } else {
                Ok(humaneval_scenarios())
            }
        }
        _ => anyhow::bail!("Unknown benchmark: {}. Available: humaneval", benchmark),
    }
}

/// List available benchmarks.
pub fn list_benchmarks() -> Vec<(&'static str, &'static str, usize)> {
    vec![(
        "humaneval",
        "164 Python function synthesis problems (OpenAI HumanEval)",
        164,
    )]
}
