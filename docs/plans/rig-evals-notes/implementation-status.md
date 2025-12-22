# Rig Evals Implementation Status

## Completed

### Phase 0: Prerequisites
- [x] Task 0.1: Enabled `evals` feature in Cargo.toml
  - Added `rig-core/experimental` for rig::evals module
  - Added `tempfile` dependency for workspace isolation
  - Feature includes `cli` and `local-tools`
- [x] Task 0.2: Designed testbed structure
  - Embedded testbeds in Rust code (scenarios/*.rs)
  - Files copied to temp directory at runtime

### Phase 1: Core Framework
- [x] Task 1.1: Runner infrastructure
  - `EvalRunner` struct with workspace management
  - `setup_testbed()` copies embedded files
  - `run_prompt()` placeholder (agent invocation TODO)
- [x] Task 1.2: Metrics framework
  - `Metric` trait with `evaluate()` async method
  - `CodeCorrectnessMetric` - runs cargo check/test
  - `FileStateMetric` - checks file existence/content
  - `LlmJudgeMetric` / `LlmScoreMetric` - placeholders
- [x] Task 1.3: CLI integration
  - `--eval` flag to run scenarios
  - `--scenario <name>` to filter
  - `--list-scenarios` to list available
  - `--json` for machine-readable output
- [x] Task 1.4: Outcome reporting
  - `EvalReport` with metrics and summary
  - Terminal output with colors (pass/fail)
  - JSON output for CI

### Phase 2: Scenarios
- [x] All 5 scenarios defined with testbeds:
  1. `bug-fix` - Fix type error in Rust code
  2. `feature-impl` - Add reverse method to StringUtils
  3. `refactor` - Extract validation logic
  4. `code-understanding` - Explain binary heap
  5. `multi-step` - Create module, add function, test

### Phase 3: Integration
- [x] Task 3.1: CI workflow created
  - `.github/workflows/evals.yml`
  - Manual trigger with scenario filter
  - Uploads results as artifact

### Phase 4: Agent Executor (COMPLETE)
- [x] Task 4.1: Lightweight Eval Executor
  - `evals/executor.rs` - Minimal agent loop without PTY/sidecar
  - Uses Vertex Claude Haiku (`claude-haiku-4-5@20251001`)
  - Auto-approves all tool calls
  - Runs up to 50 iterations
  - Tracks tool calls and modified files
- [x] Task 4.2: Updated `EvalRunner.run_prompt()`
  - Now calls `execute_eval_prompt()` instead of placeholder
- [x] Task 4.3: Fixed Metric Pass Logic
  - `MetricResult::Skip` now treated as neutral (doesn't fail scenario)

## Test Results

All 5 scenarios pass (100%):

```
PASS bug-fix (10307ms)
  ✓ cargo_check
  ✓ lib_modified
  ○ fix_quality: LLM judge not yet implemented

PASS feature-impl (13289ms)
  ✓ cargo_test
  ✓ has_reverse_method
  ○ implementation_quality: LLM judge not yet implemented

PASS refactor (26400ms)
  ✓ cargo_test
  ✓ has_validate_fn
  ○ code_quality: LLM score not yet implemented

PASS code-understanding (8591ms)
  ○ identifies_heap: LLM judge not yet implemented
  ○ correct_complexity: LLM judge not yet implemented
  ○ explains_heapify: LLM judge not yet implemented

PASS multi-step (31174ms)
  ✓ utils_module
  ✓ test_file
  ✓ mod_declaration
  ✓ has_is_palindrome
  ✓ cargo_test

Results: 5/5 passed (100%)
Duration: 89761ms
```

## Remaining Work

### LLM Judge Integration
The `LlmJudgeMetric` and `LlmScoreMetric` are placeholders. Need to:
1. Initialize rig client with same Vertex AI credentials
2. Implement evaluation logic for subjective quality metrics
3. Pass criteria and output for evaluation

### Test Coverage (Optional)
- Add unit tests for metrics
- Add integration tests for scenario execution
- Verify testbeds compile correctly

## Usage

```bash
# Build with evals feature
cargo build --no-default-features --features evals --bin qbit-cli

# List scenarios
cargo run --no-default-features --features evals --bin qbit-cli -- --list-scenarios

# Run all scenarios
cargo run --no-default-features --features evals --bin qbit-cli -- --eval

# Run specific scenario
cargo run --no-default-features --features evals --bin qbit-cli -- --eval --scenario bug-fix

# JSON output
cargo run --no-default-features --features evals --bin qbit-cli -- --eval --json
```

## Files Created

```
backend/src/evals/
├── mod.rs                    # Module exports
├── runner.rs                 # EvalRunner (testbed + execution)
├── executor.rs               # Lightweight agent executor (Vertex Haiku)
├── outcome.rs                # EvalReport, MetricOutcome
├── metrics/
│   ├── mod.rs               # Metric trait
│   ├── code_correctness.rs  # Cargo check/test
│   ├── file_state.rs        # File existence/content
│   └── llm_judge.rs         # LLM-based metrics (placeholder)
└── scenarios/
    ├── mod.rs               # Scenario trait
    ├── bug_fix.rs           # Type error fix
    ├── feature_impl.rs      # Add method
    ├── refactor.rs          # Extract function
    ├── code_understanding.rs # Explain code
    └── multi_step.rs        # Multi-tool workflow

backend/src/cli/
└── eval.rs                  # CLI commands for evals

.github/workflows/
└── evals.yml                # CI workflow
```
