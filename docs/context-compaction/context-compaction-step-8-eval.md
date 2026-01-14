# Step 8: Eval

**Goal:** Measure the quality and effectiveness of context compaction. Tune thresholds and summarizer prompts based on real-world usage.

**Outcome:** After this step, we have data-driven confidence that compaction works well and know what adjustments are needed.

---

## Prerequisites

- Steps 1-7 completed (compaction fully implemented, pruning removed)
- Access to real or realistic test conversations

## Eval Categories

1. **Summary Quality** - Does the summary capture all important context?
2. **Task Continuity** - Can the agent continue work effectively after compaction?
3. **Threshold Tuning** - Is 80% the right trigger point?
4. **Model Limits** - Are our context limit values accurate?
5. **Performance** - How long does compaction take?

---

## Task Breakdown

### 8.1 Create eval test harness

**File:** `backend/crates/qbit/tests/compaction_eval.rs`

```rust
//! Evaluation tests for context compaction.
//!
//! These tests are designed to measure the quality and effectiveness
//! of the compaction system. They should be run manually with real
//! LLM access, not in CI.
//!
//! Run with: cargo test -p qbit compaction_eval -- --ignored --nocapture

use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture for compaction evaluation.
struct CompactionEvalFixture {
    session_dir: TempDir,
    session_id: String,
}

impl CompactionEvalFixture {
    fn new() -> Self {
        let session_dir = TempDir::new().unwrap();
        let session_id = format!("eval-{}", uuid::Uuid::new_v4());
        Self { session_dir, session_id }
    }
}

/// Eval: Summary captures key task details
#[tokio::test]
#[ignore]
async fn eval_summary_captures_task() {
    // Setup: Create a transcript with a specific task
    // Action: Run summarizer
    // Assert: Summary contains key elements
    
    let fixture = CompactionEvalFixture::new();
    
    // Create transcript with a specific task
    let transcript = r#"
[turn 001] USER:
Please create a new Rust function called `calculate_fibonacci` that takes a number n and returns the nth Fibonacci number. Use memoization for efficiency.

[turn 001] TOOL_REQUEST (tool=create_file, request_id=req-1):
{"path": "src/fib.rs", "content": "use std::collections::HashMap;\n\npub fn calculate_fibonacci(n: u64) -> u64 {\n    let mut memo = HashMap::new();\n    fib_memo(n, &mut memo)\n}\n\nfn fib_memo(n: u64, memo: &mut HashMap<u64, u64>) -> u64 {\n    if n <= 1 {\n        return n;\n    }\n    if let Some(&result) = memo.get(&n) {\n        return result;\n    }\n    let result = fib_memo(n - 1, memo) + fib_memo(n - 2, memo);\n    memo.insert(n, result);\n    result\n}\n"}

[turn 001] TOOL_RESULT (tool=create_file, success=true):
File created successfully

[turn 001] ASSISTANT (completed, 1500 in / 200 out tokens):
I've created the `calculate_fibonacci` function in `src/fib.rs`. It uses memoization via a HashMap to efficiently compute Fibonacci numbers.
"#;

    // TODO: Call summarizer and check output
    // let summary = generate_summary(...).await;
    // 
    // Required elements in summary:
    // assert!(summary.contains("calculate_fibonacci") || summary.contains("Fibonacci"));
    // assert!(summary.contains("src/fib.rs"));
    // assert!(summary.contains("memoization") || summary.contains("HashMap"));
    
    println!("EVAL: Summary captures task details");
    println!("Transcript length: {} chars", transcript.len());
}

/// Eval: Summary preserves user constraints
#[tokio::test]
#[ignore]
async fn eval_summary_preserves_constraints() {
    let transcript = r#"
[turn 001] USER:
I need to add logging to the application. Important constraints:
- Use the `tracing` crate, not `log`
- All log messages must include the session ID
- Don't use println! anywhere

[turn 001] ASSISTANT (completed):
I understand. I'll add tracing-based logging with session ID context.

[turn 002] USER:
Also, make sure errors are logged at ERROR level, not WARN.

[turn 002] ASSISTANT (completed):
Got it, I'll use ERROR level for all error conditions.
"#;

    // TODO: Call summarizer and verify constraints are in summary
    // Required: 
    // - "tracing" mentioned (not "log")
    // - "session ID" mentioned
    // - "ERROR level" mentioned
    // - "no println" or equivalent mentioned
    
    println!("EVAL: Summary preserves user constraints");
}

/// Eval: Agent can continue task after compaction
#[tokio::test]
#[ignore]
async fn eval_task_continuity() {
    // This is an end-to-end test:
    // 1. Start a task (e.g., "create a TODO app with these features")
    // 2. Artificially trigger compaction
    // 3. Ask agent to continue
    // 4. Verify agent doesn't ask redundant questions
    
    // Setup: Simulate a half-completed task
    let transcript = r#"
[turn 001] USER:
Create a TODO app with:
- Add/remove items
- Mark as complete
- Save to JSON file

[turn 001] ASSISTANT (completed):
I'll create the TODO app. Let me start with the data structures.

[turn 002] TOOL_REQUEST (tool=create_file):
{"path": "src/todo.rs", "content": "pub struct Todo { id: u64, title: String, completed: bool }"}

[turn 002] TOOL_RESULT (success=true):
Created

[turn 002] ASSISTANT (completed):
I've created the Todo struct. Next I'll add the list management.
"#;

    // TODO: 
    // 1. Generate summary
    // 2. Create new context with summary
    // 3. Send "continue with the implementation"
    // 4. Verify agent knows:
    //    - The task is a TODO app
    //    - Todo struct already exists
    //    - Next step is list management
    //    - Requirements include JSON saving
    
    println!("EVAL: Task continuity after compaction");
}

/// Eval: Verify model context limits are accurate
#[tokio::test]
#[ignore]
async fn eval_model_context_limits() {
    use qbit_context::token_budget::TokenBudgetConfig;
    
    // Test each model's limits against known values
    let test_cases = vec![
        ("claude-3-5-sonnet", 200_000),
        ("claude-4-sonnet", 200_000),
        ("gpt-4o", 128_000),
        ("gpt-4-turbo", 128_000),
        ("gemini-1.5-pro", 1_000_000),
        ("o1-preview", 200_000),
    ];
    
    for (model, expected_limit) in test_cases {
        let config = TokenBudgetConfig::for_model(model);
        assert_eq!(
            config.max_context_tokens, expected_limit,
            "Model {} should have {} token limit, got {}",
            model, expected_limit, config.max_context_tokens
        );
    }
    
    println!("EVAL: Model context limits verified");
}

/// Eval: Measure compaction timing
#[tokio::test]
#[ignore]
async fn eval_compaction_performance() {
    // Create transcripts of various sizes and measure summarization time
    
    let sizes = vec![
        ("small", 5_000),   // ~5k chars
        ("medium", 50_000), // ~50k chars  
        ("large", 200_000), // ~200k chars
    ];
    
    for (label, char_count) in sizes {
        let transcript = "x".repeat(char_count);
        
        let start = std::time::Instant::now();
        // TODO: Call summarizer
        // let _ = generate_summary(&client, &transcript).await;
        let duration = start.elapsed();
        
        println!(
            "EVAL: {} transcript ({} chars) - summarized in {:?}",
            label, char_count, duration
        );
        
        // Acceptable thresholds (adjust based on model)
        // match label {
        //     "small" => assert!(duration < Duration::from_secs(10)),
        //     "medium" => assert!(duration < Duration::from_secs(30)),
        //     "large" => assert!(duration < Duration::from_secs(60)),
        //     _ => {}
        // }
    }
}

/// Eval: Compaction frequency analysis
#[tokio::test]
#[ignore]
async fn eval_compaction_frequency() {
    // Simulate a long conversation and count how often compaction would trigger
    
    // Assumptions:
    // - Average user message: 500 tokens
    // - Average assistant response: 1000 tokens
    // - Average tool call cycle: 2000 tokens
    // - Model limit: 200,000 tokens
    // - Threshold: 80% (160,000 tokens)
    
    let tokens_per_turn = 3500; // user + response + some tool calls
    let threshold_tokens = 160_000;
    
    let turns_until_compaction = threshold_tokens / tokens_per_turn;
    
    println!(
        "EVAL: At ~{} tokens/turn with 80% threshold on 200k model:",
        tokens_per_turn
    );
    println!(
        "  - Compaction triggers after ~{} turns",
        turns_until_compaction
    );
    println!(
        "  - That's about {} hour(s) of active use",
        turns_until_compaction / 20 // assuming ~20 turns/hour
    );
    
    // This should be a reasonable user experience
    assert!(turns_until_compaction >= 30, "Compaction too frequent");
    assert!(turns_until_compaction <= 100, "Compaction too infrequent (risk of overflow)");
}
```

### 8.2 Create summary quality metrics

**File:** `backend/crates/qbit/tests/compaction_eval.rs` (continued)

```rust
/// Metrics for evaluating summary quality.
struct SummaryQualityMetrics {
    /// Does summary mention the original task?
    has_task: bool,
    /// Does summary list completed work?
    has_completed_work: bool,
    /// Does summary note pending work?
    has_pending_work: bool,
    /// Does summary include key file paths?
    has_file_paths: bool,
    /// Does summary preserve user constraints?
    has_constraints: bool,
    /// Estimated summary length (chars)
    summary_length: usize,
    /// Is summary concise (< 2000 chars)?
    is_concise: bool,
}

impl SummaryQualityMetrics {
    fn evaluate(summary: &str, expected: &ExpectedContent) -> Self {
        Self {
            has_task: expected.task_keywords.iter().any(|k| summary.contains(k)),
            has_completed_work: summary.contains("done") || 
                                summary.contains("completed") || 
                                summary.contains("created"),
            has_pending_work: summary.contains("pending") || 
                              summary.contains("next") || 
                              summary.contains("remaining"),
            has_file_paths: expected.file_paths.iter().any(|p| summary.contains(p)),
            has_constraints: expected.constraints.iter().all(|c| summary.contains(c)),
            summary_length: summary.len(),
            is_concise: summary.len() < 2000,
        }
    }

    fn score(&self) -> f64 {
        let checks = [
            self.has_task,
            self.has_completed_work,
            self.has_pending_work,
            self.has_file_paths,
            self.has_constraints,
            self.is_concise,
        ];
        
        let passed = checks.iter().filter(|&&c| c).count();
        passed as f64 / checks.len() as f64
    }
}

struct ExpectedContent {
    task_keywords: Vec<&'static str>,
    file_paths: Vec<&'static str>,
    constraints: Vec<&'static str>,
}
```

### 8.3 Create threshold tuning analysis

**File:** Create analysis script or test

```rust
/// Analyze optimal threshold settings.
#[tokio::test]
#[ignore]
async fn eval_threshold_analysis() {
    // Test different thresholds and measure:
    // 1. How often compaction triggers
    // 2. How much "runway" is left after compaction
    // 3. Risk of overflow
    
    let thresholds = vec![0.70, 0.75, 0.80, 0.85, 0.90];
    let model_limit = 200_000;
    
    for threshold in thresholds {
        let trigger_point = (model_limit as f64 * threshold) as usize;
        let runway_after = model_limit - trigger_point;
        let runway_turns = runway_after / 3500; // tokens per turn estimate
        
        println!(
            "Threshold {:.0}%: triggers at {} tokens, {} token runway (~{} turns)",
            threshold * 100.0,
            trigger_point,
            runway_after,
            runway_turns
        );
        
        // Recommendation:
        // - Too low (70%): Compacts too often, user experience suffers
        // - Sweet spot (80%): Good balance of runway and frequency
        // - Too high (90%): Risk of overflow if long tool response
    }
}
```

### 8.4 Define success criteria

Create a document with clear pass/fail criteria:

```markdown
## Compaction Eval Success Criteria

### Summary Quality (must pass all)
- [ ] Summary includes original task description
- [ ] Summary lists key files created/modified
- [ ] Summary preserves user-specified constraints
- [ ] Summary is under 2000 characters
- [ ] Summary is valid markdown

### Task Continuity (must pass all)
- [ ] Agent can continue interrupted task without asking redundant questions
- [ ] Agent remembers file paths from before compaction
- [ ] Agent remembers user preferences from before compaction
- [ ] Agent doesn't repeat already-completed work

### Performance (must pass all)
- [ ] Small transcript (<10k chars): compacts in <10s
- [ ] Medium transcript (10k-50k chars): compacts in <30s
- [ ] Large transcript (50k-200k chars): compacts in <60s

### Threshold (must pass all)
- [ ] 80% threshold provides at least 5 turns of runway
- [ ] Compaction doesn't trigger before turn 30 in normal use
- [ ] No context overflow errors in 100 test sessions

### Model Limits (must pass all)
- [ ] All Claude models: verified 200k limit
- [ ] GPT-4o: verified 128k limit
- [ ] Gemini Pro: verified 1M limit
- [ ] Default fallback: 128k (safe for most models)
```

### 8.5 Create eval reporting

**File:** Create script to generate eval report

```rust
/// Generate a comprehensive eval report.
async fn generate_eval_report() -> EvalReport {
    let mut report = EvalReport::new();
    
    // Run each eval category
    report.add_section("Summary Quality", run_summary_quality_evals().await);
    report.add_section("Task Continuity", run_continuity_evals().await);
    report.add_section("Performance", run_performance_evals().await);
    report.add_section("Thresholds", run_threshold_evals().await);
    report.add_section("Model Limits", run_model_limit_evals().await);
    
    report
}

struct EvalReport {
    sections: Vec<EvalSection>,
    overall_pass: bool,
}

struct EvalSection {
    name: String,
    tests: Vec<EvalTest>,
    pass_rate: f64,
}

struct EvalTest {
    name: String,
    passed: bool,
    notes: String,
}
```

---

## Running the Eval

### Prerequisites
1. LLM API access configured
2. Sufficient API quota for ~50-100 LLM calls
3. Test data (real or synthetic conversations)

### Commands
```bash
# Run all eval tests
cargo test -p qbit compaction_eval -- --ignored --nocapture

# Run specific eval
cargo test -p qbit eval_summary_captures_task -- --ignored --nocapture

# Generate report (if implemented as binary)
cargo run -p qbit --bin eval_report
```

### Output
- Console output with pass/fail for each test
- Timing measurements for performance evals
- Recommendations for threshold tuning

---

## Definition of Done

- [ ] Eval test harness created
- [ ] At least 5 summary quality test cases
- [ ] At least 3 task continuity test cases
- [ ] Performance benchmarks established
- [ ] Threshold analysis completed
- [ ] Model limits verified against provider docs
- [ ] Success criteria documented
- [ ] All evals passing or issues documented
- [ ] Recommendations for any needed adjustments

---

## Post-Eval Actions

Based on eval results, may need to:

1. **Adjust summarizer prompt** if summaries miss key details
2. **Tune threshold** if compaction is too frequent/infrequent
3. **Update model limits** if values are incorrect
4. **Add special handling** for edge cases discovered

---

## Notes

- Evals require real LLM calls - not for CI
- Run evals periodically as models change
- Keep eval transcripts for regression testing
- Consider A/B testing different summarizer prompts
- Monitor production metrics after release
