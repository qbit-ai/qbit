//! Official SWE-bench harness integration.
//!
//! This module provides integration with the official SWE-bench Python harness
//! for authoritative test evaluation. Rather than implementing our own test
//! execution and result parsing, we delegate to the official harness which
//! handles all the complexity of different test runners, patch application,
//! and result grading.
//!
//! # Usage
//!
//! The official harness requires the `swebench` Python package:
//! ```bash
//! pip install swebench
//! ```

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::types::SWEBenchInstance;

/// Result from the official SWE-bench harness evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessResult {
    /// Whether the instance was resolved (all tests pass)
    pub resolved: bool,
    /// Whether the evaluation completed successfully
    pub completed: bool,
    /// Error message if evaluation failed
    pub error: Option<String>,
    /// Raw output from the harness
    pub output: String,
}

/// Prediction format expected by the official harness.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Prediction {
    instance_id: String,
    model_name_or_path: String,
    model_patch: String,
}

/// Run the official SWE-bench harness to evaluate an instance.
///
/// # Arguments
/// * `instance` - The SWE-bench instance being evaluated
/// * `workspace` - Path to the workspace containing the modified repository
/// * `model_name` - Name of the model/agent for logging purposes
///
/// # Returns
/// * `HarnessResult` with the evaluation outcome
///
/// # Requirements
/// * Python 3.8+ with `swebench` package installed
/// * Docker running for container-based evaluation
pub async fn run_official_harness(
    instance: &SWEBenchInstance,
    workspace: &Path,
    model_name: &str,
) -> Result<HarnessResult> {
    // Check if swebench harness is available
    if !is_swebench_available() {
        return Ok(HarnessResult {
            resolved: false,
            completed: false,
            error: Some(
                "swebench harness module not available. Install with: pip install 'swebench[harness]'"
                    .to_string(),
            ),
            output: String::new(),
        });
    }

    // Generate patch from workspace
    let repo_path = workspace.join("repo");
    let patch = generate_patch(&repo_path)?;

    if patch.is_empty() {
        return Ok(HarnessResult {
            resolved: false,
            completed: true,
            error: Some("No changes detected in workspace".to_string()),
            output: String::new(),
        });
    }

    debug!(
        "Generated patch for {}: {} bytes",
        instance.instance_id,
        patch.len()
    );

    // Create temporary directory for predictions and results
    let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let predictions_path = temp_dir.path().join("predictions.jsonl");
    let results_dir = temp_dir.path().join("results");

    // Write prediction file
    let prediction = Prediction {
        instance_id: instance.instance_id.clone(),
        model_name_or_path: model_name.to_string(),
        model_patch: patch,
    };
    let prediction_json = serde_json::to_string(&prediction)?;
    std::fs::write(&predictions_path, prediction_json)?;

    info!(
        "Running official harness for instance: {}",
        instance.instance_id
    );

    // Use venv python if available, otherwise system python
    let python = get_swebench_python()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "python".to_string());

    // Use a simple run ID - the harness creates {model_name}.{run_id}.json
    let run_id = "eval";

    // Run the official harness (updated for swebench 3.x API)
    let output = Command::new(&python)
        .args([
            "-m",
            "swebench.harness.run_evaluation",
            "-id",
            run_id,
            "-p",
            predictions_path.to_str().unwrap(),
            "-d",
            "princeton-nlp/SWE-bench_Lite",
            "--report_dir",
            results_dir.to_str().unwrap(),
            "-t",
            "600", // 10 minute timeout
            "-i",
            &instance.instance_id,
            "--max_workers",
            "1",
        ])
        .output()
        .context("Failed to run swebench harness")?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined_output = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);

    if !output.status.success() {
        warn!(
            "Harness exited with code {:?}: {}",
            output.status.code(),
            stderr
        );
    }

    // Parse results from the harness output
    let result = parse_harness_results(&results_dir, &instance.instance_id, &combined_output)?;

    // Clean up temp directory
    let _ = temp_dir.close();

    Ok(result)
}

/// Get the path to the swebench venv python, if it exists.
fn get_swebench_python() -> Option<std::path::PathBuf> {
    let home = dirs::home_dir()?;
    let venv_python = home.join(".qbit/swebench-venv/bin/python");
    if venv_python.exists() {
        Some(venv_python)
    } else {
        None
    }
}

/// Check if the swebench Python package with harness module is available.
pub fn is_swebench_available() -> bool {
    // Try venv python first, then system python
    let python = get_swebench_python()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "python".to_string());

    Command::new(&python)
        .args(["-c", "import swebench.harness.run_evaluation"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Generate a git diff from the workspace.
fn generate_patch(repo_path: &Path) -> Result<String> {
    // First, try to get diff of staged and unstaged changes
    let output = Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run git diff")?;

    if !output.status.success() {
        // Try without HEAD for repositories without commits
        let output = Command::new("git")
            .args(["diff"])
            .current_dir(repo_path)
            .output()
            .context("Failed to run git diff")?;

        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse results from the harness output directory.
fn parse_harness_results(
    results_dir: &Path,
    instance_id: &str,
    output: &str,
) -> Result<HarnessResult> {
    // Look for report files - try multiple patterns as the harness version may vary
    // New format: results_dir/*.json
    // Old format: results_dir/**/report.json
    let patterns = [
        results_dir.join("*.json"),
        results_dir.join("**").join("report.json"),
        results_dir.join("**").join("*.json"),
    ];

    // Try to find and parse any JSON report file
    for pattern in &patterns {
        if let Ok(entries) = glob::glob(pattern.to_str().unwrap()) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(&entry) {
                    if let Ok(report) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Check if this report contains our instance
                        if let Some(instance_result) = report.get(instance_id) {
                            let resolved = instance_result
                                .get("resolved")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            return Ok(HarnessResult {
                                resolved,
                                completed: true,
                                error: None,
                                output: output.to_string(),
                            });
                        }

                        // Also check resolved_ids array
                        if let Some(resolved_ids) = report.get("resolved_ids") {
                            if let Some(ids) = resolved_ids.as_array() {
                                let resolved = ids.iter().any(|id| {
                                    id.as_str().map(|s| s == instance_id).unwrap_or(false)
                                });

                                if resolved {
                                    return Ok(HarnessResult {
                                        resolved: true,
                                        completed: true,
                                        error: None,
                                        output: output.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // If we couldn't find the report, try to parse from stdout
    // The harness prints summary lines - handle both old and new formats
    let resolved = output.contains("Resolved: 1/1")
        || output.contains("Instances resolved: 1")
        || output.contains("✓=1")
        || output.contains(&format!("✓ {}", instance_id))
        || (output.contains("resolved_ids") && output.contains(instance_id));

    // Check for error indicators
    let has_error =
        output.contains("Error:") || output.contains("Traceback (most recent call last)");

    if has_error && !resolved {
        // Extract error message
        let error_msg = output
            .lines()
            .find(|line| line.contains("Error:") || line.contains("Exception:"))
            .map(|s| s.to_string());

        return Ok(HarnessResult {
            resolved: false,
            completed: true,
            error: error_msg,
            output: output.to_string(),
        });
    }

    Ok(HarnessResult {
        resolved,
        completed: true,
        error: None,
        output: output.to_string(),
    })
}

/// Fallback evaluation when official harness is not available.
///
/// This uses our simplified Docker-based evaluation as a fallback
/// when the official swebench package is not installed.
pub async fn run_fallback_evaluation(
    instance: &SWEBenchInstance,
    workspace: &Path,
) -> Result<HarnessResult> {
    use crate::docker::DockerExecutor;

    let docker = DockerExecutor::new()?;

    // Check Docker availability
    if !docker.is_available().await {
        return Ok(HarnessResult {
            resolved: false,
            completed: false,
            error: Some("Docker is not available".to_string()),
            output: String::new(),
        });
    }

    // Run tests using our Docker executor
    let test_result = docker.run_tests(instance, workspace).await?;

    let resolved = test_result.is_solved();
    let output = format!(
        "Exit code: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
        test_result.exit_code, test_result.stdout, test_result.stderr
    );

    let error = if !resolved {
        let (f2p_passed, f2p_total) = test_result.fail_to_pass_count();
        let (p2p_passed, p2p_total) = test_result.pass_to_pass_count();
        Some(format!(
            "FAIL_TO_PASS: {}/{}, PASS_TO_PASS: {}/{}",
            f2p_passed, f2p_total, p2p_passed, p2p_total
        ))
    } else {
        None
    };

    Ok(HarnessResult {
        resolved,
        completed: true,
        error,
        output,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_swebench_available() {
        // This test just checks the function runs without panic
        let _available = is_swebench_available();
    }

    #[test]
    fn test_parse_harness_results_from_output() {
        let output = "Running evaluation...\nResolved: 1/1\nCompleted successfully";
        let result =
            parse_harness_results(Path::new("/nonexistent"), "test__test-123", output).unwrap();
        assert!(result.resolved);
        assert!(result.completed);
    }

    #[test]
    fn test_parse_harness_results_with_error() {
        let output = "Error: Test execution failed\nTraceback (most recent call last):\n  ...";
        let result =
            parse_harness_results(Path::new("/nonexistent"), "test__test-123", output).unwrap();
        assert!(!result.resolved);
        assert!(result.error.is_some());
    }
}
