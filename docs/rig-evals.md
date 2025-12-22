# Rust Evaluation Framework

This guide explains the Rust-native evaluation framework for testing Qbit agent capabilities using the `rig` library.

## Overview

The Rust evals framework provides end-to-end testing of the Qbit agent through 5 evaluation scenarios that test real-world software engineering tasks. Unlike the Python/DeepEval approach, this framework:

- Runs entirely in Rust (no Python dependencies)
- Uses a lightweight agent executor (no PTY/sidecar overhead)
- Integrates with the CLI via feature flags
- Uses Vertex Claude Haiku for both agent execution and LLM-based evaluation

## Prerequisites

### Environment Variables

```bash
# Required for Vertex AI
export VERTEX_AI_PROJECT_ID=your-project-id
export VERTEX_AI_LOCATION=us-east5  # or your preferred region
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
```

Or configure in your `.env` file at the project root.

### Build Requirements

The evals feature requires the `local-tools` feature (included automatically):

```bash
cargo build --no-default-features --features evals --bin qbit-cli
```

## Quick Start

```bash
# List available scenarios
cargo run --features evals --bin qbit-cli -- --list-scenarios

# Run all scenarios
cargo run --features evals --bin qbit-cli -- --eval

# Run a specific scenario
cargo run --features evals --bin qbit-cli -- --eval --scenario bug-fix

# JSON output (for CI integration)
cargo run --features evals --bin qbit-cli -- --eval --json
```

## Scenarios

### 1. Bug Fix (`bug-fix`)

Tests the agent's ability to fix compile errors.

**Task:** Fix a type mismatch error in a Rust function that returns `i32` instead of `String`.

**Metrics:**
- `cargo_check` - Code compiles after fix
- `lib_modified` - The lib.rs file was modified
- `fix_quality` - LLM judges the fix is correct and idiomatic

### 2. Feature Implementation (`feature-impl`)

Tests adding new functionality to existing code.

**Task:** Implement a `reverse()` method for a `StringUtils` struct.

**Metrics:**
- `cargo_test` - All tests pass (including new tests)
- `has_reverse_method` - The method exists in the file
- `implementation_quality` - LLM judges implementation quality

### 3. Refactoring (`refactor`)

Tests extracting and reorganizing code.

**Task:** Extract email validation logic from a `User::new()` method into a separate `validate_email()` function.

**Metrics:**
- `cargo_test` - Tests still pass after refactor
- `has_validate_fn` - The validate function exists
- `code_quality` - LLM scores code quality (0-10 scale)

### 4. Code Understanding (`code-understanding`)

Tests the agent's ability to read and explain code.

**Task:** Explain a binary heap implementation, including its time complexity.

**Metrics:**
- `identifies_heap` - LLM judges explanation identifies heap data structure
- `correct_complexity` - LLM judges complexity analysis is correct
- `explains_heapify` - LLM judges heapify operation is explained

### 5. Multi-Step Workflow (`multi-step`)

Tests complex multi-tool workflows.

**Task:** Create a new `utils` module with an `is_palindrome()` function, add tests, and verify they pass.

**Metrics:**
- `utils_module` - `src/utils.rs` exists
- `test_file` - `tests/utils_test.rs` exists
- `mod_declaration` - `mod utils` in lib.rs
- `has_is_palindrome` - Function exists in utils.rs
- `cargo_test` - All tests pass

## Metrics

### CodeCorrectnessMetric

Verifies code correctness by running cargo commands:

```rust
// Check compilation
CodeCorrectnessMetric::cargo_check()

// Run tests
CodeCorrectnessMetric::cargo_test()
```

### FileStateMetric

Checks file existence and content:

```rust
// File exists
FileStateMetric::exists("metric_name", "path/to/file.rs")

// File contains text
FileStateMetric::contains("metric_name", "path/to/file.rs", "fn my_function")
```

### LlmJudgeMetric

Uses an LLM to make pass/fail judgments:

```rust
LlmJudgeMetric::new(
    "fix_quality",
    "The fix correctly changes the return type to String and uses .to_string() or format!",
    0.7,  // threshold (unused, judges binary pass/fail)
)

// Or with default threshold
LlmJudgeMetric::with_criteria("metric_name", "evaluation criteria")
```

### LlmScoreMetric

Uses an LLM to score output on a numeric scale:

```rust
LlmScoreMetric::new(
    "code_quality",
    "Rate the code quality: readability, proper error handling, idiomatic Rust",
    7.0,  // minimum passing score
    10.0, // maximum score
)

// Convenience for 0-10 scale
LlmScoreMetric::scale_10("quality", "criteria", 7.0)
```

## Architecture

```
backend/src/evals/
├── mod.rs                    # Module exports
├── runner.rs                 # EvalRunner (testbed setup, orchestration)
├── executor.rs               # Lightweight agent executor (Vertex Haiku)
├── outcome.rs                # EvalReport, MetricOutcome, EvalSummary
├── metrics/
│   ├── mod.rs               # Metric trait and MetricResult enum
│   ├── code_correctness.rs  # Cargo check/test metrics
│   ├── file_state.rs        # File existence/content metrics
│   └── llm_judge.rs         # LLM-based evaluation metrics
└── scenarios/
    ├── mod.rs               # Scenario trait and registry
    ├── bug_fix.rs           # Bug fix scenario
    ├── feature_impl.rs      # Feature implementation scenario
    ├── refactor.rs          # Refactoring scenario
    ├── code_understanding.rs # Code explanation scenario
    └── multi_step.rs        # Multi-step workflow scenario
```

### Execution Flow

1. **Testbed Setup:** Scenario files are embedded in Rust code and copied to a temp directory
2. **Agent Execution:** Lightweight executor runs the prompt with minimal tools
3. **Metric Evaluation:** Each metric evaluates the workspace state and/or agent output
4. **Reporting:** Results are printed to terminal or output as JSON

### Lightweight Executor

The eval executor (`executor.rs`) is a minimal agent loop that:

- Uses Vertex Claude Haiku (`claude-haiku-4-5@20251001`) for speed
- Auto-approves all tool calls (no HITL)
- Runs up to 50 iterations before stopping
- Has access to: read, write, edit, grep, list, shell tools
- Tracks tool calls and modified files

## Adding New Scenarios

### 1. Create the Scenario File

```rust
// backend/src/evals/scenarios/my_scenario.rs

use async_trait::async_trait;
use crate::evals::metrics::{CodeCorrectnessMetric, FileStateMetric, LlmJudgeMetric, Metric};
use crate::evals::scenarios::Scenario;

pub struct MyScenario;

#[async_trait]
impl Scenario for MyScenario {
    fn name(&self) -> &str {
        "my-scenario"
    }

    fn description(&self) -> &str {
        "Short description of what this tests"
    }

    fn testbed(&self) -> &str {
        "rust-my-scenario"  // Must match get_testbed_content()
    }

    fn prompt(&self) -> &str {
        "The task for the agent to complete..."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(CodeCorrectnessMetric::cargo_check()),
            Box::new(FileStateMetric::exists("file_exists", "src/lib.rs")),
            Box::new(LlmJudgeMetric::with_criteria(
                "quality",
                "The solution is correct and well-implemented",
            )),
        ]
    }
}

/// Testbed files (embedded in binary)
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        ("Cargo.toml".to_string(), r#"[package]
name = "my-testbed"
version = "0.1.0"
edition = "2021"
"#.to_string()),
        ("src/lib.rs".to_string(), r#"// Starting code here
"#.to_string()),
    ]
}
```

### 2. Register the Scenario

In `backend/src/evals/scenarios/mod.rs`:

```rust
mod my_scenario;
pub use my_scenario::MyScenario;

pub fn all_scenarios() -> Vec<Box<dyn Scenario>> {
    vec![
        // ... existing scenarios
        Box::new(MyScenario),
    ]
}
```

### 3. Register the Testbed

In `backend/src/evals/runner.rs`:

```rust
fn get_testbed_content(name: &str) -> Result<Vec<(String, String)>> {
    match name {
        // ... existing testbeds
        "rust-my-scenario" => Ok(scenarios::my_scenario::testbed_files()),
        _ => anyhow::bail!("Unknown testbed: {}", name),
    }
}
```

## CI Integration

### GitHub Actions Workflow

The evals can be triggered manually via the GitHub Actions workflow (`.github/workflows/evals.yml`):

```yaml
name: Evals

on:
  workflow_dispatch:
    inputs:
      scenario:
        description: 'Specific scenario to run (leave empty for all)'
        required: false
        default: ''
```

Required secrets/variables:
- `VERTEX_AI_PROJECT_ID` (variable)
- `VERTEX_AI_LOCATION` (variable)
- `GOOGLE_APPLICATION_CREDENTIALS` (secret - base64 encoded JSON)

### JSON Output Format

```json
{
  "total": 5,
  "passed": 5,
  "failed": 0,
  "pass_rate": 1.0,
  "total_duration_ms": 89275,
  "scenarios": [
    {
      "scenario": "bug-fix",
      "passed": true,
      "duration_ms": 11122,
      "metrics": [
        {"name": "cargo_check", "status": "pass"},
        {"name": "lib_modified", "status": "pass"},
        {"name": "fix_quality", "status": "pass"}
      ]
    }
  ]
}
```

## Troubleshooting

### "VERTEX_AI_PROJECT_ID not set"

Ensure environment variables are set:

```bash
export VERTEX_AI_PROJECT_ID=your-project
export VERTEX_AI_LOCATION=us-east5
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/creds.json
```

### Scenario Fails with File Not Found

Check that:
1. The testbed name matches in both `testbed()` and `get_testbed_content()`
2. The testbed files include all necessary paths
3. Relative paths in metrics match the testbed structure

### LLM Judge Returns Unexpected Results

- LLM judges use `temperature: 0.0` for determinism
- Check the criteria are clear and specific
- View the agent's response in verbose mode to debug

### Agent Gets Stuck or Times Out

The executor has a 50-iteration limit. If tasks are too complex:
1. Break them into smaller scenarios
2. Make the prompt more explicit with step-by-step instructions
3. Check if the testbed has all necessary files

## Performance

Typical run times on Vertex AI (Claude Haiku):

| Scenario | Duration |
|----------|----------|
| bug-fix | ~10s |
| feature-impl | ~11s |
| refactor | ~25s |
| code-understanding | ~9s |
| multi-step | ~25s |
| **Total** | **~80-90s** |

LLM judge metrics add ~1-2s per metric for the evaluation call.

## Cost Considerations

Using Claude Haiku on Vertex AI:
- Agent execution: ~$0.001-0.005 per scenario
- LLM judge metrics: ~$0.0005 per metric
- Full suite (5 scenarios): ~$0.02-0.03

This is significantly cheaper than using Sonnet or Opus for evals.
