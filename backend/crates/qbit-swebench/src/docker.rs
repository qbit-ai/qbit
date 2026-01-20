//! Docker container execution for SWE-bench test running.
//!
//! Uses Epoch AI's optimized Docker images for SWE-bench evaluation.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::types::{SWEBenchInstance, TestExecutionResult, TestResult};

/// Default timeout for test execution in seconds.
const DEFAULT_TEST_TIMEOUT_SECS: u64 = 600; // 10 minutes

/// Default timeout for image pull in seconds.
const DEFAULT_PULL_TIMEOUT_SECS: u64 = 300; // 5 minutes

/// Docker executor for running SWE-bench tests.
pub struct DockerExecutor {
    /// Docker client
    client: Docker,
    /// Test execution timeout in seconds
    test_timeout_secs: u64,
    /// Image pull timeout in seconds
    pull_timeout_secs: u64,
}

impl DockerExecutor {
    /// Create a new Docker executor.
    pub fn new() -> Result<Self> {
        let client = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker daemon")?;

        Ok(Self {
            client,
            test_timeout_secs: DEFAULT_TEST_TIMEOUT_SECS,
            pull_timeout_secs: DEFAULT_PULL_TIMEOUT_SECS,
        })
    }

    /// Set the test execution timeout.
    pub fn with_test_timeout(mut self, secs: u64) -> Self {
        self.test_timeout_secs = secs;
        self
    }

    /// Set the image pull timeout.
    pub fn with_pull_timeout(mut self, secs: u64) -> Self {
        self.pull_timeout_secs = secs;
        self
    }

    /// Check if Docker is available and running.
    pub async fn is_available(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    /// Pull the Docker image for an instance.
    pub async fn pull_image(&self, instance: &SWEBenchInstance) -> Result<()> {
        let image = instance.docker_image();
        info!("Pulling Docker image: {}", image);

        let options = Some(CreateImageOptions {
            from_image: image.clone(),
            ..Default::default()
        });

        let mut stream = self.client.create_image(options, None, None);
        let start = Instant::now();

        while let Some(result) = stream.next().await {
            if start.elapsed() > Duration::from_secs(self.pull_timeout_secs) {
                anyhow::bail!("Image pull timed out after {} seconds", self.pull_timeout_secs);
            }

            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!("Pull status: {}", status);
                    }
                }
                Err(e) => {
                    // Check if the image already exists
                    if e.to_string().contains("already exists") {
                        debug!("Image already exists");
                        return Ok(());
                    }
                    warn!("Pull warning: {}", e);
                }
            }
        }

        info!("Successfully pulled image: {}", image);
        Ok(())
    }

    /// Check if an image exists locally.
    pub async fn image_exists(&self, instance: &SWEBenchInstance) -> bool {
        let image = instance.docker_image();
        self.client.inspect_image(&image).await.is_ok()
    }

    /// Run tests for a SWE-bench instance.
    ///
    /// # Arguments
    /// * `instance` - The SWE-bench instance to test
    /// * `workspace` - Path to the workspace containing the modified repository
    pub async fn run_tests(
        &self,
        instance: &SWEBenchInstance,
        workspace: &Path,
    ) -> Result<TestExecutionResult> {
        let start = Instant::now();

        // Ensure image is available
        if !self.image_exists(instance).await {
            self.pull_image(instance).await?;
        }

        let image = instance.docker_image();
        let container_name = format!("swebench-{}-{}", instance.instance_id, uuid::Uuid::new_v4());

        // Create container configuration
        let workspace_abs = workspace
            .canonicalize()
            .with_context(|| format!("Failed to resolve workspace path: {}", workspace.display()))?;

        let host_config = HostConfig {
            mounts: Some(vec![Mount {
                target: Some("/workspace".to_string()),
                source: Some(workspace_abs.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            }]),
            // Set memory and CPU limits for safety
            memory: Some(4 * 1024 * 1024 * 1024), // 4GB
            memory_swap: Some(4 * 1024 * 1024 * 1024),
            nano_cpus: Some(2_000_000_000), // 2 CPUs
            ..Default::default()
        };

        // Build test command
        let test_cmd = self.build_test_command(instance);

        let config = Config {
            image: Some(image.clone()),
            cmd: Some(vec![
                "/bin/bash".to_string(),
                "-c".to_string(),
                test_cmd,
            ]),
            working_dir: Some("/workspace/repo".to_string()),
            host_config: Some(host_config),
            env: Some(vec![
                "PYTHONDONTWRITEBYTECODE=1".to_string(),
                "PYTHONUNBUFFERED=1".to_string(),
            ]),
            ..Default::default()
        };

        // Create container
        let create_options = Some(CreateContainerOptions {
            name: &container_name,
            platform: None,
        });

        let container = self
            .client
            .create_container(create_options, config)
            .await
            .context("Failed to create container")?;

        debug!("Created container: {}", container.id);

        // Start container
        self.client
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        debug!("Started container: {}", container.id);

        // Wait for container to finish with timeout
        let wait_result = tokio::time::timeout(
            Duration::from_secs(self.test_timeout_secs),
            self.wait_for_container(&container.id),
        )
        .await;

        let exit_code = match wait_result {
            Ok(Ok(code)) => code,
            Ok(Err(e)) => {
                warn!("Container wait error: {}", e);
                -1
            }
            Err(_) => {
                warn!("Container execution timed out");
                // Kill the container
                let _ = self.client.kill_container::<String>(&container.id, None).await;
                -1
            }
        };

        // Get container logs
        let (stdout, stderr) = self.get_container_logs(&container.id).await?;

        // Remove container
        let remove_options = Some(RemoveContainerOptions {
            force: true,
            v: true,
            ..Default::default()
        });

        let _ = self.client.remove_container(&container.id, remove_options).await;

        // Parse test results from output
        let (fail_to_pass_results, pass_to_pass_results) =
            self.parse_test_results(instance, &stdout, &stderr);

        Ok(TestExecutionResult {
            execution_success: exit_code == 0,
            exit_code,
            stdout,
            stderr,
            fail_to_pass_results,
            pass_to_pass_results,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Wait for a container to finish.
    async fn wait_for_container(&self, container_id: &str) -> Result<i32> {
        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.client.wait_container(container_id, Some(options));

        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    return Ok(response.status_code as i32);
                }
                Err(e) => {
                    return Err(e.into());
                }
            }
        }

        anyhow::bail!("Container wait stream ended unexpectedly")
    }

    /// Get container logs.
    async fn get_container_logs(&self, container_id: &str) -> Result<(String, String)> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            ..Default::default()
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut stream = self.client.logs(container_id, Some(options));

        while let Some(result) = stream.next().await {
            match result {
                Ok(log) => match log {
                    LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                },
                Err(e) => {
                    warn!("Error reading logs: {}", e);
                }
            }
        }

        Ok((stdout, stderr))
    }

    /// Build the test command for an instance.
    fn build_test_command(&self, instance: &SWEBenchInstance) -> String {
        // Apply test patch and run the relevant tests
        let fail_to_pass = instance.fail_to_pass_tests();
        let pass_to_pass = instance.pass_to_pass_tests();

        let mut all_tests: Vec<&str> = fail_to_pass.iter().map(|s| s.as_str()).collect();
        all_tests.extend(pass_to_pass.iter().map(|s| s.as_str()));

        // Build pytest command with specific tests
        let test_args = all_tests.join(" ");

        // Epoch AI containers use conda with a 'testbed' environment
        // We need to activate it before running pytest
        format!(
            r#"
set -e
cd /workspace/repo

# Source conda and activate the testbed environment
source /opt/miniconda3/etc/profile.d/conda.sh
conda activate testbed

# Show which python/pytest we're using for debugging
which python
python --version
which pytest || echo "pytest not in PATH, trying python -m pytest"

# Run specific tests
python -m pytest {} -v --tb=short 2>&1 || true
"#,
            test_args
        )
    }

    /// Parse test results from output.
    fn parse_test_results(
        &self,
        instance: &SWEBenchInstance,
        stdout: &str,
        _stderr: &str,
    ) -> (Vec<TestResult>, Vec<TestResult>) {
        let fail_to_pass = instance.fail_to_pass_tests();
        let pass_to_pass = instance.pass_to_pass_tests();

        // Parse pytest output for test results
        // Look for patterns like: "test_name PASSED" or "test_name FAILED"
        let mut results: HashMap<String, bool> = HashMap::new();

        for line in stdout.lines() {
            let line = line.trim();

            // pytest verbose output: "test_module.py::test_name PASSED"
            if line.contains(" PASSED") || line.contains(" FAILED") || line.contains(" ERROR") {
                let passed = line.contains(" PASSED");
                // Extract test name (everything before the status)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let test_name = parts[0].to_string();
                    results.insert(test_name, passed);
                }
            }
        }

        // Map results to expected test lists
        let fail_to_pass_results: Vec<TestResult> = fail_to_pass
            .iter()
            .map(|test| {
                let passed = self.find_test_result(&results, test);
                TestResult {
                    name: test.clone(),
                    passed,
                    error: if passed { None } else { Some("Test did not pass".to_string()) },
                    duration_ms: None,
                }
            })
            .collect();

        let pass_to_pass_results: Vec<TestResult> = pass_to_pass
            .iter()
            .map(|test| {
                let passed = self.find_test_result(&results, test);
                TestResult {
                    name: test.clone(),
                    passed,
                    error: if passed { None } else { Some("Test regression".to_string()) },
                    duration_ms: None,
                }
            })
            .collect();

        (fail_to_pass_results, pass_to_pass_results)
    }

    /// Find test result by name (handles various naming formats).
    fn find_test_result(&self, results: &HashMap<String, bool>, test_name: &str) -> bool {
        // Direct match
        if let Some(&passed) = results.get(test_name) {
            return passed;
        }

        // Partial match (test name might be part of the key)
        for (key, &passed) in results {
            if key.contains(test_name) || test_name.contains(key.as_str()) {
                return passed;
            }
        }

        // Default to failed if not found
        false
    }
}

impl Default for DockerExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create default DockerExecutor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_docker_connection() {
        // This test requires Docker to be running
        if let Ok(executor) = DockerExecutor::new() {
            let available = executor.is_available().await;
            println!("Docker available: {}", available);
        }
    }
}
