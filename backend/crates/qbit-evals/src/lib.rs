//! Evaluation framework for testing agent capabilities.
//!
//! This module provides end-to-end evaluation of the Qbit agent using
//! rig's experimental evals framework.
//!
//! # Feature Flag
//!
//! This crate requires the `evals` feature in the main qbit crate:
//! ```bash
//! cargo build --features evals
//! ```
//!
//! # Architecture
//!
//! - `config`: Configuration loading from settings.toml
//! - `runner`: Test harness for running agent against testbeds
//! - `executor`: Lightweight agent executor for eval runs
//! - `metrics`: Evaluation metrics (code correctness, file state, LLM judge)
//! - `scenarios`: Individual eval scenarios (bug fix, feature impl, etc.)
//! - `outcome`: Result types and reporting

pub mod config;
pub mod executor;
pub mod metrics;
pub mod outcome;
pub mod runner;
pub mod scenarios;

pub use config::EvalConfig;
pub use executor::execute_eval_prompt;
pub use metrics::MetricResult;
pub use outcome::{EvalReport, MetricOutcome};
pub use runner::{AgentOutput, EvalRunner};

// Re-export indicatif for CLI progress bars
pub use indicatif;
