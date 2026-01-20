# Agent Benchmark Integration Plan

This document outlines options for running popular, standardized coding agent benchmarks using Qbit's existing eval framework.

## Current Eval Framework Summary

### Architecture
```
qbit-evals/
├── config.rs          # Provider configuration (Vertex, OpenAI, Z.AI)
├── runner.rs          # EvalRunner: testbed setup, orchestration
├── executor.rs        # Agent execution using production agentic loop
├── metrics/           # Evaluation metrics
│   ├── code_correctness.rs  # cargo check/test
│   ├── file_state.rs        # File existence/content/modified
│   ├── llm_judge.rs         # LLM-based pass/fail and scoring
│   └── token_tracking.rs    # Token usage tracking
├── scenarios/         # Scenario definitions
│   └── *.rs           # Each scenario: testbed + prompt + metrics
└── outcome.rs         # EvalReport, EvalSummary, JSON output
```

### Key Traits
```rust
// Scenario trait - define what to test
trait Scenario {
    fn name(&self) -> &str;
    fn testbed(&self) -> &str;           // Embedded files
    fn prompt(&self) -> &str;            // Task for agent
    fn metrics(&self) -> Vec<Box<dyn Metric>>;
}

// Metric trait - evaluate results
trait Metric {
    fn name(&self) -> &str;
    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult>;
}
```

### Current Capabilities
- **Providers**: Vertex AI Claude, OpenAI GPT, Z.AI GLM
- **Testbeds**: Embedded Rust projects (embedded in binary)
- **Metrics**: Code correctness (cargo), file state, LLM judge/score
- **Output**: Terminal summaries, JSON for CI
- **Execution**: Production agentic loop with auto-approve

---

## Target Benchmarks

### 1. SWE-bench / SWE-bench Verified
- **What**: Real GitHub issues from 12 Python repos, generate patches
- **Size**: 2,200+ (full), 500 (Verified), 300 (Lite)
- **Format**: Issue description + codebase → patch
- **Evaluation**: Run test suite in Docker, check tests pass
- **Industry Standard**: The gold standard for coding agents

### 2. HumanEval
- **What**: 164 hand-crafted Python function synthesis problems
- **Size**: 164 problems
- **Format**: Function signature + docstring → implementation
- **Evaluation**: pass@k metric (unit tests)
- **Notes**: Mostly solved by top models (~92%), good baseline

### 3. MBPP (Mostly Basic Python Problems)
- **What**: 974 entry-level Python problems
- **Size**: 974 problems
- **Format**: Natural language description → code snippet
- **Evaluation**: Unit tests
- **Notes**: Easier than HumanEval, saturated benchmark

### 4. EvalPlus (HumanEval+ / MBPP+)
- **What**: HumanEval/MBPP with 80x/35x more test cases
- **Size**: Same problems, more rigorous testing
- **Purpose**: Catches overfitting to original test cases

---

## Integration Options

### Option A: Native Rust Scenarios (Extend Current Framework)

**Approach**: Create Rust scenarios that embed benchmark problems, run through existing executor.

**Implementation**:
```rust
// Example: HumanEval scenario wrapper
pub struct HumanEvalScenario {
    problem_id: String,
    prompt: String,
    test_code: String,
}

impl Scenario for HumanEvalScenario {
    fn testbed(&self) -> &str { "python-humaneval" }
    fn prompt(&self) -> &str { &self.prompt }
    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(CodeCorrectnessMetric::python_test(&self.test_code)),
        ]
    }
}

// Load all 164 problems
pub fn humaneval_scenarios() -> Vec<Box<dyn Scenario>> {
    include_str!("humaneval.jsonl")
        .lines()
        .map(|line| parse_humaneval_problem(line))
        .collect()
}
```

**Pros**:
- Fully integrated with existing framework
- Single binary, no external dependencies
- Consistent metrics and reporting
- Works with all providers (Vertex, OpenAI, Z.AI)

**Cons**:
- Must embed/manage benchmark data in Rust
- SWE-bench requires Docker integration (complex)
- No standard comparison with other agents' results

**Best For**: HumanEval, MBPP (simpler benchmarks)

---

### Option B: Harness Wrapper (Shell Out to Benchmark Tools)

**Approach**: Use existing benchmark harnesses (Python packages) from Rust via subprocess.

**Implementation**:
```rust
// SWE-bench integration
pub struct SweBenchScenario {
    instance_id: String,
}

impl Scenario for SweBenchScenario {
    fn testbed(&self) -> &str { "swebench-instance" }

    fn prompt(&self) -> &str {
        // Load from SWE-bench dataset via Python
        &self.cached_prompt
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(SweBenchEvalMetric::new(&self.instance_id)),
        ]
    }
}

// SweBenchEvalMetric - shells out to swebench harness
impl Metric for SweBenchEvalMetric {
    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        // 1. Extract patch from agent's file modifications
        // 2. Write predictions.jsonl
        // 3. Run: python -m swebench.harness.run_evaluation
        // 4. Parse results
    }
}
```

**Pros**:
- Uses official evaluation harnesses
- Results comparable to published benchmarks
- Docker-based isolation (for SWE-bench)
- Maintained by benchmark authors

**Cons**:
- Python dependency required
- More complex setup (Docker, venv)
- Slower execution
- Less control over evaluation details

**Best For**: SWE-bench (requires Docker anyway)

---

### Option C: Hybrid Approach (Recommended)

**Approach**: Different strategies for different benchmarks based on complexity.

| Benchmark | Strategy | Rationale |
|-----------|----------|-----------|
| HumanEval | Native Rust | Simple format, just run Python tests |
| MBPP | Native Rust | Same as HumanEval |
| EvalPlus | Native Rust | Enhanced test cases, same execution |
| SWE-bench Lite | Harness Wrapper | Docker required, official eval |
| SWE-bench Verified | Harness Wrapper | Docker required, official eval |

**Implementation Structure**:
```
qbit-evals/
├── benchmarks/
│   ├── mod.rs
│   ├── humaneval/
│   │   ├── mod.rs          # Native scenarios
│   │   ├── problems.jsonl  # Embedded dataset (164 problems)
│   │   └── runner.rs       # Python test runner
│   ├── mbpp/
│   │   ├── mod.rs
│   │   └── problems.jsonl  # Embedded dataset (974 problems)
│   └── swebench/
│       ├── mod.rs          # Harness wrapper
│       ├── loader.rs       # Dataset loading via HF
│       └── harness.rs      # Docker-based evaluation
├── metrics/
│   ├── python_test.rs      # Python unittest execution
│   └── swebench_eval.rs    # SWE-bench harness integration
```

**CLI Interface**:
```bash
# Run HumanEval (native, fast)
cargo run --features evals --bin qbit-cli -- --benchmark humaneval

# Run specific HumanEval problems
cargo run --features evals --bin qbit-cli -- --benchmark humaneval --problems 1-10

# Run SWE-bench Lite (Docker required)
cargo run --features evals --bin qbit-cli -- --benchmark swebench-lite

# Run SWE-bench Verified subset
cargo run --features evals --bin qbit-cli -- --benchmark swebench-verified --instances sympy__sympy-22914
```

---

### Option D: External Benchmark Runner (Minimal Integration)

**Approach**: Create a standalone predictions generator, use external tools for evaluation.

**Implementation**:
```rust
// Generate predictions file for any benchmark
pub async fn generate_predictions(
    benchmark: &str,
    output_path: &Path,
) -> Result<()> {
    let problems = load_benchmark_problems(benchmark)?;
    let mut predictions = Vec::new();

    for problem in problems {
        let runner = EvalRunner::new()?;
        let workspace = runner.setup_testbed(&problem.testbed).await?;
        let output = runner.run_prompt(&workspace, &problem.prompt).await?;

        predictions.push(json!({
            "task_id": problem.id,
            "completion": extract_code(&output.response),
            // or for SWE-bench:
            "model_patch": extract_patch(&output.files_modified),
        }));
    }

    write_jsonl(output_path, &predictions)?;
    Ok(())
}
```

Then use external tools:
```bash
# Generate predictions
cargo run --features evals --bin qbit-cli -- --generate-predictions humaneval --output predictions.jsonl

# Evaluate with official harness (HumanEval)
evaluate_functional_correctness predictions.jsonl

# Evaluate with official harness (SWE-bench)
python -m swebench.harness.run_evaluation --predictions_path predictions.jsonl
```

**Pros**:
- Maximum compatibility with official evaluation
- No need to replicate evaluation logic
- Results directly comparable to leaderboards
- Simplest to implement

**Cons**:
- Two-step process
- Requires external tool installation
- Less integrated experience

**Best For**: Initial implementation, validation against official results

---

## Recommended Implementation Roadmap

### Phase 1: HumanEval Integration (Native)
**Effort**: ~2-3 days

1. Add `humaneval` feature flag to qbit-evals
2. Create `PythonTestMetric` - executes Python code and tests
3. Embed HumanEval dataset (164 problems, ~50KB)
4. Create `HumanEvalScenario` that:
   - Sets up minimal Python testbed
   - Prompts agent with function signature + docstring
   - Runs generated code against unit tests
5. Add pass@1 metric calculation
6. CLI: `--benchmark humaneval`

**Key Files**:
```rust
// benchmarks/humaneval/mod.rs
pub fn scenarios() -> Vec<Box<dyn Scenario>> { ... }

// metrics/python_test.rs
pub struct PythonTestMetric { test_code: String }
impl Metric for PythonTestMetric { ... }
```

### Phase 2: MBPP & EvalPlus (Native)
**Effort**: ~1-2 days (reuse HumanEval infrastructure)

1. Embed MBPP dataset (974 problems)
2. Embed EvalPlus enhanced test cases
3. Create scenarios using same `PythonTestMetric`
4. CLI: `--benchmark mbpp`, `--benchmark humaneval-plus`

### Phase 3: SWE-bench Integration (Harness Wrapper)
**Effort**: ~5-7 days

1. Add `swebench` feature flag (requires Docker)
2. Create `SweBenchLoader` - loads instances from HuggingFace
3. Create `SweBenchTestbed` - clones repo, checks out commit
4. Create `SweBenchEvalMetric` - runs official harness
5. Support `SWE-bench_Lite` (300 instances) initially
6. CLI: `--benchmark swebench-lite`

**Prerequisites**:
- Docker installed and running
- Python 3.11+ with swebench package
- ~120GB disk space for Docker images

### Phase 4: Reporting & Comparison
**Effort**: ~2-3 days

1. Add benchmark-specific result formats
2. Export results in standard formats (for leaderboard submission)
3. Add comparison with published baselines
4. CI workflow for automated benchmark runs

---

## Technical Considerations

### Python Test Execution
```rust
pub struct PythonTestMetric {
    test_code: String,
    timeout_secs: u64,
}

impl Metric for PythonTestMetric {
    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        // 1. Write solution to solution.py
        let solution_path = ctx.workspace.join("solution.py");

        // 2. Write test file
        let test_path = ctx.workspace.join("test_solution.py");
        std::fs::write(&test_path, &self.test_code)?;

        // 3. Run pytest with timeout
        let output = Command::new("python")
            .args(["-m", "pytest", "-v", test_path.to_str().unwrap()])
            .current_dir(&ctx.workspace)
            .timeout(Duration::from_secs(self.timeout_secs))
            .output()?;

        if output.status.success() {
            Ok(MetricResult::Pass)
        } else {
            Ok(MetricResult::Fail {
                reason: String::from_utf8_lossy(&output.stderr).to_string()
            })
        }
    }
}
```

### SWE-bench Docker Integration
```rust
pub struct SweBenchEvalMetric {
    instance_id: String,
}

impl Metric for SweBenchEvalMetric {
    async fn evaluate(&self, ctx: &EvalContext) -> Result<MetricResult> {
        // 1. Generate patch from agent's modifications
        let patch = generate_patch(&ctx.agent_output.files_modified)?;

        // 2. Write predictions file
        let predictions = json!([{
            "instance_id": self.instance_id,
            "model_patch": patch,
            "model_name_or_path": "qbit",
        }]);
        let pred_path = ctx.workspace.join("predictions.jsonl");
        write_jsonl(&pred_path, &predictions)?;

        // 3. Run SWE-bench harness
        let output = Command::new("python")
            .args([
                "-m", "swebench.harness.run_evaluation",
                "--dataset_name", "princeton-nlp/SWE-bench_Lite",
                "--predictions_path", pred_path.to_str().unwrap(),
                "--max_workers", "1",
                "--run_id", &format!("qbit_{}", self.instance_id),
            ])
            .output()?;

        // 4. Parse results
        parse_swebench_results(&output)
    }
}
```

### Resource Requirements

| Benchmark | Disk | RAM | CPU | Time/Problem |
|-----------|------|-----|-----|--------------|
| HumanEval | 100MB | 2GB | 1 | ~10-30s |
| MBPP | 100MB | 2GB | 1 | ~10-30s |
| SWE-bench Lite | 120GB | 16GB | 8 | ~2-5min |
| SWE-bench Verified | 120GB | 16GB | 8 | ~2-5min |

---

## Cost Estimates

Using Claude Sonnet on Vertex AI:

| Benchmark | Problems | Est. Tokens/Problem | Est. Cost |
|-----------|----------|---------------------|-----------|
| HumanEval | 164 | ~2K | ~$0.50 |
| MBPP | 974 | ~2K | ~$3.00 |
| SWE-bench Lite | 300 | ~50K | ~$45.00 |
| SWE-bench Verified | 500 | ~50K | ~$75.00 |

---

## Summary of Options

| Option | Complexity | Integration | Comparability | Best For |
|--------|------------|-------------|---------------|----------|
| A: Native Rust | Medium | High | Low | HumanEval, MBPP |
| B: Harness Wrapper | High | Medium | High | SWE-bench |
| C: Hybrid | High | High | High | Production use |
| D: External Runner | Low | Low | High | Quick validation |

**Recommendation**: Start with **Option D** (External Runner) for quick validation, then implement **Option C** (Hybrid) for full integration. This gives immediate benchmarking capability while building toward a polished integrated experience.

---

## References

- [SWE-bench GitHub](https://github.com/SWE-bench/SWE-bench)
- [SWE-bench Evaluation Guide](https://www.swebench.com/SWE-bench/guides/evaluation/)
- [HumanEval GitHub](https://github.com/openai/human-eval)
- [HumanEval on HuggingFace](https://huggingface.co/datasets/openai/openai_humaneval)
- [EvalPlus](https://evalplus.github.io/)
- [Understanding LLM Code Benchmarks](https://runloop.ai/blog/understanding-llm-code-benchmarks-from-humaneval-to-swe-bench)
