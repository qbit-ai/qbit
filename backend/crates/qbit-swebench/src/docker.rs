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
    ///
    /// Returns Ok(true) if image was pulled/exists, Ok(false) if image not found.
    pub async fn pull_image(&self, instance: &SWEBenchInstance) -> Result<bool> {
        let image = instance.docker_image();
        info!("Pulling Docker image: {}", image);

        let options = Some(CreateImageOptions {
            from_image: image.clone(),
            ..Default::default()
        });

        let mut stream = self.client.create_image(options, None, None);
        let start = Instant::now();
        let mut had_error = false;

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
                    let err_str = e.to_string();
                    // Check if the image already exists
                    if err_str.contains("already exists") {
                        debug!("Image already exists");
                        return Ok(true);
                    }
                    // Check for 404 / image not found errors
                    if err_str.contains("404") || err_str.contains("not found") || err_str.contains("No such image") {
                        warn!("Image not available: {}", image);
                        return Ok(false);
                    }
                    warn!("Pull warning: {}", e);
                    had_error = true;
                }
            }
        }

        // Verify image exists after pull
        if self.image_exists(instance).await {
            info!("Successfully pulled image: {}", image);
            Ok(true)
        } else if had_error {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Check if an image exists locally (tries all alternatives).
    pub async fn image_exists(&self, instance: &SWEBenchInstance) -> bool {
        for image in instance.docker_image_alternatives() {
            if self.client.inspect_image(&image).await.is_ok() {
                return true;
            }
        }
        false
    }

    /// Find which image is available for an instance (local or pullable).
    async fn find_available_image(&self, instance: &SWEBenchInstance) -> Option<String> {
        for image in instance.docker_image_alternatives() {
            // Check if already local
            if self.client.inspect_image(&image).await.is_ok() {
                return Some(image);
            }
        }
        None
    }

    /// Try to find or pull an image, checking all alternatives.
    async fn find_or_pull_image(&self, instance: &SWEBenchInstance) -> Result<Option<String>> {
        // First check if any image is already available locally
        if let Some(image) = self.find_available_image(instance).await {
            info!("Using cached image: {}", image);
            return Ok(Some(image));
        }

        // Try to pull each alternative image
        for image in instance.docker_image_alternatives() {
            info!("Trying to pull image: {}", image);
            if self.try_pull_image(&image).await? {
                return Ok(Some(image));
            }
        }

        Ok(None)
    }

    /// Try to pull a specific image. Returns Ok(true) if successful, Ok(false) if not found.
    async fn try_pull_image(&self, image: &str) -> Result<bool> {
        let options = Some(CreateImageOptions {
            from_image: image.to_string(),
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
                    let err_str = e.to_string();
                    if err_str.contains("already exists") {
                        return Ok(true);
                    }
                    if err_str.contains("404") || err_str.contains("not found") || err_str.contains("No such image") || err_str.contains("manifest unknown") {
                        debug!("Image not available: {}", image);
                        return Ok(false);
                    }
                    warn!("Pull warning for {}: {}", image, e);
                }
            }
        }

        // Verify image exists after pull
        Ok(self.client.inspect_image(image).await.is_ok())
    }

    /// Run tests for a SWE-bench instance.
    ///
    /// # Arguments
    /// * `instance` - The SWE-bench instance to test
    /// * `workspace` - Path to the workspace containing the modified repository
    ///
    /// Returns an error with "IMAGE_NOT_AVAILABLE" in the message if the Docker image
    /// doesn't exist for this instance.
    pub async fn run_tests(
        &self,
        instance: &SWEBenchInstance,
        workspace: &Path,
    ) -> Result<TestExecutionResult> {
        let start = Instant::now();

        // Try to find or pull an available image (tries alternatives)
        let image = match self.find_or_pull_image(instance).await? {
            Some(img) => img,
            None => {
                anyhow::bail!(
                    "IMAGE_NOT_AVAILABLE: No Docker image available for instance {}. \
                     Tried: {:?}",
                    instance.instance_id,
                    instance.docker_image_alternatives()
                );
            }
        };
        let container_name = format!("swebench-{}-{}", instance.instance_id, uuid::Uuid::new_v4());

        // Write the test patch to the workspace so it can be applied inside Docker
        // The test patch adds new test cases that verify the fix
        let test_patch_path = workspace.join("repo").join(".swebench_test_patch.diff");
        if !instance.test_patch.is_empty() {
            std::fs::write(&test_patch_path, &instance.test_patch)
                .with_context(|| format!("Failed to write test patch to {}", test_patch_path.display()))?;
            debug!("Wrote test patch ({} bytes) to {}", instance.test_patch.len(), test_patch_path.display());
        }

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

        // Build test command (will apply test patch before running tests)
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
        let fail_to_pass = instance.fail_to_pass_tests();
        let pass_to_pass = instance.pass_to_pass_tests();

        let mut all_tests: Vec<&str> = fail_to_pass.iter().map(|s| s.as_str()).collect();
        all_tests.extend(pass_to_pass.iter().map(|s| s.as_str()));

        // Build pytest command with specific tests
        let test_args = all_tests.join(" ");

        // Check if there's a test patch to apply
        let has_test_patch = !instance.test_patch.is_empty();

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

# Apply the test patch if it exists
# The test patch adds new test cases that verify the fix
{apply_test_patch}

# Run specific tests
python -m pytest {test_args} -v --tb=short 2>&1 || true
"#,
            apply_test_patch = if has_test_patch {
                r#"
if [ -f .swebench_test_patch.diff ]; then
    echo "Applying test patch..."
    echo "=== Test patch contents (first 50 lines) ==="
    head -50 .swebench_test_patch.diff
    echo "=== End of test patch preview ==="

    # Try git apply first (strict)
    if git apply --whitespace=nowarn --check .swebench_test_patch.diff 2>/dev/null; then
        git apply --whitespace=nowarn .swebench_test_patch.diff && echo "Test patch applied successfully (git apply)"
    # Try git apply with 3-way merge (handles some conflicts)
    elif git apply --whitespace=nowarn --3way .swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied with 3-way merge"
    # Fallback to patch command (more lenient)
    elif patch -p1 --forward --ignore-whitespace < .swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied successfully (patch -p1)"
    # Try patch with fuzz factor
    elif patch -p1 --forward --ignore-whitespace --fuzz=3 < .swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied with fuzz (patch -p1 --fuzz=3)"
    else
        echo "ERROR: Failed to apply test patch with all methods"
        echo "=== git apply error ==="
        git apply --whitespace=nowarn .swebench_test_patch.diff 2>&1 || true
        echo "=== Checking if files exist ==="
        head -20 .swebench_test_patch.diff | grep "^diff --git" | sed 's/diff --git a\///' | cut -d' ' -f1 | while read f; do
            if [ -f "$f" ]; then echo "EXISTS: $f"; else echo "MISSING: $f"; fi
        done
    fi
    rm -f .swebench_test_patch.diff
else
    echo "No test patch file found"
fi
"#
            } else {
                "echo 'No test patch for this instance'"
            },
            test_args = test_args
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
