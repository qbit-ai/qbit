# Evaluation Methodology

This document provides a detailed, step-by-step breakdown of exactly how Qbit evaluates each SWE-bench instance.

## Overview

The evaluation follows a 5-step process for each instance:

```
[1/5] Setup Workspace     → Clone repo, apply test patch, protect test files
[2/5] Start Container     → Launch Docker testbed for agent testing
[3/5] Run Agent           → Agent works on the problem with test access
[4/5] Stop Container      → Clean up testbed container
[5/5] Final Evaluation    → Run official harness to grade the solution
```

## Step 1: Setup Workspace

### 1.1 Clone Repository

We clone the repository at the exact commit specified in the instance:

```rust
// From scenario.rs
let repo_manager = RepoManager::new()?;
let repo_path = repo_manager.setup_workspace(&self.instance, &workspace)?;
```

**Implementation Details:**
- Bare repository cache at `~/.qbit/benchmarks/swebench/repos/`
- Working copy created at `base_commit` (not HEAD)
- No access to commits after the issue was filed

### 1.2 Apply Test Patch

The test patch adds the FAIL_TO_PASS tests that verify the fix:

```rust
// From scenario.rs
if !self.instance.test_patch.is_empty() {
    let test_patch_path = repo_path.join(".swebench_test_patch.diff");
    std::fs::write(&test_patch_path, &self.instance.test_patch)?;

    // Apply using git apply
    std::process::Command::new("git")
        .args(["apply", "--whitespace=nowarn", ".swebench_test_patch.diff"])
        .current_dir(&repo_path)
        .output()?;
}
```

**Why this matters:** The agent needs to run the failing tests to understand the problem. Without the test patch, there would be no tests to verify the fix.

### 1.3 Protect Test Files

Test files are made read-only to prevent the agent from modifying them:

```rust
// From repo.rs
pub fn protect_test_files(&self, repo_path: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(repo_path) {
        if is_test_file(&path) {
            // Remove write permissions
            let mode = metadata.permissions().mode() & !0o222;
            std::fs::set_permissions(&path, Permissions::from_mode(mode))?;
            count += 1;
        }
    }
    Ok(count)
}
```

**Test file patterns:**
- `tests/*`, `test/*`, `*/tests/*`, `*/test/*`
- `test_*.py`, `*_test.py`

## Step 2: Start Docker Container

### 2.1 Find Docker Image

We try multiple image sources in order:

```rust
// From types.rs
pub fn docker_image_alternatives(&self) -> Vec<String> {
    vec![
        // Primary: Epoch AI optimized images (native architecture)
        format!("ghcr.io/epoch-research/swe-bench.eval.{}.{}",
                native_arch, self.instance_id),
        // Fallback 1: Epoch AI images (emulated architecture)
        format!("ghcr.io/epoch-research/swe-bench.eval.{}.{}",
                emulated_arch, self.instance_id),
        // Fallback 2: Official SWE-bench images
        format!("swebench/sweb.eval.{}", self.instance_id),
    ]
}
```

### 2.2 Container Configuration

```rust
// From docker.rs
let host_config = HostConfig {
    mounts: Some(vec![Mount {
        target: Some("/workspace".to_string()),
        source: Some(workspace_abs.to_string_lossy().to_string()),
        typ: Some(MountTypeEnum::BIND),
        read_only: Some(false),
        ..Default::default()
    }]),
    memory: Some(4 * 1024 * 1024 * 1024),  // 4GB RAM limit
    memory_swap: Some(4 * 1024 * 1024 * 1024),
    nano_cpus: Some(2_000_000_000),         // 2 CPUs
    ..Default::default()
};
```

### 2.3 Apply Test Patch to Container

The test patch is also applied inside the container's `/testbed` directory:

```rust
// From docker.rs
pub async fn apply_test_patch_to_container(
    &self,
    container_name: &str,
    test_patch: &str,
) -> Result<()> {
    // Base64 encode to safely pass through shell
    let patch_b64 = base64::encode(test_patch.as_bytes());

    let apply_cmd = format!(r#"
cd /testbed
echo '{}' | base64 -d > /tmp/test_patch.diff
git apply --whitespace=nowarn /tmp/test_patch.diff
"#, patch_b64);
    // ... execute in container
}
```

## Step 3: Run Agent

### 3.1 Agent Prompt

The agent receives a structured prompt (from `scenario.rs`):

```markdown
You are fixing a software engineering issue from the SWE-bench benchmark.

## Repository
- Repository: {repo}
- Version: {version}

## Problem Statement
{problem_statement}

## Tests to Fix
The following tests currently fail and must pass after your fix:
- `{test_name_1}`
- `{test_name_2}`

## Success Criteria
1. All tests listed above pass
2. No other tests regress
3. Only necessary files were modified

## Constraints
- Do not modify test files (they are read-only)
- Do not refactor or "improve" unrelated code
```

### 3.2 Available Tool: `run_swebench_test`

The agent can run tests via a restricted tool:

```json
{
    "name": "run_swebench_test",
    "description": "Run tests in the SWE-bench Docker test environment",
    "parameters": {
        "test_path": {
            "type": "string",
            "description": "Test file, function, or pattern"
        },
        "verbose": {
            "type": "boolean",
            "description": "Enable verbose output"
        }
    }
}
```

**Why a custom tool?** To prevent the agent from accessing git history (which contains the fix commits) via direct `docker exec` commands.

### 3.3 Test Execution Flow

When the agent calls `run_swebench_test`:

```bash
# 1. Sync changes from /workspace/repo to /testbed (excluding test files)
cd /workspace/repo
for file in $(git diff --name-only HEAD); do
    if ! is_test_file "$file"; then
        cp "$file" "/testbed/$file"
    fi
done

# 2. Run test from /testbed
cd /testbed
{test_command} {test_path}
```

**Test file exclusion is critical:** The agent's changes to source files are synced, but test files are never synced. This prevents any attempt to modify tests.

## Step 4: Stop Container

After agent completion, the testbed container is removed:

```rust
// From scenario.rs
if let Some(ref name) = container_name {
    let _ = docker.stop_container(name).await;
}
```

## Step 5: Final Evaluation

### 5.1 Official SWE-bench Harness

When the `swebench` Python package is installed, we use the official harness:

```rust
// From harness.rs
let output = Command::new(&python)
    .args([
        "-m",
        "swebench.harness.run_evaluation",
        "-id", run_id,
        "-p", predictions_path.to_str().unwrap(),
        "-d", "princeton-nlp/SWE-bench_Lite",
        "--report_dir", results_dir.to_str().unwrap(),
        "-t", "600",  // 10 minute timeout
        "-i", &instance.instance_id,
        "--max_workers", "1",
    ])
    .output()?;
```

### 5.2 Prediction Format

The agent's patch is formatted as a prediction:

```json
{
    "instance_id": "django__django-11133",
    "model_name_or_path": "qbit-agent",
    "model_patch": "diff --git a/django/http/response.py..."
}
```

### 5.3 Fallback Evaluation

If the official harness isn't available, we use our Docker executor:

```rust
// From harness.rs
pub async fn run_fallback_evaluation(
    instance: &SWEBenchInstance,
    workspace: &Path,
) -> Result<HarnessResult> {
    let docker = DockerExecutor::new()?;
    let test_result = docker.run_tests(instance, workspace).await?;

    Ok(HarnessResult {
        resolved: test_result.is_solved(),
        // ...
    })
}
```

Both methods apply the same criteria: all FAIL_TO_PASS must pass, no regressions allowed.

### 5.4 Result Determination

```rust
// From types.rs
impl TestExecutionResult {
    pub fn is_solved(&self) -> bool {
        self.fail_to_pass_success() && self.pass_to_pass_success()
    }

    pub fn fail_to_pass_success(&self) -> bool {
        !self.fail_to_pass_results.is_empty()
            && self.fail_to_pass_results.iter().all(|r| r.passed)
    }

    pub fn pass_to_pass_success(&self) -> bool {
        self.pass_to_pass_results.iter().all(|r| r.passed)
    }
}
```

## Result Categories

| Result | Criteria | Counts as Solved |
|--------|----------|------------------|
| **Solved** | All F2P pass, all P2P pass | ✓ Yes |
| **Partial** | Some F2P pass, no regressions | ✗ No |
| **Failed** | F2P failures or regressions | ✗ No |
| **Error** | Docker/timeout/execution error | ✗ No |
| **Skipped** | No Docker image available | Not counted |

## Repository-Specific Test Commands

Different repositories use different test runners:

| Repository | Test Command |
|------------|--------------|
| `django/django` | `./tests/runtests.py --verbosity 2 --settings=test_sqlite --parallel 1` |
| `astropy/astropy` | `pytest -rA -vv -o console_output_style=classic --tb=no` |
| `sphinx-doc/sphinx` | `tox --current-env -epy39 -v --` |
| `sympy/sympy` | `bin/test -C --verbose` |
| Default | `pytest --no-header -rA --tb=no -p no:cacheprovider` |

These match the official [SWE-bench MAP_REPO_VERSION_TO_SPECS](https://github.com/SWE-bench/SWE-bench/blob/main/swebench/harness/constants/python.py).

## Timeline of a Single Evaluation

```
T+0s     - Clone repository at base_commit
T+2s     - Apply test patch to workspace
T+3s     - Protect test files (read-only)
T+5s     - Pull Docker image (if not cached)
T+30s    - Start testbed container
T+35s    - Apply test patch to container /testbed
T+40s    - Start agent execution
           ↳ Agent reads problem statement
           ↳ Agent explores codebase
           ↳ Agent runs failing tests
           ↳ Agent makes changes
           ↳ Agent verifies fix
T+5-20m  - Agent completes
T+X      - Stop testbed container
T+X+5s   - Generate patch from workspace
T+X+10s  - Run official harness evaluation
T+X+30s  - Parse results, record outcome
```

## Verification Points

Anyone can verify:

1. **Dataset Source** - Check HuggingFace `princeton-nlp/SWE-bench_Lite`
2. **Docker Images** - Pull and inspect Epoch AI images
3. **Patch Generation** - `git diff HEAD` in workspace after agent run
4. **Test Execution** - Re-run official harness with same patch
5. **Agent Transcript** - Review full tool calls and responses
