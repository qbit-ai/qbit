//! Tool execution logic for the agent bridge.
//!
//! This module contains the logic for executing various types of tools:
//! - Indexer tools (code search, file analysis)
//! - Tavily tools (web search)
//! - Plan tools (task planning and tracking)
//!
//! Note: Workflow tool execution is handled in the qbit crate to avoid
//! circular dependencies with WorkflowState and BridgeLlmExecutor types.

use std::path::PathBuf;
use std::sync::Arc;

use serde_json::json;
use vtcode_core::tools::tree_sitter::analysis::CodeAnalyzer;

use qbit_core::events::AiEvent;

// NOTE: Workflow tool execution (WorkflowState, BridgeLlmExecutor, execute_workflow_tool)
// has been moved to qbit crate to avoid circular dependencies
use qbit_indexer::IndexerState;
use qbit_web::tavily::TavilyState;
use qbit_web::web_fetch::WebFetcher;

/// Result type for tool execution: (json_result, success_flag)
type ToolResult = (serde_json::Value, bool);

/// Helper to create an error result
fn error_result(msg: impl Into<String>) -> ToolResult {
    (json!({"error": msg.into()}), false)
}

/// Execute an indexer tool by name.
pub async fn execute_indexer_tool(
    indexer_state: Option<&Arc<IndexerState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> ToolResult {
    let Some(indexer) = indexer_state else {
        return error_result("Indexer not initialized");
    };

    // Helper to get a string argument
    let get_str = |key: &str| args.get(key).and_then(|v| v.as_str()).unwrap_or("");

    match tool_name {
        "indexer_search_code" => {
            let pattern = get_str("pattern");
            let path_filter = args.get("path_filter").and_then(|v| v.as_str());

            match indexer.with_indexer(|idx| idx.search(pattern, path_filter)) {
                Ok(results) => (
                    json!({
                        "matches": results.iter().map(|r| json!({
                            "file": r.file_path,
                            "line": r.line_number,
                            "content": r.line_content,
                            "matches": r.matches
                        })).collect::<Vec<_>>(),
                        "count": results.len()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        "indexer_search_files" => {
            let pattern = get_str("pattern");

            match indexer.with_indexer(|idx| idx.find_files(pattern)) {
                Ok(files) => (json!({"files": files, "count": files.len()}), true),
                Err(e) => error_result(e.to_string()),
            }
        }
        "indexer_analyze_file" => {
            let file_path = get_str("file_path");
            let mut analyzer = match indexer.get_analyzer() {
                Ok(a) => a,
                Err(e) => return error_result(e.to_string()),
            };

            let path = PathBuf::from(file_path);
            let tree = match analyzer.parse_file(&path).await {
                Ok(t) => t,
                Err(e) => return error_result(format!("Failed to parse file: {}", e)),
            };

            let code_analyzer = CodeAnalyzer::new(&tree.language);
            let analysis = code_analyzer.analyze(&tree, file_path);

            (
                json!({
                    "symbols": analysis.symbols.iter().map(|s| json!({
                        "name": s.name,
                        "kind": format!("{:?}", s.kind),
                        "line": s.position.row,
                        "column": s.position.column
                    })).collect::<Vec<_>>(),
                    "metrics": {
                        "lines_of_code": analysis.metrics.lines_of_code,
                        "lines_of_comments": analysis.metrics.lines_of_comments,
                        "blank_lines": analysis.metrics.blank_lines,
                        "functions_count": analysis.metrics.functions_count,
                        "classes_count": analysis.metrics.classes_count,
                        "comment_ratio": analysis.metrics.comment_ratio
                    },
                    "dependencies": analysis.dependencies.iter().map(|d| json!({
                        "name": d.name,
                        "kind": format!("{:?}", d.kind)
                    })).collect::<Vec<_>>()
                }),
                true,
            )
        }
        "indexer_extract_symbols" => {
            use vtcode_core::tools::tree_sitter::languages::LanguageAnalyzer;

            let file_path = get_str("file_path");
            let mut analyzer = match indexer.get_analyzer() {
                Ok(a) => a,
                Err(e) => return error_result(e.to_string()),
            };

            let path = PathBuf::from(file_path);
            let tree = match analyzer.parse_file(&path).await {
                Ok(t) => t,
                Err(e) => return error_result(format!("Failed to parse file: {}", e)),
            };

            let lang_analyzer = LanguageAnalyzer::new(&tree.language);
            let symbols = lang_analyzer.extract_symbols(&tree);

            (
                json!({
                    "symbols": symbols.iter().map(|s| json!({
                        "name": s.name,
                        "kind": format!("{:?}", s.kind),
                        "line": s.position.row,
                        "column": s.position.column,
                        "scope": s.scope,
                        "signature": s.signature,
                        "documentation": s.documentation
                    })).collect::<Vec<_>>(),
                    "count": symbols.len()
                }),
                true,
            )
        }
        "indexer_get_metrics" => {
            let file_path = get_str("file_path");
            let mut analyzer = match indexer.get_analyzer() {
                Ok(a) => a,
                Err(e) => return error_result(e.to_string()),
            };

            let path = PathBuf::from(file_path);
            let tree = match analyzer.parse_file(&path).await {
                Ok(t) => t,
                Err(e) => return error_result(format!("Failed to parse file: {}", e)),
            };

            let code_analyzer = CodeAnalyzer::new(&tree.language);
            let metrics = code_analyzer.analyze(&tree, file_path).metrics;

            (
                json!({
                    "lines_of_code": metrics.lines_of_code,
                    "lines_of_comments": metrics.lines_of_comments,
                    "blank_lines": metrics.blank_lines,
                    "functions_count": metrics.functions_count,
                    "classes_count": metrics.classes_count,
                    "variables_count": metrics.variables_count,
                    "imports_count": metrics.imports_count,
                    "comment_ratio": metrics.comment_ratio
                }),
                true,
            )
        }
        "indexer_detect_language" => {
            let file_path = get_str("file_path");
            let analyzer = match indexer.get_analyzer() {
                Ok(a) => a,
                Err(e) => return error_result(e.to_string()),
            };

            let path = PathBuf::from(file_path);
            match analyzer.detect_language_from_path(&path) {
                Ok(lang) => (json!({"language": format!("{:?}", lang)}), true),
                Err(e) => error_result(e.to_string()),
            }
        }
        _ => error_result(format!("Unknown indexer tool: {}", tool_name)),
    }
}

/// Execute a Tavily web search tool.
pub async fn execute_tavily_tool(
    tavily_state: Option<&Arc<TavilyState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> ToolResult {
    let Some(tavily) = tavily_state else {
        return error_result("Web search not available - TAVILY_API_KEY not configured");
    };

    // Helper to get a string argument
    let get_str = |key: &str| args.get(key).and_then(|v| v.as_str()).unwrap_or("");

    match tool_name {
        "web_search" => {
            let query = get_str("query");
            let max_results = args
                .get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize);

            match tavily.search(query, max_results).await {
                Ok(results) => (
                    json!({
                        "query": results.query,
                        "results": results.results.iter().map(|r| json!({
                            "title": r.title,
                            "url": r.url,
                            "content": r.content,
                            "score": r.score
                        })).collect::<Vec<_>>(),
                        "answer": results.answer,
                        "count": results.results.len()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        "web_search_answer" => {
            let query = get_str("query");

            match tavily.answer(query).await {
                Ok(result) => (
                    json!({
                        "query": result.query,
                        "answer": result.answer,
                        "sources": result.sources.iter().map(|r| json!({
                            "title": r.title,
                            "url": r.url,
                            "content": r.content,
                            "score": r.score
                        })).collect::<Vec<_>>()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        "web_extract" => {
            let urls: Vec<String> = args
                .get("urls")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            match tavily.extract(urls).await {
                Ok(results) => (
                    json!({
                        "results": results.results.iter().map(|r| json!({
                            "url": r.url,
                            "content": r.raw_content
                        })).collect::<Vec<_>>(),
                        "failed_urls": results.failed_urls,
                        "count": results.results.len()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        "web_crawl" => {
            let url = get_str("url");
            let max_depth = args
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            match tavily.crawl(url.to_string(), max_depth).await {
                Ok(results) => (
                    json!({
                        "results": results.results.iter().map(|r| json!({
                            "url": r.url,
                            "content": r.raw_content
                        })).collect::<Vec<_>>(),
                        "failed_urls": results.failed_urls,
                        "count": results.results.len()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        "web_map" => {
            let url = get_str("url");
            let max_depth = args
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);

            match tavily.map(url.to_string(), max_depth).await {
                Ok(results) => (
                    json!({
                        "urls": results.urls,
                        "base_url": results.base_url,
                        "count": results.urls.len()
                    }),
                    true,
                ),
                Err(e) => error_result(e.to_string()),
            }
        }
        _ => error_result(format!("Unknown web search tool: {}", tool_name)),
    }
}

/// Execute a web fetch tool using readability-based content extraction.
pub async fn execute_web_fetch_tool(tool_name: &str, args: &serde_json::Value) -> ToolResult {
    if tool_name != "web_fetch" {
        return error_result(format!("Unknown web fetch tool: {}", tool_name));
    }

    // web_fetch expects a single "url" parameter (not "urls" array)
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => {
            return error_result(
                "web_fetch requires a 'url' parameter (string). Example: {\"url\": \"https://example.com\"}"
            )
        }
    };

    let fetcher = WebFetcher::new();

    match fetcher.fetch(&url).await {
        Ok(result) => (
            json!({
                "url": result.url,
                "content": result.content
            }),
            true,
        ),
        Err(e) => error_result(format!("Failed to fetch {}: {}", url, e)),
    }
}

/// Execute the update_plan tool.
///
/// Updates the task plan with new steps and their statuses.
/// Emits a PlanUpdated event when the plan is successfully updated.
pub async fn execute_plan_tool(
    plan_manager: &Arc<qbit_planner::PlanManager>,
    event_tx: &tokio::sync::mpsc::UnboundedSender<AiEvent>,
    args: &serde_json::Value,
) -> ToolResult {
    // Parse the arguments into UpdatePlanArgs
    let update_args: qbit_planner::UpdatePlanArgs = match serde_json::from_value(args.clone()) {
        Ok(a) => a,
        Err(e) => return error_result(format!("Invalid update_plan arguments: {}", e)),
    };

    // Update the plan
    match plan_manager.update_plan(update_args).await {
        Ok(plan) => {
            // Emit PlanUpdated event
            let _ = event_tx.send(AiEvent::PlanUpdated {
                version: plan.version,
                summary: plan.summary.clone(),
                steps: plan.steps.clone(),
                explanation: None,
            });

            (
                json!({
                    "success": true,
                    "version": plan.version,
                    "summary": {
                        "total": plan.summary.total,
                        "completed": plan.summary.completed,
                        "in_progress": plan.summary.in_progress,
                        "pending": plan.summary.pending
                    }
                }),
                true,
            )
        }
        Err(e) => error_result(format!("Failed to update plan: {}", e)),
    }
}

/// Normalize tool arguments for run_pty_cmd.
/// If the command is passed as an array, convert it to a space-joined string.
/// This prevents shell_words::join() from quoting metacharacters like &&, ||, |, etc.
pub fn normalize_run_pty_cmd_args(mut args: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = args.as_object_mut() {
        if let Some(command) = obj.get_mut("command") {
            if let Some(arr) = command.as_array() {
                // Convert array to space-joined string
                let cmd_str: String = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                *command = serde_json::Value::String(cmd_str);
            }
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_run_pty_cmd_array_to_string() {
        // Command as array with shell operators
        let args = json!({
            "command": ["cd", "/path", "&&", "pwd"],
            "cwd": "."
        });

        let normalized = normalize_run_pty_cmd_args(args);

        assert_eq!(normalized["command"].as_str().unwrap(), "cd /path && pwd");
        // Other fields should be preserved
        assert_eq!(normalized["cwd"].as_str().unwrap(), ".");
    }

    #[test]
    fn test_normalize_run_pty_cmd_string_unchanged() {
        // Command already as string - should be unchanged
        let args = json!({
            "command": "cd /path && pwd",
            "cwd": "."
        });

        let normalized = normalize_run_pty_cmd_args(args);

        assert_eq!(normalized["command"].as_str().unwrap(), "cd /path && pwd");
    }

    #[test]
    fn test_normalize_run_pty_cmd_pipe_operator() {
        let args = json!({
            "command": ["ls", "-la", "|", "grep", "foo"]
        });

        let normalized = normalize_run_pty_cmd_args(args);

        assert_eq!(normalized["command"].as_str().unwrap(), "ls -la | grep foo");
    }

    #[test]
    fn test_normalize_run_pty_cmd_redirect() {
        let args = json!({
            "command": ["echo", "hello", ">", "output.txt"]
        });

        let normalized = normalize_run_pty_cmd_args(args);

        assert_eq!(
            normalized["command"].as_str().unwrap(),
            "echo hello > output.txt"
        );
    }

    #[test]
    fn test_normalize_run_pty_cmd_empty_array() {
        let args = json!({
            "command": []
        });

        let normalized = normalize_run_pty_cmd_args(args);

        assert_eq!(normalized["command"].as_str().unwrap(), "");
    }

    #[test]
    fn test_normalize_run_pty_cmd_no_command_field() {
        // Args without command field should pass through unchanged
        let args = json!({
            "cwd": "/some/path"
        });

        let normalized = normalize_run_pty_cmd_args(args.clone());

        assert_eq!(normalized, args);
    }
}
