# Eval Framework Overview

The qbit-evals crate is a Rust-native evaluation framework for end-to-end testing of the Qbit AI agent. It tests actual production code paths using isolated testbeds and structured metrics.

## Architecture

```
qbit-evals/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Module exports
    ├── config.rs                 # Provider configuration loading
    ├── runner.rs                 # EvalRunner: testbed orchestration
    ├── executor.rs               # Agent executor for evals
    ├── outcome.rs                # Report types & formatting
    ├── metrics/
    │   ├── mod.rs                # Metric trait definition
    │   ├── code_correctness.rs   # Cargo check/test metrics
    │   ├── file_state.rs         # File existence/content checks
    │   ├── llm_judge.rs          # LLM-based judgment metrics
    │   └── token_tracking.rs     # Token usage metrics
    └── scenarios/
        ├── mod.rs                # Scenario trait & registry
        ├── bug_fix.rs            # Fix compile error scenario
        ├── feature_impl.rs       # Implement new feature scenario
        ├── refactor.rs           # Extract function scenario
        ├── code_understanding.rs # Explain code scenario
        ├── multi_step.rs         # Complex workflow scenario
        ├── web_search.rs         # Web search scenario
        ├── multi_turn.rs         # Multi-turn conversation tests
        └── prompt_composition.rs # System prompt instruction tests
```

## Key Concepts

### Scenarios

A scenario defines a complete evaluation case:

```rust
pub trait Scenario: Send + Sync {
    fn name(&self) -> &str;                    // Unique identifier
    fn description(&self) -> &str;             // Human-readable description
    fn testbed(&self) -> &str;                 // Embedded testbed name
    fn prompt(&self) -> &str;                  // Task prompt for agent
    fn metrics(&self) -> Vec<Box<dyn Metric>>; // Evaluation metrics
    fn system_prompt(&self) -> Option<&str>;   // Optional custom system prompt

    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport>;
}
```

### Metrics

Metrics evaluate agent performance:

| Metric Type | Purpose |
|-------------|---------|
| `CodeCorrectnessMetric` | Runs `cargo check/test`, checks exit code |
| `FileStateMetric` | Verifies file existence, content, modifications |
| `LlmJudgeMetric` | Uses LLM to judge if criteria met (pass/fail) |
| `LlmScoreMetric` | Uses LLM to score on numeric scale |
| `TokenTrackingMetric` | Validates token usage patterns |

### Testbeds

Testbeds are embedded project templates copied to temp directories:

- `rust-bug-fix` - Rust project with type error
- `rust-feature` - Feature implementation task
- `rust-refactor` - Refactoring task
- `rust-understanding` - Code understanding task
- `rust-multi-step` - Multi-step task
- `minimal` - Minimal testbed for web search

## Execution Flow

```
1. CLI invokes `run_evals(scenario_filter, options)`
   ↓
2. EvalRunner created with temp workspace
   ↓
3. For each scenario:
   a. Setup testbed (copy embedded files to temp dir)
   b. Execute agent via qbit-ai's unified agentic loop
   c. Collect AgentOutput (response, tool_calls, files_modified)
   d. Run metrics against EvalContext
   e. Generate EvalReport
   ↓
4. Aggregate into EvalSummary
   ↓
5. Output results (terminal or JSON)
```

## CLI Usage

```bash
# Run all default scenarios
cargo run --features evals,cli --bin qbit-cli -- --eval

# Run specific scenario
cargo run --features evals,cli --bin qbit-cli -- --eval --scenario bug-fix

# List available scenarios
cargo run --features evals,cli --bin qbit-cli -- --list-scenarios

# Parallel execution with verbose output
cargo run --features evals,cli --bin qbit-cli -- --eval --parallel --verbose

# JSON output for CI
cargo run --features evals,cli --bin qbit-cli -- --eval --json

# Specific provider
cargo run --features evals,cli --bin qbit-cli -- --eval --provider openai
```

## Provider Support

| Provider | Model | Web Search |
|----------|-------|------------|
| `vertex-claude` (default) | Claude Sonnet 4.5 | `web_search_20250305` |
| `zai` | GLM-4.7 | Tavily fallback |
| `openai` | GPT-5.1 (Responses API) | `web_search_preview` |

## Available Scenarios

### Core Scenarios (default)

| Scenario | Description | Metrics |
|----------|-------------|---------|
| `bug-fix` | Fix type mismatch error | cargo_check, file_modified, fix_quality |
| `feature-impl` | Implement StringUtils::reverse() | cargo_check, has_method, correctness |
| `refactor` | Extract validation logic | cargo_test, extracted_function, quality |
| `code-understanding` | Explain binary heap | response_quality |
| `multi-step` | Create module, function, tests | multiple file/code checks |
| `web-search` | Use web search tool | response_quality, sources_cited |

### Prompt Composition Scenarios

| Scenario | Tests |
|----------|-------|
| `prompt-output-format` | JSON output format compliance |
| `prompt-coding-conventions` | Coding standards (snake_case, docs) |
| `prompt-tool-preference` | Tool selection based on instructions |
| `prompt-brevity-instruction` | Response conciseness |
| `prompt-sub-agent-awareness` | Sub-agent capability awareness |
| `prompt-provider-context` | Provider-specific feature usage |
| `prompt-conflicting-instructions` | Priority handling |

### Optional Scenarios

| Scenario | Purpose |
|----------|---------|
| `openai-web-search` | OpenAI-specific web search |
| `multi-turn-file` | Multi-turn file operations |
| `multi-turn-reasoning` | Reasoning ID preservation (OpenAI) |

## Output Format

### Terminal Output

```
✓ bug-fix (11122ms)
  ✓ cargo_check: Pass
  ✓ lib_modified: Pass
  ✓ fix_quality: Pass

Results: 5/5 passed (100%)
Duration: 89275ms
```

### JSON Output

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

## Adding New Scenarios

1. Create scenario file in `scenarios/`:
   ```rust
   pub struct MyScenario;

   impl Scenario for MyScenario {
       fn name(&self) -> &str { "my-scenario" }
       fn description(&self) -> &str { "Tests something specific" }
       fn testbed(&self) -> &str { "my-testbed" }
       fn prompt(&self) -> &str { "Do the thing" }

       fn metrics(&self) -> Vec<Box<dyn Metric>> {
           vec![
               Box::new(CodeCorrectnessMetric::cargo_check()),
               Box::new(FileStateMetric::modified("file_modified", "src/lib.rs")),
           ]
       }
   }

   pub fn testbed_files() -> Vec<(String, String)> {
       vec![
           ("Cargo.toml".to_string(), r#"[package]..."#.to_string()),
           ("src/lib.rs".to_string(), r#"// code..."#.to_string()),
       ]
   }
   ```

2. Register in `scenarios/mod.rs`:
   ```rust
   pub fn default_scenarios() -> Vec<Box<dyn Scenario>> {
       vec![
           // ...existing scenarios...
           Box::new(my_scenario::MyScenario),
       ]
   }
   ```

3. Add testbed dispatch in `runner.rs`:
   ```rust
   fn get_testbed_content(name: &str) -> Result<Vec<(String, String)>> {
       match name {
           // ...existing testbeds...
           "my-testbed" => Ok(my_scenario::testbed_files()),
       }
   }
   ```

## Design Principles

1. **Test production code paths** - Uses same unified agentic loop as main agent
2. **Provider-agnostic** - Same scenarios run on Vertex, OpenAI, Z.AI
3. **Isolated execution** - Temp directories, no environment pollution
4. **Rich metrics** - Binary, scored, and LLM-judged evaluations
5. **CI-friendly** - JSON output, exit codes, parallel execution
6. **Extensible** - Simple trait-based scenario definition
