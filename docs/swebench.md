# SWE-bench Lite Integration

SWE-bench Lite is a benchmark of 300 real GitHub issues from popular Python repositories. It tests an AI agent's ability to understand codebases, locate bugs, and implement fixes that pass the project's test suite.

## Overview

Each SWE-bench instance contains:
- A GitHub repository and commit to check out
- A problem statement (the original issue description)
- Tests that should pass after the fix (`FAIL_TO_PASS`)
- Tests that should continue passing (`PASS_TO_PASS`)

The agent is given the problem statement and must:
1. Explore the repository to understand the codebase
2. Identify the files that need modification
3. Implement a fix
4. Ensure the fix passes all tests without regressions

## Prerequisites

- **Docker**: Required for running tests in isolated containers
  ```bash
  # Verify Docker is running
  docker ps
  ```
- **Disk space**: ~20GB for repository caches and Docker images
- **Network**: Initial run downloads dataset and Docker images

## CLI Arguments

### Core Arguments

| Argument | Description |
|----------|-------------|
| `--swebench` | Enable SWE-bench benchmark mode |
| `--instance <ID>` | Run a specific instance (e.g., `django__django-11133`) |
| `--problems <RANGE>` | Filter instances by index (e.g., `0-9`, `0,5,10`) |

### Execution Options

| Argument | Default | Description |
|----------|---------|-------------|
| `--parallel` | off | Run instances concurrently |
| `--concurrency <N>` | 4 | Max concurrent instances when parallel |
| `--eval-provider <PROVIDER>` | vertex-claude | LLM provider (`vertex-claude`, `openai`, `zai`) |
| `--eval-model <MODEL>` | provider default | Override model (e.g., `claude-opus-4-5@20250929`) |

### Output Options

| Argument | Description |
|----------|-------------|
| `--json` | Output results as JSON |
| `--pretty` | CI-friendly formatted summary |
| `--output <FILE>` | Save summary JSON to file |
| `--results-dir <DIR>` | Save per-instance detailed JSON results |
| `--transcript` | Print full agent transcript before results |
| `-v, --verbose` | Show verbose output (debug info) |

### Debugging Options

| Argument | Description |
|----------|-------------|
| `--workspace-dir <DIR>` | Use persistent workspace (survives between runs) |
| `--test-only` | Skip agent, only run Docker tests (requires `--workspace-dir`) |

## Examples

### Run a Single Instance

Test with one instance to verify setup:

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --instance django__django-11133
```

### Run First 10 Instances

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --problems 0-9
```

### Run in Parallel with Concurrency Limit

Run 50 instances with max 4 concurrent:

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --problems 0-49 --parallel --concurrency 4
```

### Use a Specific Model

Run with Claude Opus 4.5 on Vertex AI:

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --instance django__django-11133 \
    --eval-provider vertex-claude \
    --eval-model claude-opus-4-5@20250929
```

### Save Detailed Results for Analysis

Save per-instance JSON files for post-hoc analysis:

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --problems 0-49 --parallel \
    --results-dir ./swebench-results \
    --output ./swebench-summary.json
```

Each instance gets a JSON file in `./swebench-results/` containing:
- Full agent transcript and tool calls
- Test execution stdout/stderr
- FAIL_TO_PASS and PASS_TO_PASS test results
- Modified files list
- Token usage and timing

### Debug a Failing Instance

Use persistent workspace to debug without re-running the agent:

```bash
# First run - saves workspace
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --instance django__django-11133 \
    --workspace-dir ./debug-workspace

# Later - re-run just the tests after manual changes
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --instance django__django-11133 \
    --workspace-dir ./debug-workspace \
    --test-only
```

### CI Integration

For CI pipelines, use JSON output and save results:

```bash
cargo run --no-default-features --features evals,cli --bin qbit-cli -- \
    --swebench --problems 0-299 \
    --parallel --concurrency 8 \
    --json \
    --output ./results/swebench-$(date +%Y%m%d).json \
    --results-dir ./results/instances/
```

## Evaluation Criteria

An instance is considered **SOLVED** when:
1. All `FAIL_TO_PASS` tests pass (the fix works)
2. All `PASS_TO_PASS` tests still pass (no regressions)

An instance **FAILS** if:
- Any `FAIL_TO_PASS` test still fails
- Any `PASS_TO_PASS` test regresses (was passing, now failing)

## Output Format

### Terminal Output

```
Running swebench benchmark (10 instances)
Real GitHub issues from Python repositories (300 total in Lite)
Provider: vertex-claude

  [1/4] Setting up workspace at commit abc1234...
  [2/4] Running agent...
  [3/4] Agent modified 2 files
  [4/4] Running tests in Docker...

PASS django__django-11133 (45230ms)
  ✓ swebench-tests
```

### JSON Output Structure

Summary (`--output`):
```json
{
  "total": 10,
  "passed": 7,
  "failed": 3,
  "pass_rate": 0.7,
  "total_duration_ms": 450000,
  "scenarios": [...]
}
```

Per-instance (`--results-dir`):
```json
{
  "scenario": "django__django-11133",
  "passed": true,
  "duration_ms": 45230,
  "metrics": [...],
  "agent_output": {
    "response": "...",
    "tool_calls": [...],
    "files_modified": ["django/http/response.py"],
    "tokens_used": 15420
  },
  "extra": {
    "instance_id": "django__django-11133",
    "test_execution": {
      "fail_to_pass": {"passed": 1, "total": 1, "tests": [...]},
      "pass_to_pass": {"passed": 5, "total": 5, "tests": [...]},
      "stdout": "...",
      "stderr": "..."
    }
  }
}
```

## Timeouts

| Operation | Timeout |
|-----------|---------|
| Total scenario | 30 min |
| Agent execution | 15 min |
| Docker test execution | 10 min |
| Docker image pull | 5 min |

## Troubleshooting

### "Docker is not available"

Ensure Docker daemon is running:
```bash
docker ps  # Should list containers, not error
```

### "No module named pytest" in test output

This indicates the Docker container's conda environment wasn't activated properly. The test command should activate the `testbed` environment automatically.

### Tests show "collected 0 items"

The test patch may not have been applied. Check that the instance has a valid `test_patch` field. Use `--verbose` to see detailed logs.

### Instance takes too long

Some instances have large repositories or many tests. Use `--concurrency 2` to reduce parallel load, or run problematic instances sequentially.

### Out of disk space

Clear cached repositories:
```bash
rm -rf ~/.qbit/benchmarks/swebench/repos/
```

Clear Docker images:
```bash
docker system prune -a
```

## Repository Cache

Repositories are cached at `~/.qbit/benchmarks/swebench/repos/` to avoid re-cloning. Each repository is cloned once and then worktrees are created for each instance's specific commit.

## Docker Images

SWE-bench uses Epoch AI's optimized Docker images which are ~10x smaller than the official images:
- `ghcr.io/epoch-research/swe-bench.eval.x86_64.<instance_id>`
- `ghcr.io/epoch-research/swe-bench.eval.arm64.<instance_id>`

Images are pulled automatically on first use and cached by Docker.

### Missing Images

Not all SWE-bench instances have pre-built Docker images available from Epoch AI. When an image is missing, the instance is automatically **skipped** rather than failing. Check for `"status": "skip"` in JSON output.

### Building Images Locally

If you need to run instances without pre-built images, you can build them using the official SWE-bench harness:

```bash
# Install SWE-bench
pip install swebench

# Build images locally (--namespace '' forces local build)
python -m swebench.harness.run_evaluation \
    --predictions_path /path/to/predictions.json \
    --swe_bench_tasks princeton-nlp/SWE-bench_Lite \
    --namespace '' \
    --run_id my_run

# Or use the docker build scripts directly
git clone https://github.com/aorwall/SWE-bench-docker
cd SWE-bench-docker
./build.sh <namespace> <instance_id>
```

**Storage requirements:**
- Base + environment images: ~100GB
- Full SWE-bench (all 2290 instances): ~684GB unoptimized
- Epoch AI optimized registry: ~67GB total

See [SWE-bench Docker Setup](https://www.swebench.com/SWE-bench/guides/docker_setup/) for detailed instructions.
