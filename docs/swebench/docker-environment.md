# Docker Environment

This document details the Docker container setup used for SWE-bench evaluation.

## Why Docker?

Docker provides:
1. **Isolation** - Tests run in a clean environment
2. **Reproducibility** - Same environment every time
3. **Correct Dependencies** - Each repo has specific Python/package versions
4. **Consistency** - Matches official SWE-bench evaluation

## Image Sources

We use Docker images in this priority order:

### 1. Epoch AI Optimized Images (Primary)

```
ghcr.io/epoch-research/swe-bench.eval.{arch}.{instance_id}
```

**Architecture-specific:**
- ARM64 (Apple Silicon): `ghcr.io/epoch-research/swe-bench.eval.arm64.django__django-11133`
- x86_64 (Intel/AMD): `ghcr.io/epoch-research/swe-bench.eval.x86_64.django__django-11133`

**Why Epoch AI images?**
- ~10x smaller than official images
- Pre-built for multiple architectures
- Same test environment as official

### 2. Cross-Architecture Fallback

If native architecture isn't available, we try the other architecture with emulation:

```rust
// From types.rs
#[cfg(target_arch = "aarch64")]
let (native_arch, emulated_arch) = ("arm64", "x86_64");
#[cfg(not(target_arch = "aarch64"))]
let (native_arch, emulated_arch) = ("x86_64", "arm64");
```

### 3. Official SWE-bench Images

```
swebench/sweb.eval.{instance_id}
```

From DockerHub, maintained by the SWE-bench team.

## Image Selection Logic

```rust
// From docker.rs
async fn find_or_pull_image(&self, instance: &SWEBenchInstance) -> Result<Option<String>> {
    // 1. Check if any image is already local
    if let Some(image) = self.find_available_image(instance).await {
        return Ok(Some(image));  // Use cached image
    }

    // 2. Try to pull each alternative
    for image in instance.docker_image_alternatives() {
        if self.try_pull_image(&image).await? {
            return Ok(Some(image));
        }
    }

    Ok(None)  // No image available
}
```

## Container Configuration

### Resource Limits

```rust
// From docker.rs
let host_config = HostConfig {
    memory: Some(4 * 1024 * 1024 * 1024),      // 4 GB RAM
    memory_swap: Some(4 * 1024 * 1024 * 1024), // No swap
    nano_cpus: Some(2_000_000_000),             // 2 CPU cores
    // ...
};
```

| Resource | Limit | Rationale |
|----------|-------|-----------|
| Memory | 4 GB | Sufficient for large test suites |
| CPU | 2 cores | Prevents runaway processes |
| Swap | 4 GB | Equal to memory (no overcommit) |

### Workspace Mount

```rust
// From docker.rs
let host_config = HostConfig {
    mounts: Some(vec![Mount {
        target: Some("/workspace".to_string()),
        source: Some(workspace_abs.to_string_lossy().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(false),  // Agent needs write access
        ..Default::default()
    }]),
    // ...
};
```

**Mount structure:**
```
Host: ~/.qbit/workspaces/{session_id}/{instance_id}/
  └── repo/                    # Repository with agent's changes
      ├── src/
      ├── tests/
      └── ...

Container: /workspace/
  └── repo/                    # Mounted from host
```

### Environment Variables

```rust
env: Some(vec![
    "PYTHONDONTWRITEBYTECODE=1".to_string(),  // No .pyc files
    "PYTHONUNBUFFERED=1".to_string(),         // Real-time output
]),
```

## Container Lifecycle

### 1. Testbed Container (For Agent Work)

A long-running container for the agent to run tests:

```rust
// From docker.rs
pub async fn start_testbed_container(
    &self,
    instance: &SWEBenchInstance,
    workspace: &Path,
) -> Result<String> {
    let config = Config {
        image: Some(image),
        cmd: Some(vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            "sleep infinity".to_string(),  // Keep container alive
        ]),
        working_dir: Some("/workspace/repo".to_string()),
        // ...
    };

    // Create and start
    let container = self.client.create_container(create_options, config).await?;
    self.client.start_container(&container.id, None).await?;

    Ok(container_name)
}
```

### 2. Evaluation Container (For Final Tests)

A one-shot container for final evaluation:

```rust
// From docker.rs
pub async fn run_tests(
    &self,
    instance: &SWEBenchInstance,
    workspace: &Path,
) -> Result<TestExecutionResult> {
    let config = Config {
        image: Some(image),
        cmd: Some(vec![
            "/bin/bash".to_string(),
            "-c".to_string(),
            test_cmd,  // Run tests and exit
        ]),
        // ...
    };

    // Wait for completion with timeout
    tokio::time::timeout(
        Duration::from_secs(self.test_timeout_secs),  // 10 minutes
        self.wait_for_container(&container.id),
    ).await
}
```

## Timeouts

| Operation | Default | Configurable |
|-----------|---------|--------------|
| Image Pull | 5 minutes | `with_pull_timeout(secs)` |
| Test Execution | 10 minutes | `with_test_timeout(secs)` |

```rust
// From docker.rs
const DEFAULT_TEST_TIMEOUT_SECS: u64 = 600;   // 10 minutes
const DEFAULT_PULL_TIMEOUT_SECS: u64 = 300;   // 5 minutes

// Custom timeouts
let executor = DockerExecutor::new()?
    .with_test_timeout(900)    // 15 minutes
    .with_pull_timeout(600);   // 10 minutes
```

## What's Inside the Container

Each Epoch AI container includes:

```
/opt/miniconda3/          # Conda installation
  └── envs/testbed/       # Pre-configured environment
      └── python          # Correct Python version

/testbed/                 # Pre-cloned repository
  ├── setup.py            # Already installed in editable mode
  ├── src/
  └── tests/
```

### Conda Environment Activation

All commands run with the testbed environment active:

```rust
// From tools.rs
let full_command = format!(
    "source /opt/miniconda3/etc/profile.d/conda.sh && \
     conda activate testbed && {}",
    command
);
```

## File Synchronization

### During Agent Work

The agent modifies files in `/workspace/repo` (mounted from host). Before running tests, changes are synced to `/testbed`:

```bash
# From tools.rs - build_test_command
cd /workspace/repo

# Function to check if a file is a test file
is_test_file() {
    local file="$1"
    case "$file" in
        tests/*|test/*|*/tests/*|*/test/*|test_*.py|*_test.py)
            return 0  # Is a test file
            ;;
        *)
            return 1  # Not a test file
            ;;
    esac
}

# Sync non-test files to /testbed
for file in $(git diff --name-only HEAD); do
    if [ -f "$file" ]; then
        if is_test_file "$file"; then
            continue  # NEVER sync test files
        fi
        mkdir -p "/testbed/$(dirname "$file")"
        cp "$file" "/testbed/$file"
    fi
done

cd /testbed
{test_command} {test_path}
```

**Critical:** Test files are NEVER synced. This prevents any attempt by the agent to modify tests.

### Test Patch Application

The test patch (which adds FAIL_TO_PASS tests) is applied directly to `/testbed`:

```rust
// From docker.rs
pub async fn apply_test_patch_to_container(
    &self,
    container_name: &str,
    test_patch: &str,
) -> Result<()> {
    // Base64 encode for safe shell passing
    let patch_b64 = base64::encode(test_patch.as_bytes());

    let apply_cmd = format!(r#"
cd /testbed
echo '{}' | base64 -d > /tmp/test_patch.diff
git apply --whitespace=nowarn /tmp/test_patch.diff
rm /tmp/test_patch.diff
"#, patch_b64);

    // Execute in container
}
```

## Error Handling

### Image Not Available

When no Docker image exists for an instance:

```rust
// From docker.rs
if image.is_none() {
    anyhow::bail!(
        "IMAGE_NOT_AVAILABLE: No Docker image available for instance {}",
        instance.instance_id
    );
}
```

This results in a "skipped" result, not a failure:

```rust
// From scenario.rs
if err_msg.contains("IMAGE_NOT_AVAILABLE") {
    return Ok(self.create_skip_report(
        &agent_output,
        duration_ms,
        "Docker image not available for this instance",
    ));
}
```

### Container Timeout

If tests exceed the timeout:

```rust
// From docker.rs
Err(_) => {
    warn!("Container execution timed out");
    // Kill the runaway container
    let _ = self.client.kill_container::<String>(&container.id, None).await;
    -1  // Return error exit code
}
```

## Verifying Container Environment

You can inspect the container environment:

```bash
# Pull an image
docker pull ghcr.io/epoch-research/swe-bench.eval.arm64.django__django-11133

# Run interactively
docker run -it --rm ghcr.io/epoch-research/swe-bench.eval.arm64.django__django-11133 bash

# Inside container:
source /opt/miniconda3/etc/profile.d/conda.sh
conda activate testbed
python --version  # Should match instance's Python version
pip list          # Should have correct dependencies
cd /testbed
./tests/runtests.py --help
```

## Security Considerations

1. **No network access** - Containers run with default network (no special privileges)
2. **No privileged mode** - Standard container isolation
3. **Read-only test files** - Protected at host level before mounting
4. **Limited resources** - Memory and CPU caps prevent DoS
5. **Cleanup** - Containers are removed after use

## Troubleshooting

### Docker Not Running

```bash
# Check Docker status
docker info

# Start Docker (macOS)
open -a Docker

# Start Docker (Linux)
sudo systemctl start docker
```

### Image Pull Fails

```bash
# Check registry connectivity
docker pull ghcr.io/epoch-research/swe-bench.eval.arm64.django__django-11133

# Try official images
docker pull swebench/sweb.eval.django__django-11133

# Check disk space
docker system df
```

### Container Runs Out of Memory

```bash
# Increase Docker's memory limit (Docker Desktop settings)
# Or modify resource limits in code
```
