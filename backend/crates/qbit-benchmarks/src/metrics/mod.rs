//! Metrics for benchmark evaluation.
//!
//! Provides specialized metrics for different benchmark types:
//! - `PythonTestMetric`: Runs Python test code to verify solutions

mod python_test;

pub use python_test::PythonTestMetric;
