//! PR check scenario - lightweight multi-turn scenario for CI.
//!
//! This scenario is designed to run quickly and cheaply in PR checks while
//! still verifying core agent capabilities:
//! - Tool awareness
//! - Sub-agent awareness
//! - Basic tool operations (list, create, edit, grep, ast-grep)
//! - Sub-agent delegation
//! - General response quality

use anyhow::Result;
use async_trait::async_trait;

use crate::metrics::{self, Metric};
use crate::outcome::EvalReport;
use crate::runner::EvalRunner;
use crate::scenarios::Scenario;

/// Lightweight multi-turn scenario for PR checks.
///
/// Tests core agent capabilities in a single scenario:
/// 1. Tool awareness - agent knows what tools it has
/// 2. Sub-agent awareness - agent knows what sub-agents it has
/// 3. File operations - list, create, edit files
/// 4. Search operations - grep and ast-grep
/// 5. Sub-agent delegation - coder creates file, coder edits file
/// 6. File deletion via executor sub-agent
/// 7. Creative response - generate a poem about AI evals
pub struct PrCheckScenario;

#[async_trait]
impl Scenario for PrCheckScenario {
    fn name(&self) -> &str {
        "pr-check"
    }

    fn description(&self) -> &str {
        "Lightweight multi-turn scenario testing tool awareness, file ops, search, sub-agents, and creativity"
    }

    fn testbed(&self) -> &str {
        "pr-check"
    }

    fn prompt(&self) -> &str {
        // First turn prompt - we override run() for multi-turn
        "Which tools do you have access to? List them briefly."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            // Verify tool awareness
            Box::new(metrics::LlmJudgeMetric::with_criteria(
                "tool_awareness",
                "The agent's response mentions core tools like read_file, edit_file, create_file, \
                 grep_file, list_files, ast_grep, and run_command (or run_pty_cmd). \
                 It should list multiple file operation and search tools.",
            )),
            // Verify sub-agent awareness
            Box::new(metrics::LlmJudgeMetric::with_criteria(
                "sub_agent_awareness",
                "The agent's response mentions sub-agents including coder, analyzer, explorer, \
                 researcher, and executor. It should describe their purposes briefly.",
            )),
            // Verify main agent file was created
            Box::new(metrics::FileStateMetric::exists(
                "file_created",
                "src/lib.rs",
            )),
            // Verify main agent file was edited
            Box::new(metrics::FileStateMetric::contains(
                "file_edited",
                "src/lib.rs",
                "modified",
            )),
            // Verify coder sub-agent created the file
            Box::new(metrics::FileStateMetric::exists(
                "coder_file_created",
                "src/greeting.rs",
            )),
            // Verify coder sub-agent edited the file
            Box::new(metrics::FileStateMetric::contains(
                "coder_file_edited",
                "src/greeting.rs",
                "edited",
            )),
            // Verify file was deleted (src/temp.rs should NOT exist after deletion)
            Box::new(metrics::FileStateMetric::not_exists(
                "file_deleted",
                "src/temp.rs",
            )),
            // Verify poem quality
            Box::new(metrics::LlmJudgeMetric::with_criteria(
                "poem_quality",
                "The agent created a short poem (4-8 lines) about AI evals - why they are \
                 powerful and beautiful, and what life would be like without them. The poem \
                 should be creative, thoughtful, and genuinely reflect on the value of evals.",
            )),
        ]
    }

    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Multi-turn conversation testing various capabilities
        let prompts = [
            // Turn 1: Tool awareness
            "Which tools do you have access to? List the main ones briefly.",
            // Turn 2: Sub-agent awareness
            "Which sub-agents do you have access to? Briefly describe each one.",
            // Turn 3: File operations - list, create
            "In this workspace, list the files in the current directory. Then create a new file \
             at src/lib.rs with the content: `pub fn hello() -> &'static str { \"Hello\" }`",
            // Turn 4: Edit and grep
            "Edit src/lib.rs to add a comment '// modified by agent' at the top of the file. \
             Then use grep to find the word 'modified' in the file.",
            // Turn 5: AST-grep test
            "Use ast_grep to find function definitions in src/lib.rs. Show me the pattern you used.",
            // Turn 6: Sub-agent delegation - coder creates file
            "Delegate a task to the coder sub-agent: create a new file src/greeting.rs with a \
             function `pub fn greet(name: &str) -> String` that returns a formatted greeting. \
             The coder should use /dev/null as the source in the diff to create a new file.",
            // Turn 7: Sub-agent delegation - coder edits file
            "Delegate another task to the coder sub-agent: edit src/greeting.rs to add a comment \
             '// edited by coder' at the top of the file.",
            // Turn 8: Sub-agent delegation - executor deletes file
            "Delegate a task to the executor sub-agent: delete the file src/temp.rs using the rm command.",
            // Turn 9: Creative response
            "Write a short poem (4-8 lines) about AI evals - why they are powerful and beautiful, \
             and what life would be like without them. Be creative and thoughtful!",
        ];

        let multi_output = runner.run_multi_turn(&workspace, &prompts).await?;

        // Collect all responses for metrics evaluation
        let all_responses: String = multi_output
            .turns
            .iter()
            .enumerate()
            .map(|(i, turn)| format!("Turn {}: {}", i + 1, turn.response))
            .collect::<Vec<_>>()
            .join("\n\n");

        // Use the last turn's output for the base report
        let last_turn = multi_output
            .turns
            .last()
            .cloned()
            .expect("Should have at least one turn");

        // Create a combined agent output for metrics that includes all responses
        let combined_output = crate::runner::AgentOutput {
            response: all_responses,
            tool_calls: multi_output
                .turns
                .iter()
                .flat_map(|t| t.tool_calls.clone())
                .collect(),
            files_modified: multi_output
                .turns
                .iter()
                .flat_map(|t| t.files_modified.clone())
                .collect(),
            duration_ms: multi_output.total_duration_ms,
            tokens_used: last_turn.tokens_used,
        };

        // Create report with prompts for transcript output
        let mut report = EvalReport::new_with_prompts(
            self.name(),
            combined_output.clone(),
            start.elapsed().as_millis() as u64,
            prompts.iter().map(|s| s.to_string()).collect(),
        );

        // Evaluate metrics
        let ctx = crate::metrics::EvalContext {
            workspace,
            agent_output: combined_output,
            prompt: prompts.join(" -> "),
        };

        for metric in self.metrics() {
            let result = metric.evaluate(&ctx).await?;
            report.add_metric(metric.name(), result);
        }

        // Add multi-turn specific info
        report.add_metric(
            "turns_completed",
            metrics::MetricResult::Score {
                value: multi_output.turns.len() as f64,
                max: prompts.len() as f64,
            },
        );

        // Count total tool calls across all turns
        let total_tool_calls: usize = multi_output.turns.iter().map(|t| t.tool_calls.len()).sum();
        if total_tool_calls >= 5 {
            report.add_metric("sufficient_tool_usage", metrics::MetricResult::Pass);
        } else {
            report.add_metric(
                "sufficient_tool_usage",
                metrics::MetricResult::Fail {
                    reason: format!("Expected at least 5 tool calls, got {}", total_tool_calls),
                },
            );
        }

        Ok(report)
    }
}

/// Testbed files for the pr-check scenario.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        // Create a minimal project structure
        (
            "Cargo.toml".to_string(),
            r#"[package]
name = "pr-check-testbed"
version = "0.1.0"
edition = "2021"
"#
            .to_string(),
        ),
        // Create src directory with a placeholder
        (
            "src/.gitkeep".to_string(),
            "# Placeholder to ensure src directory exists\n".to_string(),
        ),
        // A temp file that will be deleted by the executor sub-agent
        (
            "src/temp.rs".to_string(),
            "// This file will be deleted by the executor sub-agent\npub fn temp() {}\n"
                .to_string(),
        ),
        // README for context
        (
            "README.md".to_string(),
            "# PR Check Testbed\n\nA minimal Rust project for testing agent capabilities.\n"
                .to_string(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_name() {
        let scenario = PrCheckScenario;
        assert_eq!(scenario.name(), "pr-check");
    }

    #[test]
    fn test_testbed_files() {
        let files = testbed_files();
        assert!(!files.is_empty());

        // Should have Cargo.toml
        assert!(files.iter().any(|(path, _)| path == "Cargo.toml"));
    }

    #[test]
    fn test_metrics_count() {
        let scenario = PrCheckScenario;
        let metrics = scenario.metrics();
        // Should have: tool_awareness, sub_agent_awareness, file_created, file_edited,
        // coder_file_created, coder_file_edited, file_deleted, poem_quality
        assert_eq!(metrics.len(), 8);
    }
}
