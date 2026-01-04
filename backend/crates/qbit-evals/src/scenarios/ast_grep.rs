//! AST-grep evaluation scenario.
//!
//! Tests the agent's ability to use the ast_grep tool for structural code search
//! and the ast_grep_replace tool for refactoring.

use async_trait::async_trait;

use crate::metrics::{FileStateMetric, LlmScoreMetric, Metric};
use crate::scenarios::Scenario;

/// Scenario: Find all console.log calls using AST patterns.
///
/// This scenario tests the agent's ability to:
/// 1. Use ast_grep for structural code search (not regex)
/// 2. Identify matching patterns across multiple files
/// 3. Report results accurately
pub struct AstGrepSearchScenario;

#[async_trait]
impl Scenario for AstGrepSearchScenario {
    fn name(&self) -> &str {
        "ast-grep-search"
    }

    fn description(&self) -> &str {
        "Use ast_grep to find all console.log calls with a string argument in a JavaScript project"
    }

    fn testbed(&self) -> &str {
        "js-ast-grep"
    }

    fn prompt(&self) -> &str {
        "Use the ast_grep tool to find all console.log calls in this JavaScript project. \
         Report how many you found and list the files containing them. \
         Do NOT use grep or regex - use the ast_grep tool for structural search."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            Box::new(LlmScoreMetric::scale_10(
                "tool_usage",
                "Did the agent use ast_grep (not grep or regex) to search for console.log patterns? \
                 Score 1 if grep/regex was used instead, 10 if ast_grep was used correctly.",
                7.0,
            )),
            Box::new(LlmScoreMetric::scale_10(
                "accuracy",
                "Did the agent correctly identify that there are 4 console.log calls across 3 files \
                 (app.js: 2, utils.js: 1, logger.js: 1)?",
                7.0,
            )),
        ]
    }
}

/// Scenario: Replace console.log with logger.info using AST patterns.
///
/// This scenario tests the agent's ability to:
/// 1. Use ast_grep_replace for structural refactoring
/// 2. Preserve captured variables in replacements
/// 3. Make changes across multiple files
pub struct AstGrepReplaceScenario;

#[async_trait]
impl Scenario for AstGrepReplaceScenario {
    fn name(&self) -> &str {
        "ast-grep-replace"
    }

    fn description(&self) -> &str {
        "Use ast_grep_replace to replace console.log with logger.info across a JavaScript project"
    }

    fn testbed(&self) -> &str {
        "js-ast-grep"
    }

    fn prompt(&self) -> &str {
        "Use the ast_grep_replace tool to replace all console.log($MSG) calls with logger.info($MSG) \
         in this JavaScript project. The variable $MSG should be preserved in the replacement. \
         Make the changes to all JavaScript files."
    }

    fn metrics(&self) -> Vec<Box<dyn Metric>> {
        vec![
            // Check that app.js was modified to use logger.info
            Box::new(FileStateMetric::contains(
                "app_uses_logger",
                "src/app.js",
                "logger.info",
            )),
            Box::new(FileStateMetric::contains(
                "utils_uses_logger",
                "src/utils.js",
                "logger.info",
            )),
            Box::new(LlmScoreMetric::scale_10(
                "console_log_removed",
                "Did the agent successfully replace console.log with logger.info in all files? \
                 Check that console.log calls are no longer present in app.js and utils.js.",
                7.0,
            )),
            Box::new(LlmScoreMetric::scale_10(
                "tool_usage",
                "Did the agent use ast_grep_replace (not manual editing) to make the changes? \
                 Score 1 if manual editing was used, 10 if ast_grep_replace was used correctly.",
                7.0,
            )),
        ]
    }
}

/// Testbed files for the AST-grep scenarios.
///
/// Creates a simple JavaScript project with console.log calls in multiple files.
pub fn testbed_files() -> Vec<(String, String)> {
    vec![
        (
            "package.json".to_string(),
            r#"{
  "name": "ast-grep-testbed",
  "version": "1.0.0",
  "main": "src/app.js"
}
"#
            .to_string(),
        ),
        (
            "src/app.js".to_string(),
            r#"// Main application file
import { formatDate } from './utils.js';
import { log } from './logger.js';

function main() {
    console.log('Starting application...');
    const date = formatDate(new Date());
    console.log('Current date: ' + date);
    log('Application started');
}

main();
"#
            .to_string(),
        ),
        (
            "src/utils.js".to_string(),
            r#"// Utility functions
export function formatDate(date) {
    console.log('Formatting date...');
    return date.toISOString().split('T')[0];
}

export function formatNumber(num) {
    return num.toLocaleString();
}
"#
            .to_string(),
        ),
        (
            "src/logger.js".to_string(),
            r#"// Custom logger module
const prefix = '[APP]';

export function log(message) {
    console.log(prefix + ' ' + message);
}

export function error(message) {
    console.error(prefix + ' ERROR: ' + message);
}
"#
            .to_string(),
        ),
    ]
}
