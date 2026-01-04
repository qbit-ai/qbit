//! File state metric - verifies files exist and contain expected content.

use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use regex::Regex;

use super::{EvalContext, Metric, MetricResult};

/// Check type for file state verification.
#[derive(Debug, Clone)]
pub enum FileCheck {
    /// File must exist.
    Exists,
    /// File must not exist.
    NotExists,
    /// File must contain the given string.
    Contains(String),
    /// File must match the given regex pattern.
    Matches(String),
    /// File must have been modified (compared to original).
    Modified,
}

/// Metric that verifies file state after agent execution.
pub struct FileStateMetric {
    /// Name of this metric instance.
    name: String,
    /// Relative path to the file.
    path: PathBuf,
    /// Check to perform.
    check: FileCheck,
}

impl FileStateMetric {
    /// Create a metric that checks if a file exists.
    pub fn exists(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            check: FileCheck::Exists,
        }
    }

    /// Create a metric that checks if a file does NOT exist.
    pub fn not_exists(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            check: FileCheck::NotExists,
        }
    }

    /// Create a metric that checks if a file contains a string.
    pub fn contains(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        pattern: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            check: FileCheck::Contains(pattern.into()),
        }
    }

    /// Create a metric that checks if a file matches a regex.
    pub fn matches(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        pattern: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            check: FileCheck::Matches(pattern.into()),
        }
    }

    /// Create a metric that checks if a file was modified.
    pub fn modified(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            check: FileCheck::Modified,
        }
    }
}

#[async_trait]
impl Metric for FileStateMetric {
    fn name(&self) -> &str {
        &self.name
    }

    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        let full_path = ctx.workspace.join(&self.path);

        match &self.check {
            FileCheck::Exists => {
                if full_path.exists() {
                    Ok(MetricResult::Pass)
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!("File does not exist: {}", self.path.display()),
                    })
                }
            }
            FileCheck::NotExists => {
                if !full_path.exists() {
                    Ok(MetricResult::Pass)
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!("File should not exist: {}", self.path.display()),
                    })
                }
            }
            FileCheck::Contains(pattern) => {
                if !full_path.exists() {
                    return Ok(MetricResult::Fail {
                        reason: format!("File does not exist: {}", self.path.display()),
                    });
                }
                let content = std::fs::read_to_string(&full_path)?;
                if content.contains(pattern) {
                    Ok(MetricResult::Pass)
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!(
                            "File {} does not contain '{}'",
                            self.path.display(),
                            pattern
                        ),
                    })
                }
            }
            FileCheck::Matches(pattern) => {
                if !full_path.exists() {
                    return Ok(MetricResult::Fail {
                        reason: format!("File does not exist: {}", self.path.display()),
                    });
                }
                let content = std::fs::read_to_string(&full_path)?;
                let regex = Regex::new(pattern)?;
                if regex.is_match(&content) {
                    Ok(MetricResult::Pass)
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!(
                            "File {} does not match pattern '{}'",
                            self.path.display(),
                            pattern
                        ),
                    })
                }
            }
            FileCheck::Modified => {
                // Check if file is in the modified list
                let was_modified = ctx
                    .agent_output
                    .files_modified
                    .iter()
                    .any(|p| p.ends_with(&self.path));

                if was_modified {
                    Ok(MetricResult::Pass)
                } else {
                    Ok(MetricResult::Fail {
                        reason: format!("File was not modified: {}", self.path.display()),
                    })
                }
            }
        }
    }
}
