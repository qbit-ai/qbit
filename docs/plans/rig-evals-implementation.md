# Rig Evals Implementation Plan

Replace Python/DeepEval-based evaluations with Rust-native rig::evals framework.

---

> **⚠️ SUB-AGENT QUICK REFERENCE**
> - Response budget: **500 tokens max**
> - Report: task started → files created → verified → done
> - Do NOT: echo code, explain decisions, copy tool output
> - Format: bullet points, not paragraphs
> - **Long output?** Write to `docs/plans/rig-evals-notes/task-X.Y.md`

---

## Agent Instructions (CRITICAL)

**All agents working on tasks in this plan MUST follow these rules:**

### Output Rules

1. **Minimal status updates** - Do NOT narrate every step. Only report:
   - Task started (1 line)
   - Blockers/questions (if any)
   - Task completed (1 line with result)

2. **No code echoing** - Do NOT paste back code you wrote. Just confirm the file path and what was added.

3. **No explanations unless asked** - Do NOT explain Rust concepts, rig APIs, or design decisions unless specifically asked.

4. **Compressed results** - When reporting completion:
   ```
   ✓ Task 1.2 complete: Created metrics/mod.rs with Metric trait,
     CodeCorrectnessMetric, FileStateMetric. 4 files, 280 lines. Tests pass.
   ```
   NOT:
   ```
   I've completed Task 1.2. Let me explain what I did. First, I created
   the metrics module... [500 lines of explanation]
   ```

5. **No tool output copying** - Do NOT copy compiler output, test results, or file contents into your response. Summarize: "cargo check passed" or "3 tests passed".

6. **Write long content to files** - If you need to communicate:
   - Detailed error logs
   - Implementation notes for future tasks
   - Design decisions that affect other tasks
   - Debugging information

   Write to: `docs/plans/rig-evals-notes/task-X.Y.md` (replace X.Y with your task ID)

   Then reference it briefly in your response:
   ```
   ✓ Task 1.2 complete. See docs/plans/rig-evals-notes/task-1.2.md for API notes.
   ```

### Notes Directory Structure

```
docs/plans/rig-evals-notes/
  task-0.1.md    # Notes from Task 0.1
  task-1.2.md    # Notes from Task 1.2
  blockers.md    # Cross-task blockers (append-only)
```

Agents should create this directory if it doesn't exist.

### Example Good Response

```
Starting Task 2.1 (Bug Fix Scenario)

Created:
- backend/src/evals/testbeds/rust-bug-fix/Cargo.toml
- backend/src/evals/testbeds/rust-bug-fix/src/lib.rs (intentional type error)
- backend/src/evals/scenarios/bug_fix.rs

Verified: cargo check fails as expected, scenario compiles.

✓ Task 2.1 complete. Notes: docs/plans/rig-evals-notes/task-2.1.md
```

### Example Good Response (with blocker)

```
Starting Task 1.1 (Runner Infrastructure)

Blocker: rig::evals requires `experimental` feature but Cargo.toml has rig-core 0.9.
Need rig-core ≥1.0 for evals module.

Wrote details to docs/plans/rig-evals-notes/task-1.1.md

✗ Task 1.1 blocked. Dependency on Task 0.1 not met.
```

### Example Bad Response

```
I'll now work on Task 2.1, the Bug Fix Scenario. This task requires me to
create a testbed with an intentional compile error that the agent will need
to fix. Let me start by understanding what we need...

First, I'll create the Cargo.toml file. Here's what I'm adding:
[package]
name = "rust-bug-fix"
... [50 more lines]

Now let me create the lib.rs with the bug:
pub fn add(a: i32, b: i32) -> String {
    a + b  // This is the intentional error!
}
... [continues for 500 more lines]
```

### Token Budget

Each agent has an effective budget of **~500 tokens for their final response**. Plan accordingly.

---

## Goals

1. Test actual agent capabilities, not implementation details
2. Single language (Rust) for agent + evals
3. Reproducible, isolated testbeds
4. 5 high-value end-to-end scenarios replacing ~3000 lines of low-value tests

## Architecture

```
backend/
  src/
    evals/                      # New eval framework
      mod.rs                    # Module exports, feature gate
      runner.rs                 # Agent test harness
      outcome.rs                # Result types and reporting
      scenarios/
        mod.rs
        bug_fix.rs
        feature_impl.rs
        refactor.rs
        code_understanding.rs
        multi_step.rs
      metrics/
        mod.rs
        code_correctness.rs     # Compile/test verification
        file_state.rs           # File existence/content checks
        behavioral.rs           # LLM-as-judge wrappers
      testbeds/                 # Fixture projects (git submodule or embedded)
        rust-simple/
        go-simple/
        typescript-simple/
  Cargo.toml                    # Add `evals` feature flag
```

---

## Phase 0: Prerequisites (Sequential)

These must be completed before parallel work can begin.

### Task 0.1: Enable rig evals feature

**Depends on:** Nothing
**Blocks:** All Phase 1 tasks

- [ ] Update `backend/Cargo.toml` to enable rig-core `experimental` feature
- [ ] Add `evals` feature flag to qbit backend
- [ ] Verify `rig::evals` module is accessible
- [ ] Create `backend/src/evals/mod.rs` with feature gate

**Acceptance:**
```rust
#[cfg(feature = "evals")]
pub mod evals;
```
Compiles with `cargo check --features evals`

---

### Task 0.2: Design testbed structure

**Depends on:** Nothing
**Blocks:** Phase 2 testbed tasks

- [ ] Decide: embedded in repo vs git submodule vs generated at runtime
- [ ] Define testbed interface (how evals discover/reset testbeds)
- [ ] Document testbed requirements in this file

**Recommendation:** Embed minimal testbeds in `backend/src/evals/testbeds/` as static files, copy to temp dir at runtime for isolation.

**Acceptance:** Decision documented, interface defined

---

## Phase 1: Core Framework (Parallel)

These tasks can be worked on concurrently after Phase 0 completes.

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  Task 1.1       │  │  Task 1.2       │  │  Task 1.3       │
│  Runner         │  │  Metrics        │  │  CLI            │
│  Infrastructure │  │  Framework      │  │  Integration    │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

### Task 1.1: Eval runner infrastructure

**Depends on:** 0.1
**Blocks:** Phase 2 scenarios

- [ ] Create `runner.rs` with `EvalRunner` struct
- [ ] Implement testbed setup (copy to temp dir)
- [ ] Implement agent invocation (reuse `agentic_loop` or CLI)
- [ ] Implement testbed teardown
- [ ] Capture agent output, tool calls, final state

**Interface:**
```rust
pub struct EvalRunner {
    pub workspace: TempDir,
    pub model: String,
}

impl EvalRunner {
    pub async fn setup_testbed(&self, testbed: &str) -> Result<PathBuf>;
    pub async fn run_prompt(&self, prompt: &str) -> Result<AgentOutput>;
    pub async fn cleanup(&self) -> Result<()>;
}

pub struct AgentOutput {
    pub response: String,
    pub tool_calls: Vec<ToolCall>,
    pub files_modified: Vec<PathBuf>,
    pub duration_ms: u64,
}
```

**Acceptance:** Can run a simple prompt against a testbed, capture output

---

### Task 1.2: Metrics framework

**Depends on:** 0.1
**Blocks:** Phase 2 scenarios

- [ ] Create `metrics/mod.rs` with `Metric` trait
- [ ] Implement `CodeCorrectnessMetric` (runs shell command, checks exit code)
- [ ] Implement `FileStateMetric` (checks file exists, contains pattern)
- [ ] Implement `LlmJudgeMetric` wrapper around `rig::evals::LlmJudgeMetric`
- [ ] Implement `LlmScoreMetric` wrapper around `rig::evals::LlmScoreMetric`

**Interface:**
```rust
pub trait Metric: Send + Sync {
    fn name(&self) -> &str;
    async fn evaluate(&self, ctx: &EvalContext) -> MetricResult;
}

pub enum MetricResult {
    Pass,
    Fail { reason: String },
    Score { value: f64, max: f64 },
    Skip { reason: String },
}

pub struct EvalContext {
    pub workspace: PathBuf,
    pub agent_output: AgentOutput,
    pub scenario: ScenarioConfig,
}
```

**Acceptance:** All 4 metric types implemented with unit tests

---

### Task 1.3: CLI integration

**Depends on:** 0.1
**Blocks:** Nothing (can be extended later)

- [ ] Add `eval` subcommand to qbit CLI
- [ ] Implement `--scenario` filter
- [ ] Implement `--model` override
- [ ] Implement `--list` to show available scenarios
- [ ] Implement JSON output for CI integration

**Interface:**
```bash
qbit-cli eval                           # Run all scenarios
qbit-cli eval --scenario bug-fix        # Run specific scenario
qbit-cli eval --model claude-sonnet-4-20250514   # Override model
qbit-cli eval --list                    # List available scenarios
qbit-cli eval --output json             # JSON output for CI
```

**Acceptance:** CLI runs and reports results (can use stub scenarios initially)

---

### Task 1.4: Outcome reporting

**Depends on:** 0.1
**Blocks:** Phase 3

- [ ] Create `outcome.rs` with result types
- [ ] Implement terminal reporter (colored pass/fail)
- [ ] Implement JSON reporter
- [ ] Implement summary statistics

**Interface:**
```rust
pub struct EvalReport {
    pub scenario: String,
    pub passed: bool,
    pub metrics: Vec<MetricOutcome>,
    pub duration_ms: u64,
    pub agent_output: AgentOutput,
}

pub struct MetricOutcome {
    pub name: String,
    pub result: MetricResult,
}
```

**Acceptance:** Pretty terminal output, valid JSON output

---

## Phase 2: Scenarios & Testbeds (Parallel)

These 5 scenarios can be implemented concurrently after Phase 1 tasks complete.

```
┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│  Task 2.1    │ │  Task 2.2    │ │  Task 2.3    │ │  Task 2.4    │ │  Task 2.5    │
│  Bug Fix     │ │  Feature     │ │  Refactor    │ │  Code        │ │  Multi-Step  │
│              │ │  Impl        │ │              │ │  Understanding│ │              │
└──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘
```

### Task 2.1: Bug fix scenario

**Depends on:** 1.1, 1.2
**Blocks:** Nothing

**Scenario:** Agent fixes a compile error in Rust code

- [ ] Create testbed: `rust-bug-fix/` with intentional type error
- [ ] Create `scenarios/bug_fix.rs`
- [ ] Define prompt template
- [ ] Configure metrics: CodeCorrectness (cargo check), FileState, LlmJudge

**Testbed structure:**
```
rust-bug-fix/
  Cargo.toml
  src/
    lib.rs        # Contains: fn add(a: i32, b: i32) -> String { a + b }
                  # Error: mismatched types, expected String, found i32
```

**Prompt:** "Fix the compile error in src/lib.rs"

**Metrics:**
| Metric | Criteria |
|--------|----------|
| CodeCorrectness | `cargo check` exits 0 |
| FileState | `src/lib.rs` modified |
| LlmJudge | Fix is minimal and correct (not a workaround) |

**Acceptance:** Scenario runs, passes when agent fixes correctly, fails otherwise

---

### Task 2.2: Feature implementation scenario

**Depends on:** 1.1, 1.2
**Blocks:** Nothing

**Scenario:** Agent adds a new method to existing code

- [ ] Create testbed: `rust-feature/` with utility struct
- [ ] Create `scenarios/feature_impl.rs`
- [ ] Define prompt template
- [ ] Configure metrics: CodeCorrectness (cargo test), FileState, LlmJudge

**Testbed structure:**
```
rust-feature/
  Cargo.toml
  src/
    lib.rs        # struct StringUtils; impl StringUtils { fn uppercase(&self, s: &str) -> String }
  tests/
    integration.rs  # #[test] fn test_reverse() { assert_eq!(StringUtils.reverse("abc"), "cba"); }
                    # Test exists but method doesn't - will fail until implemented
```

**Prompt:** "Add a `reverse` method to StringUtils that reverses a string. The test in tests/integration.rs should pass."

**Metrics:**
| Metric | Criteria |
|--------|----------|
| CodeCorrectness | `cargo test` exits 0 |
| FileState | `reverse` method exists in lib.rs |
| LlmJudge | Implementation is idiomatic Rust |

**Acceptance:** Scenario runs, passes when agent implements correctly

---

### Task 2.3: Refactoring scenario

**Depends on:** 1.1, 1.2
**Blocks:** Nothing

**Scenario:** Agent extracts logic into a separate function

- [ ] Create testbed: `rust-refactor/` with long function
- [ ] Create `scenarios/refactor.rs`
- [ ] Define prompt template
- [ ] Configure metrics: CodeCorrectness (cargo test), FileState, LlmScore

**Testbed structure:**
```
rust-refactor/
  Cargo.toml
  src/
    lib.rs        # fn process(input: &str) -> Result<Output> {
                  #     // 50 lines of validation logic inline
                  #     // 20 lines of processing logic
                  # }
  tests/
    lib_test.rs   # Existing tests that verify behavior
```

**Prompt:** "Extract the validation logic from the `process` function into a separate `validate` function. Keep the existing tests passing."

**Metrics:**
| Metric | Criteria |
|--------|----------|
| CodeCorrectness | `cargo test` exits 0 (behavior preserved) |
| FileState | `validate` function exists |
| LlmScore | Code is cleaner (0-10 scale) |

**Acceptance:** Scenario runs, passes when refactor is clean and tests pass

---

### Task 2.4: Code understanding scenario

**Depends on:** 1.1, 1.2
**Blocks:** Nothing

**Scenario:** Agent explains code accurately

- [ ] Create testbed: `rust-understanding/` with algorithm
- [ ] Create `scenarios/code_understanding.rs`
- [ ] Define prompt template and expected answers
- [ ] Configure metrics: SemanticSimilarity, LlmJudge

**Testbed structure:**
```
rust-understanding/
  Cargo.toml
  src/
    lib.rs        # impl<T: Ord> BinaryHeap<T> {
                  #     fn heapify_down(&mut self, idx: usize) { ... }
                  #     fn extract_min(&mut self) -> Option<T> { ... }
                  # }
```

**Prompt:** "Read src/lib.rs and answer: 1) What data structure is implemented? 2) What is the time complexity of extract_min? 3) Why does heapify_down compare with children?"

**Expected answers:**
1. Binary min-heap
2. O(log n)
3. To maintain heap property after removal

**Metrics:**
| Metric | Criteria |
|--------|----------|
| LlmJudge | Answer 1 correct (mentions heap/priority queue) |
| LlmJudge | Answer 2 correct (O(log n) or equivalent) |
| LlmJudge | Answer 3 correct (mentions heap property/ordering) |

**Acceptance:** Scenario runs, passes when explanations are accurate

---

### Task 2.5: Multi-step task scenario

**Depends on:** 1.1, 1.2
**Blocks:** Nothing

**Scenario:** Agent completes a workflow requiring multiple tools

- [ ] Create testbed: `rust-multi-step/` with minimal project
- [ ] Create `scenarios/multi_step.rs`
- [ ] Define prompt template
- [ ] Configure metrics: FileState (multiple), CodeCorrectness

**Testbed structure:**
```
rust-multi-step/
  Cargo.toml
  src/
    lib.rs        # // empty or minimal
```

**Prompt:** "Create a new module called `utils` with a function `is_palindrome(s: &str) -> bool`. Add a test for it in a new file `tests/utils_test.rs`. Then run the tests to verify it works."

**Metrics:**
| Metric | Criteria |
|--------|----------|
| FileState | `src/utils.rs` exists |
| FileState | `tests/utils_test.rs` exists |
| FileState | `src/lib.rs` has `mod utils` |
| CodeCorrectness | `cargo test` exits 0 |

**Acceptance:** Scenario runs, all files created and tests pass

---

## Phase 3: Migration & Cleanup (Sequential)

After Phase 2 completes.

### Task 3.1: CI integration

**Depends on:** 2.1-2.5
**Blocks:** 3.2

- [ ] Add GitHub Actions workflow for Rust evals
- [ ] Run evals on PR (optional, can be expensive)
- [ ] Run evals on merge to main
- [ ] Store results as artifacts

**Acceptance:** Evals run in CI, results visible

---

### Task 3.2: Remove Python evals

**Depends on:** 3.1
**Blocks:** Nothing

- [ ] Verify Rust evals cover critical scenarios
- [ ] Move workspace isolation tests to `backend/` Rust tests
- [ ] Move server API tests to integration tests
- [ ] Delete `evals/` Python directory
- [ ] Update CLAUDE.md and documentation

**Acceptance:** `evals/` directory removed, no regression in coverage

---

## Dependency Graph

```
Phase 0 (Sequential)
    │
    ├── 0.1 Enable rig evals ────────┬─────────────────────────────────────┐
    │                                │                                     │
    └── 0.2 Design testbeds          │                                     │
                                     ▼                                     │
Phase 1 (Parallel) ──────────────────┼─────────────────────────────────────┤
    │                                │                                     │
    ├── 1.1 Runner ──────────────────┤                                     │
    ├── 1.2 Metrics ─────────────────┤                                     │
    ├── 1.3 CLI ─────────────────────┤                                     │
    └── 1.4 Reporting ───────────────┤                                     │
                                     │                                     │
                                     ▼                                     │
Phase 2 (Parallel) ──────────────────┼─────────────────────────────────────┤
    │                                │                                     │
    ├── 2.1 Bug Fix ─────────────────┤                                     │
    ├── 2.2 Feature Impl ────────────┤                                     │
    ├── 2.3 Refactor ────────────────┤                                     │
    ├── 2.4 Code Understanding ──────┤                                     │
    └── 2.5 Multi-Step ──────────────┤                                     │
                                     │                                     │
                                     ▼                                     │
Phase 3 (Sequential) ────────────────┴─────────────────────────────────────┘
    │
    ├── 3.1 CI Integration
    └── 3.2 Remove Python evals
```

## Parallelization Summary

| Phase | Tasks | Parallelizable | Est. Effort |
|-------|-------|----------------|-------------|
| 0 | 2 | No | 2-3 hours |
| 1 | 4 | Yes (4 agents) | 4-6 hours each |
| 2 | 5 | Yes (5 agents) | 3-4 hours each |
| 3 | 2 | No | 2-3 hours |

**Maximum parallelization:** 5 agents working on Phase 2 scenarios simultaneously after Phase 1 framework is ready.

**Critical path:** 0.1 → 1.1 + 1.2 → any scenario → 3.1 → 3.2

---

## Main Agent Dispatch Protocol

When dispatching sub-agents, use this template:

```
Execute Task [X.Y] from docs/plans/rig-evals-implementation.md

CRITICAL: Follow the Agent Instructions section. Keep response under 500 tokens.
Report only: files created, verification status, completion confirmation.
```

### Collecting Results

When a sub-agent completes, record only:
- Task ID
- Pass/Fail
- Files created (list)
- Any blockers for dependent tasks

Do NOT ask sub-agents to elaborate or explain their work. If you need details, read their notes file at `docs/plans/rig-evals-notes/task-X.Y.md`.

### Handling Failures

If a sub-agent reports a blocker:
1. Record the blocker
2. Check if other tasks can proceed
3. Address blocker before retrying
4. Do NOT spawn multiple agents to retry the same task

## Success Criteria

- [ ] All 5 scenarios pass with Claude Sonnet
- [ ] Evals complete in < 10 minutes total
- [ ] JSON output parseable by CI
- [ ] No Python dependencies for evals
- [ ] Documentation updated
