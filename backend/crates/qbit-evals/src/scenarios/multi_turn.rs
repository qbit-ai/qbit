//! Multi-turn conversation scenarios.
//!
//! These scenarios test the agent's ability to maintain conversation context
//! across multiple turns, which is critical for OpenAI Responses API compatibility
//! where reasoning IDs must be preserved in history.

use anyhow::Result;
use async_trait::async_trait;

use crate::metrics::{self, Metric};
use crate::outcome::EvalReport;
use crate::runner::EvalRunner;
use crate::scenarios::Scenario;

/// Scenario that tests multi-turn conversation with file operations.
///
/// This scenario:
/// 1. First turn: Creates a file with specific content
/// 2. Second turn: Reads the file and modifies it based on context
///
/// This tests that:
/// - Conversation history is properly maintained
/// - Reasoning IDs are preserved for OpenAI models
/// - Tool calls work correctly across turns
pub struct MultiTurnFileScenario;

#[async_trait]
impl Scenario for MultiTurnFileScenario {
    fn name(&self) -> &str {
        "multi-turn-file"
    }

    fn description(&self) -> &str {
        "Create and modify a file across multiple conversation turns"
    }

    fn testbed(&self) -> &str {
        "empty"
    }

    fn prompt(&self) -> &str {
        // This is the first turn prompt; we override run() for multi-turn
        "Create a file called notes.txt with the content 'First note'"
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(metrics::FileStateMetric::exists("file_exists", "notes.txt")),
            Box::new(metrics::FileStateMetric::contains(
                "has_second_note",
                "notes.txt",
                "Second note",
            )),
        ]
    }

    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Run multi-turn conversation
        let prompts = [
            "Create a file called notes.txt with the content 'First note'",
            "Read notes.txt, then add a second line with 'Second note'",
        ];

        let multi_output = runner.run_multi_turn(&workspace, &prompts).await?;

        // Use the last turn's output for the report
        let last_turn = multi_output
            .turns
            .last()
            .cloned()
            .expect("Should have at least one turn");

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            last_turn.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Evaluate metrics
        let ctx = crate::metrics::EvalContext {
            workspace,
            agent_output: last_turn,
            prompt: prompts.join(" -> "),
        };

        for metric in self.metrics() {
            let result = metric.evaluate(&ctx).await?;
            report.add_metric(metric.name(), result);
        }

        // Add multi-turn specific info - use Score to include message
        report.add_metric(
            "turns_completed",
            metrics::MetricResult::Score {
                value: multi_output.turns.len() as f64,
                max: prompts.len() as f64,
            },
        );

        Ok(report)
    }
}

/// Scenario that specifically tests OpenAI reasoning ID preservation.
///
/// This scenario uses tool calls in both turns to ensure that
/// reasoning items are properly associated with function calls
/// in the conversation history.
pub struct MultiTurnReasoningScenario;

#[async_trait]
impl Scenario for MultiTurnReasoningScenario {
    fn name(&self) -> &str {
        "multi-turn-reasoning"
    }

    fn description(&self) -> &str {
        "Test reasoning ID preservation across turns (OpenAI Responses API)"
    }

    fn testbed(&self) -> &str {
        "empty"
    }

    fn prompt(&self) -> &str {
        "List the current directory contents"
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            // Check that the file was created
            Box::new(metrics::FileStateMetric::exists(
                "test_file_exists",
                "test.txt",
            )),
            Box::new(metrics::FileStateMetric::contains(
                "test_file_content",
                "test.txt",
                "hello world",
            )),
        ]
    }

    async fn run(&self, runner: &EvalRunner) -> Result<EvalReport> {
        let start = std::time::Instant::now();

        // Setup testbed
        let workspace = runner.setup_testbed(self.testbed()).await?;

        // Run multi-turn conversation with tool calls in each turn
        // This specifically tests the OpenAI reasoning ID issue where
        // the second turn would fail if reasoning IDs weren't preserved
        let prompts = [
            "List the current directory contents using ls",
            "Now create a file called test.txt with 'hello world'",
            "Read back the file you just created",
        ];

        let multi_output = runner.run_multi_turn(&workspace, &prompts).await?;

        // Use the last turn's output for the report
        let last_turn = multi_output
            .turns
            .last()
            .cloned()
            .expect("Should have at least one turn");

        // Create report
        let mut report = EvalReport::new(
            self.name(),
            last_turn.clone(),
            start.elapsed().as_millis() as u64,
        );

        // Evaluate metrics
        let ctx = crate::metrics::EvalContext {
            workspace,
            agent_output: last_turn,
            prompt: prompts.join(" -> "),
        };

        for metric in self.metrics() {
            let result = metric.evaluate(&ctx).await?;
            report.add_metric(metric.name(), result);
        }

        // Add multi-turn specific metrics
        report.add_metric(
            "turns_completed",
            metrics::MetricResult::Score {
                value: multi_output.turns.len() as f64,
                max: prompts.len() as f64,
            },
        );

        // Check that all turns completed with tool calls
        let total_tool_calls: usize = multi_output.turns.iter().map(|t| t.tool_calls.len()).sum();

        // If we got here without error, the reasoning ID preservation worked
        if total_tool_calls > 0 {
            report.add_metric("tool_calls_succeeded", metrics::MetricResult::Pass);
        } else {
            report.add_metric(
                "tool_calls_succeeded",
                metrics::MetricResult::Fail {
                    reason: "No tool calls were made".to_string(),
                },
            );
        }

        Ok(report)
    }
}

/// Embedded testbed for empty workspace.
pub fn empty_testbed() -> Vec<(String, String)> {
    // Return empty - workspace will just be an empty directory
    vec![]
}
