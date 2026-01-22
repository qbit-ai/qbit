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
        let client =
            Docker::connect_with_local_defaults().context("Failed to connect to Docker daemon")?;

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
                anyhow::bail!(
                    "Image pull timed out after {} seconds",
                    self.pull_timeout_secs
                );
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
                    if err_str.contains("404")
                        || err_str.contains("not found")
                        || err_str.contains("No such image")
                    {
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
                anyhow::bail!(
                    "Image pull timed out after {} seconds",
                    self.pull_timeout_secs
                );
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
                    if err_str.contains("404")
                        || err_str.contains("not found")
                        || err_str.contains("No such image")
                        || err_str.contains("manifest unknown")
                    {
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

    /// Start a testbed container that stays running for agent interaction.
    ///
    /// This starts a container with the workspace mounted, allowing the agent
    /// to run commands (like pytest) inside the container via `docker exec`.
    ///
    /// # Arguments
    /// * `instance` - The SWE-bench instance
    /// * `workspace` - Path to the workspace containing the repository
    ///
    /// # Returns
    /// * Container name that can be used with `docker exec`
    /// * Returns error with "IMAGE_NOT_AVAILABLE" if no image is available
    pub async fn start_testbed_container(
        &self,
        instance: &SWEBenchInstance,
        workspace: &Path,
    ) -> Result<String> {
        // Try to find or pull an available image
        let image = match self.find_or_pull_image(instance).await? {
            Some(img) => img,
            None => {
                anyhow::bail!(
                    "IMAGE_NOT_AVAILABLE: No Docker image available for instance {}",
                    instance.instance_id
                );
            }
        };

        let container_name = format!(
            "swebench-testbed-{}",
            instance.instance_id.replace("__", "-")
        );

        // Check if container already exists and remove it
        if self
            .client
            .inspect_container(&container_name, None)
            .await
            .is_ok()
        {
            info!("Removing existing container: {}", container_name);
            let remove_options = Some(RemoveContainerOptions {
                force: true,
                v: true,
                ..Default::default()
            });
            let _ = self
                .client
                .remove_container(&container_name, remove_options)
                .await;
        }

        let workspace_abs = workspace.canonicalize().with_context(|| {
            format!("Failed to resolve workspace path: {}", workspace.display())
        })?;

        let host_config = HostConfig {
            mounts: Some(vec![Mount {
                target: Some("/workspace".to_string()),
                source: Some(workspace_abs.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(false),
                ..Default::default()
            }]),
            memory: Some(4 * 1024 * 1024 * 1024), // 4GB
            memory_swap: Some(4 * 1024 * 1024 * 1024),
            nano_cpus: Some(2_000_000_000), // 2 CPUs
            ..Default::default()
        };

        // Start container with a long-running command (sleep infinity)
        let config = Config {
            image: Some(image.clone()),
            cmd: Some(vec![
                "/bin/bash".to_string(),
                "-c".to_string(),
                "sleep infinity".to_string(),
            ]),
            working_dir: Some("/workspace/repo".to_string()),
            host_config: Some(host_config),
            env: Some(vec![
                "PYTHONDONTWRITEBYTECODE=1".to_string(),
                "PYTHONUNBUFFERED=1".to_string(),
            ]),
            ..Default::default()
        };

        let create_options = Some(CreateContainerOptions {
            name: &container_name,
            platform: None,
        });

        let container = self
            .client
            .create_container(create_options, config)
            .await
            .context("Failed to create testbed container")?;

        debug!("Created testbed container: {}", container.id);

        self.client
            .start_container(&container.id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start testbed container")?;

        info!(
            "Started testbed container: {} ({})",
            container_name,
            &container.id[..12]
        );

        Ok(container_name)
    }

    /// Stop and remove a testbed container.
    pub async fn stop_container(&self, container_name: &str) -> Result<()> {
        info!("Stopping testbed container: {}", container_name);

        let remove_options = Some(RemoveContainerOptions {
            force: true,
            v: true,
            ..Default::default()
        });

        self.client
            .remove_container(container_name, remove_options)
            .await
            .with_context(|| format!("Failed to remove container: {}", container_name))?;

        Ok(())
    }

    /// Apply a test patch to /testbed inside a running container.
    ///
    /// This is used to add the FAIL_TO_PASS tests to the container's testbed
    /// so the agent can run them during its work. Since we exclude test files
    /// from syncing (to prevent the agent from modifying them), we need to
    /// apply the test patch directly.
    pub async fn apply_test_patch_to_container(
        &self,
        container_name: &str,
        test_patch: &str,
    ) -> Result<()> {
        use bollard::exec::{CreateExecOptions, StartExecResults};

        if test_patch.is_empty() {
            return Ok(());
        }

        // Write patch to a temp file inside the container and apply it
        // We use base64 encoding to safely pass the patch content
        let patch_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            test_patch.as_bytes(),
        );

        let apply_cmd = format!(
            r#"
cd /testbed
echo '{}' | base64 -d > /tmp/test_patch.diff
if git apply --whitespace=nowarn --check /tmp/test_patch.diff 2>/dev/null; then
    git apply --whitespace=nowarn /tmp/test_patch.diff
    echo "Test patch applied successfully"
elif git apply --whitespace=nowarn --reverse --check /tmp/test_patch.diff 2>/dev/null; then
    echo "Test patch already applied"
else
    # Try with patch command as fallback
    patch -p1 --forward < /tmp/test_patch.diff 2>/dev/null || echo "Patch may already be applied"
fi
rm -f /tmp/test_patch.diff
"#,
            patch_b64
        );

        let exec_options = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["bash", "-c", &apply_cmd]),
            ..Default::default()
        };

        let exec = self
            .client
            .create_exec(container_name, exec_options)
            .await
            .context("Failed to create exec for test patch")?;

        match self.client.start_exec(&exec.id, None).await? {
            StartExecResults::Attached { mut output, .. } => {
                while let Some(Ok(msg)) = output.next().await {
                    match msg {
                        bollard::container::LogOutput::StdOut { message } => {
                            debug!(
                                "Test patch output: {}",
                                String::from_utf8_lossy(&message).trim()
                            );
                        }
                        bollard::container::LogOutput::StdErr { message } => {
                            debug!(
                                "Test patch stderr: {}",
                                String::from_utf8_lossy(&message).trim()
                            );
                        }
                        _ => {}
                    }
                }
            }
            StartExecResults::Detached => {}
        }

        Ok(())
    }

    /// Get the docker exec command prefix for running commands in a testbed container.
    ///
    /// Returns a command string that can be used with shell execution to run
    /// commands inside the container with the conda testbed environment activated.
    pub fn get_docker_exec_command(container_name: &str, command: &str) -> String {
        format!(
            "docker exec {} bash -c 'source /opt/miniconda3/etc/profile.d/conda.sh && conda activate testbed && {}'",
            container_name,
            command.replace('\'', "'\\''") // Escape single quotes
        )
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
            std::fs::write(&test_patch_path, &instance.test_patch).with_context(|| {
                format!(
                    "Failed to write test patch to {}",
                    test_patch_path.display()
                )
            })?;
            debug!(
                "Wrote test patch ({} bytes) to {}",
                instance.test_patch.len(),
                test_patch_path.display()
            );
        }

        // Create container configuration
        let workspace_abs = workspace.canonicalize().with_context(|| {
            format!("Failed to resolve workspace path: {}", workspace.display())
        })?;

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
            cmd: Some(vec!["/bin/bash".to_string(), "-c".to_string(), test_cmd]),
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
                let _ = self
                    .client
                    .kill_container::<String>(&container.id, None)
                    .await;
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

        let _ = self
            .client
            .remove_container(&container.id, remove_options)
            .await;

        // Parse test results from output
        let (fail_to_pass_results, pass_to_pass_results) =
            Self::parse_test_results(instance, &stdout, &stderr);

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

        // Get the first (and typically only) result from the wait stream
        if let Some(result) = stream.next().await {
            match result {
                Ok(response) => Ok(response.status_code as i32),
                Err(e) => Err(e.into()),
            }
        } else {
            anyhow::bail!("Container wait stream ended unexpectedly")
        }
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
    ///
    /// Uses the repository-specific test runner from the official SWE-bench specs.
    /// Test names from FAIL_TO_PASS and PASS_TO_PASS are passed as-is without
    /// conversion - they're already in the correct format for each repository's
    /// test runner.
    fn build_test_command(&self, instance: &SWEBenchInstance) -> String {
        let fail_to_pass = instance.fail_to_pass_tests();
        let pass_to_pass = instance.pass_to_pass_tests();
        let test_cmd = instance.test_command();

        // Combine all test names and quote them to handle special characters
        let all_tests: Vec<String> = fail_to_pass
            .iter()
            .chain(pass_to_pass.iter())
            .map(|t| format!("'{}'", t.replace('\'', "'\\''")))
            .collect();
        let test_args = all_tests.join(" ");

        // Check if there's a test patch to apply
        let has_test_patch = !instance.test_patch.is_empty();

        // The test runner type determines how we identify it in logs
        let runner_name = match instance.test_runner() {
            crate::types::TestRunner::Django => "Django",
            crate::types::TestRunner::SymPy => "SymPy",
            crate::types::TestRunner::Sphinx => "Sphinx/tox",
            crate::types::TestRunner::Pytest => "pytest",
        };

        // Epoch AI containers have the repo at /testbed with the environment pre-configured.
        // We need to:
        // 1. Copy changes from /workspace/repo to /testbed
        // 2. Apply the test patch to /testbed
        // 3. Run tests from /testbed (to avoid path conflicts with conftest.py)
        format!(
            r#"
set -e

# Source conda and activate the testbed environment
source /opt/miniconda3/etc/profile.d/conda.sh
conda activate testbed

# Show which python we're using for debugging
which python
python --version

# Copy agent's changes from /workspace/repo to /testbed
# This preserves the container's environment while applying the agent's fixes
# IMPORTANT: Test files are EXCLUDED - they should not be modified by the agent
echo "=== Syncing changes from /workspace/repo to /testbed ==="
cd /workspace/repo

# Function to check if a file is a test file
is_test_file() {{
    local file="$1"
    case "$file" in
        tests/*|test/*|*/tests/*|*/test/*|test_*.py|*_test.py)
            return 0  # true - is a test file
            ;;
        *)
            return 1  # false - not a test file
            ;;
    esac
}}

# Find modified files and copy them to /testbed
# Use git diff to find changed files (if git is available)
if [ -d .git ]; then
    # Get list of modified/added files
    for file in $(git diff --name-only HEAD 2>/dev/null || git status --porcelain | awk '{{print $2}}'); do
        if [ -f "$file" ]; then
            # Skip test files - they should not be modified by the agent
            if is_test_file "$file"; then
                echo "  Skipped (test file): $file"
                continue
            fi
            # Create directory structure in /testbed if needed
            mkdir -p "/testbed/$(dirname "$file")"
            cp "$file" "/testbed/$file"
            echo "  Copied: $file"
        fi
    done
else
    # Fallback: copy all Python files that differ (excluding test files)
    echo "  No git repo, copying all modified Python files..."
    find . -name "*.py" -newer /testbed ! -path "*/tests/*" ! -path "*/test/*" ! -name "test_*.py" -exec cp --parents {{}} /testbed/ \; 2>/dev/null || true
fi

# Now work from /testbed
cd /testbed

# Apply the test patch if it exists
{apply_test_patch}

# Run tests using {runner_name}
echo "=== Running tests with {runner_name} ==="
{test_cmd} {test_args} 2>&1
"#,
            apply_test_patch = if has_test_patch {
                r#"
if [ -f /workspace/repo/.swebench_test_patch.diff ]; then
    echo "Applying test patch..."
    echo "=== Test patch contents (first 50 lines) ==="
    head -50 /workspace/repo/.swebench_test_patch.diff
    echo "=== End of test patch preview ==="

    # Check if patch is already applied (via git apply --reverse --check)
    if git apply --whitespace=nowarn --reverse --check /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch already applied (skipping)"
    # Try git apply first (strict)
    elif git apply --whitespace=nowarn --check /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
        git apply --whitespace=nowarn /workspace/repo/.swebench_test_patch.diff && echo "Test patch applied successfully (git apply)"
    # Try git apply with 3-way merge (handles some conflicts)
    elif git apply --whitespace=nowarn --3way /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied with 3-way merge"
    # Fallback to patch command (more lenient)
    elif patch -p1 --forward --ignore-whitespace < /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied successfully (patch -p1)"
    # Try patch with fuzz factor
    elif patch -p1 --forward --ignore-whitespace --fuzz=3 < /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
        echo "Test patch applied with fuzz (patch -p1 --fuzz=3)"
    else
        # Check if it might already be applied
        if git apply --whitespace=nowarn --reverse --check /workspace/repo/.swebench_test_patch.diff 2>/dev/null; then
            echo "Test patch already applied"
        else
            echo "WARNING: Could not apply test patch (may already be partially applied)"
        fi
    fi
else
    echo "No test patch file found"
fi
"#
            } else {
                "echo 'No test patch for this instance'"
            },
            runner_name = runner_name,
            test_cmd = test_cmd,
            test_args = test_args,
        )
    }

    /// Strip ANSI escape codes from a string.
    fn strip_ansi_codes(s: &str) -> String {
        // ANSI escape codes start with ESC[ (0x1B 0x5B) and end with a letter
        // Common patterns: \x1b[0m, \x1b[31m, \x1b[32m, etc.
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip the escape sequence
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                                  // Skip until we hit a letter (the terminator)
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    /// Parse test results from output.
    ///
    /// This is a pure parsing function that doesn't require Docker.
    fn parse_test_results(
        instance: &SWEBenchInstance,
        stdout: &str,
        stderr: &str,
    ) -> (Vec<TestResult>, Vec<TestResult>) {
        let fail_to_pass = instance.fail_to_pass_tests();
        let pass_to_pass = instance.pass_to_pass_tests();

        // Strip ANSI codes from output for reliable parsing
        let clean_stdout = Self::strip_ansi_codes(stdout);
        let clean_stderr = Self::strip_ansi_codes(stderr);

        // Parse test output for test results
        // Supports both pytest and Django test runner formats
        let mut results: HashMap<String, bool> = HashMap::new();

        // Combine stdout and stderr for error extraction
        let combined_output = format!("{}\n{}", clean_stdout, clean_stderr);

        debug!(
            "Parsing test results from {} lines of output",
            clean_stdout.lines().count()
        );

        let is_django = instance.test_runner() == crate::types::TestRunner::Django;

        for line in clean_stdout.lines() {
            let line = line.trim();

            // Django test runner format: "test_method (module.TestClass) ... ok/FAIL/ERROR"
            // Examples:
            //   test_override_file_upload_permissions (test_utils.tests.OverrideSettingsTests) ... ok
            //   test_something (admin_views.tests.AdminViewTest) ... FAIL
            //   module_name (unittest.loader._FailedTest) ... ERROR
            if is_django && line.contains(" ... ") {
                // Parse Django format
                if let Some((test_part, status_part)) = line.rsplit_once(" ... ") {
                    let passed = status_part.trim().eq_ignore_ascii_case("ok");
                    let is_error = status_part.trim().eq_ignore_ascii_case("error");

                    // Extract test method and class: "test_method (module.Class)"
                    // Also handle _FailedTest: "module (unittest.loader._FailedTest)"
                    if let Some((method_name, class_part)) = test_part.rsplit_once(" (") {
                        let class_path = class_part.trim_end_matches(')');

                        // Check for _FailedTest (module import failure)
                        if class_path.contains("_FailedTest") {
                            // The method_name here is actually the module that failed
                            // Mark any tests containing this module as failed
                            debug!(
                                "Django module load failure: {} ({})",
                                method_name, class_path
                            );
                            // Store with a special key to match later
                            results.insert(format!("__module_fail__{}", method_name), false);
                            continue;
                        }

                        // Build the full test name: module.Class.method
                        let full_test_name = format!("{}.{}", class_path, method_name);
                        debug!(
                            "Parsed Django test result: {} = {}",
                            full_test_name,
                            if passed {
                                "ok"
                            } else if is_error {
                                "ERROR"
                            } else {
                                "FAIL"
                            }
                        );
                        results.insert(full_test_name, passed);

                        // Also store just the class path for partial matching
                        // (in case FAIL_TO_PASS has module.Class without method)
                        if !results.contains_key(class_path) || passed {
                            results.insert(class_path.to_string(), passed);
                        }
                    }
                }
            }
            // pytest verbose output: "test_module.py::test_name PASSED"
            else if line.contains(" PASSED")
                || line.contains(" FAILED")
                || line.contains(" ERROR")
            {
                let passed = line.contains(" PASSED");
                // Extract test name (everything before the status)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    let test_name = parts[0].to_string();
                    debug!(
                        "Parsed pytest result: {} = {}",
                        test_name,
                        if passed { "PASSED" } else { "FAILED" }
                    );
                    results.insert(test_name, passed);
                }
            }
        }

        debug!("Found {} test results in output", results.len());

        // Log what we're looking for vs what we found
        debug!("Looking for FAIL_TO_PASS tests: {:?}", fail_to_pass);
        debug!("Looking for PASS_TO_PASS tests: {:?}", pass_to_pass);
        debug!(
            "Parsed result keys: {:?}",
            results.keys().collect::<Vec<_>>()
        );

        // Extract error messages from pytest output
        // Look for common error patterns
        let error_patterns = Self::extract_error_messages(&combined_output);

        // Map results to expected test lists
        let fail_to_pass_results: Vec<TestResult> = fail_to_pass
            .iter()
            .map(|test| {
                let passed = Self::find_test_result(&results, test);
                debug!(
                    "FAIL_TO_PASS test '{}': passed={} (looking in {} parsed results)",
                    test,
                    passed,
                    results.len()
                );
                let error = if passed {
                    None
                } else {
                    Self::find_error_for_test(&error_patterns, test, &combined_output)
                        .or_else(|| Some("Test did not pass".to_string()))
                };
                TestResult {
                    name: test.clone(),
                    passed,
                    error,
                    duration_ms: None,
                }
            })
            .collect();

        let pass_to_pass_results: Vec<TestResult> = pass_to_pass
            .iter()
            .map(|test| {
                let passed = Self::find_test_result(&results, test);
                let error = if passed {
                    None
                } else {
                    Self::find_error_for_test(&error_patterns, test, &combined_output)
                        .or_else(|| Some("Test regression".to_string()))
                };
                TestResult {
                    name: test.clone(),
                    passed,
                    error,
                    duration_ms: None,
                }
            })
            .collect();

        (fail_to_pass_results, pass_to_pass_results)
    }

    /// Extract error messages from pytest output.
    fn extract_error_messages(output: &str) -> HashMap<String, String> {
        let mut errors = HashMap::new();
        let lines: Vec<&str> = output.lines().collect();

        // Look for error blocks in pytest output
        // Format: "FAILED test_name - ErrorType: message"
        for line in &lines {
            if line.contains("FAILED") && line.contains(" - ") {
                if let Some(idx) = line.find(" - ") {
                    let test_part = line[..idx].trim();
                    let error_part = line[idx + 3..].trim();
                    if let Some(test_name) = test_part.split_whitespace().last() {
                        errors.insert(test_name.to_string(), error_part.to_string());
                    }
                }
            }
        }

        // Also look for common Python errors
        for (i, line) in lines.iter().enumerate() {
            if line.contains("ImportError:")
                || line.contains("ModuleNotFoundError:")
                || line.contains("SyntaxError:")
                || line.contains("NameError:")
                || line.contains("AttributeError:")
                || line.contains("TypeError:")
            {
                // Try to find which test this belongs to by looking backwards
                for j in (0..i).rev() {
                    if lines[j].contains("::")
                        && (lines[j].contains("test_") || lines[j].contains("Test"))
                    {
                        let test_name = lines[j].split_whitespace().next().unwrap_or("");
                        if !test_name.is_empty() && !errors.contains_key(test_name) {
                            errors.insert(test_name.to_string(), line.trim().to_string());
                        }
                        break;
                    }
                }
            }
        }

        errors
    }

    /// Find error message for a specific test.
    fn find_error_for_test(
        errors: &HashMap<String, String>,
        test_name: &str,
        output: &str,
    ) -> Option<String> {
        // Direct match
        if let Some(error) = errors.get(test_name) {
            return Some(error.clone());
        }

        // Partial match
        for (key, error) in errors {
            if key.contains(test_name) || test_name.contains(key.as_str()) {
                return Some(error.clone());
            }
        }

        // Look for collection errors (tests that couldn't even be collected)
        if output.contains("collected 0 items") {
            // Check for import errors in the output
            if output.contains("ImportError") {
                for line in output.lines() {
                    if line.contains("ImportError") {
                        return Some(format!("Collection failed: {}", line.trim()));
                    }
                }
            }
            if output.contains("SyntaxError") {
                for line in output.lines() {
                    if line.contains("SyntaxError") {
                        return Some(format!("Collection failed: {}", line.trim()));
                    }
                }
            }
            return Some("Test collection failed - check for import or syntax errors".to_string());
        }

        None
    }

    /// Find test result by name (handles various naming formats).
    fn find_test_result(results: &HashMap<String, bool>, test_name: &str) -> bool {
        // Direct match
        if let Some(&passed) = results.get(test_name) {
            return passed;
        }

        // Check for module-level failures (Django _FailedTest)
        // Test name like "test_utils.tests.TestClass.test_method" - check if "test_utils" module failed
        for (key, &_passed) in results {
            if key.starts_with("__module_fail__") {
                let failed_module = key.trim_start_matches("__module_fail__");
                // Check if the test_name starts with or contains this failed module
                if test_name.starts_with(failed_module)
                    || test_name.starts_with(&format!("{}.", failed_module))
                {
                    debug!(
                        "Test {} matched module failure for {}",
                        test_name, failed_module
                    );
                    return false;
                }
            }
        }

        // Partial match (test name might be part of the key)
        for (key, &passed) in results {
            // Skip module failure markers in partial matching
            if key.starts_with("__module_fail__") {
                continue;
            }
            if key.contains(test_name) || test_name.contains(key.as_str()) {
                return passed;
            }
        }

        // Default to failed if not found
        debug!(
            "Test {} not found in results, defaulting to failed",
            test_name
        );
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

    #[test]
    fn test_strip_ansi_codes() {
        // Test stripping color codes from pytest output
        let input = "test_foo.py::test_bar \x1b[32mPASSED\x1b[0m";
        let expected = "test_foo.py::test_bar PASSED";
        assert_eq!(DockerExecutor::strip_ansi_codes(input), expected);

        // Test with red color (failed)
        let input = "test_foo.py::test_baz \x1b[31mFAILED\x1b[0m";
        let expected = "test_foo.py::test_baz FAILED";
        assert_eq!(DockerExecutor::strip_ansi_codes(input), expected);

        // Test with multiple codes
        let input = "\x1b[1m\x1b[31mFAILED\x1b[0m test.py";
        let expected = "FAILED test.py";
        assert_eq!(DockerExecutor::strip_ansi_codes(input), expected);

        // Test with no codes
        let input = "plain text";
        assert_eq!(DockerExecutor::strip_ansi_codes(input), input);
    }

    #[test]
    fn test_parse_pytest_output_with_ansi() {
        // Simulate actual pytest output with ANSI codes
        let stdout = r#"
test_rst.py::test_read_normal [32mPASSED[0m
test_rst.py::test_with_header_rows [31mFAILED[0m
"#;
        // Replace with actual escape character
        let stdout = stdout
            .replace("[32m", "\x1b[32m")
            .replace("[31m", "\x1b[31m")
            .replace("[0m", "\x1b[0m");

        let cleaned = DockerExecutor::strip_ansi_codes(&stdout);
        assert!(cleaned.contains("test_read_normal PASSED"));
        assert!(cleaned.contains("test_with_header_rows FAILED"));
    }

    #[test]
    fn test_parse_django_test_output() {
        // Create a test instance with Django test runner
        let instance = SWEBenchInstance {
            instance_id: "django__django-11133".to_string(),
            repo: "django/django".to_string(),
            base_commit: "abc123".to_string(),
            problem_statement: "Test".to_string(),
            patch: "".to_string(),
            test_patch: "".to_string(),
            fail_to_pass: r#"["test_utils.tests.OverrideSettingsTests.test_override_file_upload_permissions"]"#.to_string(),
            pass_to_pass: "[]".to_string(),
            version: "3.0".to_string(),
            environment_setup_commit: "def456".to_string(),
            hints_text: None,
            created_at: None,
        };

        // Django test output format with passing test
        let stdout = r#"
Testing against Django installed in '/testbed/django'
System check identified no issues (0 silenced).
test_override_file_upload_permissions (test_utils.tests.OverrideSettingsTests) ... ok

----------------------------------------------------------------------
Ran 1 test in 0.001s

OK
"#;

        let (fail_to_pass_results, _) = DockerExecutor::parse_test_results(&instance, stdout, "");

        assert_eq!(fail_to_pass_results.len(), 1);
        assert!(
            fail_to_pass_results[0].passed,
            "Test should be marked as passed, got: {:?}",
            fail_to_pass_results[0]
        );
    }

    #[test]
    fn test_parse_django_test_output_with_module_failure() {
        // Create a test instance with Django test runner
        let instance = SWEBenchInstance {
            instance_id: "django__django-11133".to_string(),
            repo: "django/django".to_string(),
            base_commit: "abc123".to_string(),
            problem_statement: "Test".to_string(),
            patch: "".to_string(),
            test_patch: "".to_string(),
            fail_to_pass: r#"["test_utils.tests.OverrideSettingsTests.test_override_file_upload_permissions"]"#.to_string(),
            pass_to_pass: "[]".to_string(),
            version: "3.0".to_string(),
            environment_setup_commit: "def456".to_string(),
            hints_text: None,
            created_at: None,
        };

        // Django test output with module load failure
        let stdout = r#"
Testing against Django installed in '/testbed/django'
test_utils (unittest.loader._FailedTest) ... ERROR

======================================================================
ERROR: test_utils (unittest.loader._FailedTest)
----------------------------------------------------------------------
ImportError: Failed to import test module: test_utils
"#;

        let (fail_to_pass_results, _) = DockerExecutor::parse_test_results(&instance, stdout, "");

        assert_eq!(fail_to_pass_results.len(), 1);
        assert!(
            !fail_to_pass_results[0].passed,
            "Test should be marked as failed due to module load error"
        );
    }

    #[test]
    fn test_parse_django_test_output_mixed() {
        // Create a test instance with multiple tests
        let instance = SWEBenchInstance {
            instance_id: "django__django-11133".to_string(),
            repo: "django/django".to_string(),
            base_commit: "abc123".to_string(),
            problem_statement: "Test".to_string(),
            patch: "".to_string(),
            test_patch: "".to_string(),
            fail_to_pass: r#"["admin_views.tests.AdminViewBasicTest.test_login"]"#.to_string(),
            pass_to_pass: r#"["admin_views.tests.AdminViewBasicTest.test_logout"]"#.to_string(),
            version: "3.0".to_string(),
            environment_setup_commit: "def456".to_string(),
            hints_text: None,
            created_at: None,
        };

        // Django test output with mixed results
        let stdout = r#"
Testing against Django installed in '/testbed/django'
test_login (admin_views.tests.AdminViewBasicTest) ... ok
test_logout (admin_views.tests.AdminViewBasicTest) ... FAIL

----------------------------------------------------------------------
Ran 2 tests in 0.005s

FAILED (failures=1)
"#;

        let (fail_to_pass_results, pass_to_pass_results) =
            DockerExecutor::parse_test_results(&instance, stdout, "");

        assert_eq!(fail_to_pass_results.len(), 1);
        assert!(
            fail_to_pass_results[0].passed,
            "test_login should pass: {:?}",
            fail_to_pass_results[0]
        );

        assert_eq!(pass_to_pass_results.len(), 1);
        assert!(
            !pass_to_pass_results[0].passed,
            "test_logout should fail: {:?}",
            pass_to_pass_results[0]
        );
    }
}
