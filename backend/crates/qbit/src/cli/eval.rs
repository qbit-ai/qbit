//! CLI evaluation runner.
//!
//! Provides the entry point for running evals from the command line.

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use tokio::sync::Semaphore;
use qbit_evals::indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use qbit_evals::outcome::{EvalReport, EvalSummary};
use qbit_evals::runner::EvalRunner;
use qbit_evals::scenarios::{
    all_scenarios, default_scenarios_for_provider, get_openai_model_scenario, get_scenario,
    list_openai_models, openai_model_scenarios, Scenario,
};
use qbit_evals::EvalProvider;
use tracing_subscriber::EnvFilter;

/// Color helpers that respect CI environment.
/// In CI, ANSI escape codes are stripped for cleaner logs.
mod color {
    use std::sync::OnceLock;

    static IS_CI: OnceLock<bool> = OnceLock::new();

    fn is_ci() -> bool {
        *IS_CI.get_or_init(|| std::env::var("CI").map(|v| v == "true").unwrap_or(false))
    }

    pub fn red(s: &str) -> String {
        if is_ci() {
            s.to_string()
        } else {
            format!("\x1b[31m{}\x1b[0m", s)
        }
    }

    pub fn green(s: &str) -> String {
        if is_ci() {
            s.to_string()
        } else {
            format!("\x1b[32m{}\x1b[0m", s)
        }
    }

    pub fn yellow(s: &str) -> String {
        if is_ci() {
            s.to_string()
        } else {
            format!("\x1b[33m{}\x1b[0m", s)
        }
    }

    pub fn cyan(s: &str) -> String {
        if is_ci() {
            s.to_string()
        } else {
            format!("\x1b[36m{}\x1b[0m", s)
        }
    }

    pub fn red_line() -> &'static str {
        if is_ci() {
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        } else {
            "\x1b[31m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        }
    }

    pub fn green_line() -> &'static str {
        if is_ci() {
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
        } else {
            "\x1b[32m━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
        }
    }

    pub fn check_mark() -> &'static str {
        if is_ci() {
            "[PASS]"
        } else {
            "\x1b[32m✓\x1b[0m"
        }
    }

    pub fn x_mark() -> &'static str {
        if is_ci() {
            "[FAIL]"
        } else {
            "\x1b[31m✗\x1b[0m"
        }
    }
}

/// Options for eval output.
pub struct EvalOutputOptions {
    /// Output JSON to stdout.
    pub json: bool,
    /// Pretty print CI-friendly summary.
    pub pretty: bool,
    /// Save JSON results to a file.
    pub output_file: Option<PathBuf>,
    /// Print the full agent transcript before results.
    pub transcript: bool,
}

/// List all available scenarios.
pub fn list_scenarios() {
    println!("Available evaluation scenarios:\n");
    for scenario in all_scenarios() {
        println!("  {} - {}", scenario.name(), scenario.description());
    }
    println!();
}

/// Print the full agent transcript from eval results.
fn print_transcript(summary: &EvalSummary) {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    AGENT TRANSCRIPT");
    println!("═══════════════════════════════════════════════════════════════");

    for report in &summary.reports {
        println!();
        println!("┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("┃ Scenario: {}", report.scenario);
        println!("┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        // Parse the response to separate turns
        let response = &report.agent_output.response;

        // Check if this is a multi-turn response (contains "Turn X:")
        if response.contains("Turn 1:") {
            // Split by "Turn N:" pattern and print each turn separately
            let mut current_turn = String::new();
            let mut current_turn_num = 0;

            for line in response.lines() {
                // Check if this line starts a new turn
                if let Some(rest) = line.strip_prefix("Turn ") {
                    if let Some(colon_pos) = rest.find(':') {
                        if let Ok(num) = rest[..colon_pos].trim().parse::<u32>() {
                            // Print previous turn if exists
                            if current_turn_num > 0 && !current_turn.trim().is_empty() {
                                let prompt = report
                                    .prompts
                                    .get((current_turn_num - 1) as usize)
                                    .map(|s| s.as_str());
                                print_turn(current_turn_num, prompt, &current_turn);
                            }
                            current_turn_num = num;
                            // Start new turn with content after "Turn N:"
                            current_turn = rest[colon_pos + 1..].to_string();
                            continue;
                        }
                    }
                }
                // Add line to current turn
                if current_turn_num > 0 {
                    current_turn.push('\n');
                    current_turn.push_str(line);
                }
            }

            // Print last turn
            if current_turn_num > 0 && !current_turn.trim().is_empty() {
                let prompt = report
                    .prompts
                    .get((current_turn_num - 1) as usize)
                    .map(|s| s.as_str());
                print_turn(current_turn_num, prompt, &current_turn);
            }
        } else {
            // Single turn - just print the response
            println!();
            println!("┌─ Agent Response ─────────────────────────────────────────────");
            for line in response.lines() {
                println!("│ {}", line);
            }
            println!("└───────────────────────────────────────────────────────────────");
        }

        // Print tool calls summary
        if !report.agent_output.tool_calls.is_empty() {
            println!();
            println!(
                "┌─ Tool Calls ({} total) ─────────────────────────────────────",
                report.agent_output.tool_calls.len()
            );
            for tc in &report.agent_output.tool_calls {
                let status = if tc.success { "✓" } else { "✗" };
                println!("│ {} {}", status, tc.name);
            }
            println!("└───────────────────────────────────────────────────────────────");
        }
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!();
}

/// Print a single turn from the transcript.
fn print_turn(turn_num: u32, prompt: Option<&str>, content: &str) {
    println!();
    println!(
        "┌─ Turn {} ─────────────────────────────────────────────────────",
        turn_num
    );
    println!("│ {}:", color::cyan("User"));
    if let Some(p) = prompt {
        for line in p.lines() {
            println!("│   {}", line);
        }
    } else {
        println!("│   [prompt not available]");
    }
    println!("│");
    println!("│ {}:", color::yellow("Agent"));
    for line in content.trim().lines() {
        println!("│   {}", line);
    }
    println!("└───────────────────────────────────────────────────────────────");
}

/// List available OpenAI models for testing.
pub fn list_openai_model_scenarios() {
    println!("Available OpenAI models for connectivity testing:\n");
    for (model_id, model_name) in list_openai_models() {
        println!("  {} - {}", model_id, model_name);
    }
    println!();
    println!("Run with: --openai-models");
    println!("Run specific model: --openai-models --openai-model gpt-5.1");
    println!();
}

/// Run evaluation scenarios.
pub async fn run_evals(
    scenario_filter: Option<&str>,
    json_output: bool,
    verbose: bool,
    parallel: bool,
    provider: EvalProvider,
    output_options: Option<EvalOutputOptions>,
) -> Result<()> {
    // Initialize tracing for evals (since we bypass the normal CLI bootstrap)
    let log_level = if verbose { "debug" } else { "warn" };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("qbit={}", log_level).parse().unwrap())
                .add_directive(format!("qbit_evals={}", log_level).parse().unwrap())
                .add_directive(format!("qbit_ai={}", log_level).parse().unwrap()),
        )
        .try_init();

    let scenarios = if let Some(name) = scenario_filter {
        match get_scenario(name) {
            Some(s) => vec![s],
            None => {
                eprintln!("Unknown scenario: {}", name);
                eprintln!("Use --list-scenarios to see available scenarios");
                anyhow::bail!("Unknown scenario: {}", name);
            }
        }
    } else {
        // Use default scenarios filtered for the selected provider
        // (e.g., web-search is excluded for Z.AI)
        default_scenarios_for_provider(provider)
    };

    // Determine if we should suppress normal output (when using new output options)
    let use_new_output = output_options.is_some();
    let opts = output_options.unwrap_or(EvalOutputOptions {
        json: json_output,
        pretty: false,
        output_file: None,
        transcript: false,
    });

    // Suppress intermediate output when using transcript mode or other new output options
    let suppress_intermediate = use_new_output || opts.transcript;

    if !opts.json && !suppress_intermediate {
        println!("Using LLM provider: {}", provider);
    }

    let summary = if parallel && scenarios.len() > 1 {
        run_parallel(
            scenarios,
            opts.json,
            verbose,
            provider,
            suppress_intermediate,
        )
        .await?
    } else {
        run_sequential(
            scenarios,
            opts.json,
            verbose,
            provider,
            suppress_intermediate,
        )
        .await?
    };

    // Handle output based on options
    if let Some(ref output_path) = opts.output_file {
        let file = File::create(output_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &summary.to_json())?;
        eprintln!("Results saved to: {}", output_path.display());
    }

    // Print transcript before results if requested
    if opts.transcript {
        print_transcript(&summary);
    }

    if opts.pretty {
        summary.print_ci_summary(&mut std::io::stdout(), &provider.to_string())?;
    } else if opts.json {
        println!("{}", serde_json::to_string(&summary.to_json())?);
    } else if !use_new_output {
        summary.print_summary(&mut std::io::stdout())?;
    }

    // Print final PASS/FAIL result for GitHub Actions
    // Z.AI uses 80% pass threshold, others require 100%
    let pass_threshold = match provider {
        EvalProvider::Zai => 0.80,
        _ => 1.0,
    };
    let passed = summary.pass_rate() >= pass_threshold;

    println!();
    if !passed {
        println!("{}", color::red_line());
        println!(
            "{}",
            color::red(&format!(
                "  FAIL: {} of {} scenarios failed ({:.0}% pass rate, {:.0}% required)",
                summary.failed_count(),
                summary.reports.len(),
                summary.pass_rate() * 100.0,
                pass_threshold * 100.0
            ))
        );
        println!("{}", color::red_line());
        anyhow::bail!(
            "{} of {} scenarios failed ({:.0}% pass rate, {:.0}% required)",
            summary.failed_count(),
            summary.reports.len(),
            summary.pass_rate() * 100.0,
            pass_threshold * 100.0
        );
    } else {
        println!("{}", color::green_line());
        if summary.failed_count() > 0 {
            println!(
                "{}",
                color::green(&format!(
                    "  PASS: {}/{} scenarios passed ({:.0}% >= {:.0}% threshold)",
                    summary.passed_count(),
                    summary.reports.len(),
                    summary.pass_rate() * 100.0,
                    pass_threshold * 100.0
                ))
            );
        } else {
            println!(
                "{}",
                color::green(&format!(
                    "  PASS: All {} scenarios passed",
                    summary.reports.len()
                ))
            );
        }
        println!("{}", color::green_line());
    }

    Ok(())
}

/// Get the metric pass threshold for a provider.
///
/// Z.AI uses 80% threshold, others require 100%.
fn metric_pass_threshold(provider: EvalProvider) -> f64 {
    match provider {
        EvalProvider::Zai => 0.80,
        _ => 1.0,
    }
}

/// Run scenarios sequentially.
async fn run_sequential(
    scenarios: Vec<Box<dyn Scenario>>,
    json_output: bool,
    verbose: bool,
    provider: EvalProvider,
    quiet: bool,
) -> Result<EvalSummary> {
    let runner = EvalRunner::new_verbose_with_provider(verbose, provider)?;
    let mut summary = EvalSummary::default();
    let threshold = metric_pass_threshold(provider);

    for scenario in scenarios {
        if !json_output && !quiet {
            println!("Running scenario: {}", scenario.name());
        }

        match scenario.run(&runner).await {
            Ok(mut report) => {
                // Apply metric pass threshold (Z.AI uses 80%, others 100%)
                report.apply_pass_threshold(threshold);

                if json_output && !quiet {
                    println!("{}", serde_json::to_string(&report.to_json())?);
                } else if !quiet {
                    report.print_summary(&mut std::io::stdout())?;
                }
                summary.add(report);
            }
            Err(e) => {
                eprintln!("Error running scenario {}: {}", scenario.name(), e);
            }
        }
    }

    Ok(summary)
}

/// Run scenarios in parallel with animated progress display.
async fn run_parallel(
    scenarios: Vec<Box<dyn Scenario>>,
    json_output: bool,
    verbose: bool,
    provider: EvalProvider,
    quiet: bool,
) -> Result<EvalSummary> {
    // Create log directory for verbose output if needed
    let log_dir = if verbose {
        let dir = std::env::temp_dir().join("qbit-eval-logs");
        std::fs::create_dir_all(&dir)?;
        Some(Arc::new(dir))
    } else {
        None
    };

    // For JSON output or quiet mode, use simple execution without progress bars
    if json_output || quiet {
        return run_parallel_simple(scenarios, log_dir, verbose, provider, quiet).await;
    }

    // Create multi-progress display
    let multi_progress = MultiProgress::new();

    // Create a header line
    let header = multi_progress.add(ProgressBar::new_spinner());
    header.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    let scenario_count = scenarios.len();
    if let Some(ref dir) = log_dir {
        header.set_message(format!(
            "Running {} scenarios in parallel (logs: {})",
            scenario_count,
            dir.display()
        ));
    } else {
        header.set_message(format!("Running {} scenarios in parallel", scenario_count));
    }
    header.tick();

    // Spinner style for running scenarios
    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .template("  {spinner:.cyan} {wide_msg}")
        .unwrap();

    // Create progress bars for each scenario
    let progress_bars: Vec<_> = scenarios
        .iter()
        .map(|scenario| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_message(format!("{:<20} running...", scenario.name()));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        })
        .collect();

    // Create futures for each scenario with its own runner
    let futures: Vec<_> = scenarios
        .into_iter()
        .zip(progress_bars.into_iter())
        .map(|(scenario, pb)| {
            let name = scenario.name().to_string();
            let log_dir_clone = log_dir.clone();
            let log_file = log_dir_clone
                .as_ref()
                .map(|dir| dir.join(format!("{}.log", name)));
            let log_path_for_result = log_file.clone();

            async move {
                let runner = if let Some(path) = log_file {
                    match EvalRunner::new_with_log_file_and_provider(path, provider) {
                        Ok(r) => r,
                        Err(e) => {
                            pb.set_style(
                                ProgressStyle::default_spinner()
                                    .template("  {msg}")
                                    .unwrap(),
                            );
                            pb.finish_with_message(format!(
                                "{} {:<20} error: {}",
                                color::x_mark(),
                                name,
                                e
                            ));
                            return (name, Err(e), None::<PathBuf>);
                        }
                    }
                } else {
                    match EvalRunner::new_with_provider(provider) {
                        Ok(r) => r,
                        Err(e) => {
                            pb.set_style(
                                ProgressStyle::default_spinner()
                                    .template("  {msg}")
                                    .unwrap(),
                            );
                            pb.finish_with_message(format!(
                                "{} {:<20} error: {}",
                                color::x_mark(),
                                name,
                                e
                            ));
                            return (name, Err(e), None);
                        }
                    }
                };

                let result = scenario.run(&runner).await;

                // Update progress bar with result
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("  {msg}")
                        .unwrap(),
                );

                match &result {
                    Ok(report) => {
                        let passed = report.metrics.iter().filter(|m| m.result.passed()).count();
                        let total = report.metrics.len();
                        let duration_secs = report.duration_ms as f64 / 1000.0;

                        let status = if report.passed {
                            format!(
                                "{} {:<20} {} ({}/{} metrics) [{:.1}s]",
                                color::check_mark(),
                                name,
                                color::green("passed"),
                                passed,
                                total,
                                duration_secs
                            )
                        } else {
                            format!(
                                "{} {:<20} {} ({}/{} metrics) [{:.1}s]",
                                color::x_mark(),
                                name,
                                color::red("failed"),
                                passed,
                                total,
                                duration_secs
                            )
                        };
                        pb.finish_with_message(status);
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "{} {:<20} {}: {}",
                            color::x_mark(),
                            name,
                            color::red("error"),
                            e
                        ));
                    }
                }

                (name, result, log_path_for_result)
            }
        })
        .collect();

    // Run all scenarios concurrently
    let results = join_all(futures).await;

    // Finish header
    header.finish_and_clear();

    // Collect results
    let mut summary = EvalSummary::default();
    let mut reports: Vec<(String, EvalReport, Option<PathBuf>)> = Vec::new();
    let mut errors: Vec<(String, anyhow::Error)> = Vec::new();
    let threshold = metric_pass_threshold(provider);

    for (name, result, log_path) in results {
        match result {
            Ok(mut report) => {
                // Apply metric pass threshold (Z.AI uses 80%, others 100%)
                report.apply_pass_threshold(threshold);
                summary.add(report.clone());
                reports.push((name, report, log_path));
            }
            Err(e) => errors.push((name, e)),
        }
    }

    // Print a blank line after progress bars
    println!();

    // Print verbose log locations if any
    if verbose && !reports.is_empty() {
        println!("Verbose logs:");
        reports.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, _, log_path) in &reports {
            if let Some(path) = log_path {
                if path.exists() {
                    println!("  {} → {}", name, path.display());
                }
            }
        }
        println!();
    }

    for (name, e) in errors {
        eprintln!("Error running scenario {}: {}", name, e);
    }

    Ok(summary)
}

/// Simple parallel execution without progress bars (for JSON output or quiet mode).
async fn run_parallel_simple(
    scenarios: Vec<Box<dyn Scenario>>,
    log_dir: Option<Arc<PathBuf>>,
    _verbose: bool,
    provider: EvalProvider,
    quiet: bool,
) -> Result<EvalSummary> {
    let futures: Vec<_> = scenarios
        .into_iter()
        .map(|scenario| {
            let name = scenario.name().to_string();
            let log_file = log_dir
                .as_ref()
                .map(|dir| dir.join(format!("{}.log", name)));

            async move {
                let runner = if let Some(path) = log_file {
                    match EvalRunner::new_with_log_file_and_provider(path, provider) {
                        Ok(r) => r,
                        Err(e) => return (name, Err(e)),
                    }
                } else {
                    match EvalRunner::new_with_provider(provider) {
                        Ok(r) => r,
                        Err(e) => return (name, Err(e)),
                    }
                };
                let result = scenario.run(&runner).await;
                (name, result)
            }
        })
        .collect();

    let results = join_all(futures).await;

    let mut summary = EvalSummary::default();
    let threshold = metric_pass_threshold(provider);

    for (name, result) in results {
        match result {
            Ok(mut report) => {
                // Apply metric pass threshold (Z.AI uses 80%, others 100%)
                report.apply_pass_threshold(threshold);
                if !quiet {
                    println!("{}", serde_json::to_string(&report.to_json())?);
                }
                summary.add(report);
            }
            Err(e) => {
                eprintln!("Error running scenario {}: {}", name, e);
            }
        }
    }

    Ok(summary)
}

/// List available benchmarks.
pub fn list_benchmark_options() {
    println!("Available benchmarks:\n");
    for (name, description, count) in qbit_benchmarks::list_benchmarks() {
        println!("  {} - {} ({} problems)", name, description, count);
    }

    // Add SWE-bench info
    let (name, desc, count) = qbit_swebench::benchmark_info();
    println!("  {} - {} ({} instances)", name, desc, count);

    println!();
    println!("Run with:");
    println!("  --benchmark humaneval              # HumanEval benchmark");
    println!("  --swebench                         # SWE-bench Lite benchmark");
    println!();
    println!("Filter examples:");
    println!("  --benchmark humaneval --problems 0-9");
    println!("  --swebench --instance django__django-11133");
    println!("  --swebench --problems 0-9");
    println!();
}

/// Run a benchmark suite.
///
/// # Arguments
/// * `benchmark` - Name of the benchmark to run (e.g., "humaneval")
/// * `problems` - Optional problem filter (e.g., "0-10" or "0,5,10")
/// * `json_output` - Whether to output JSON
/// * `verbose` - Whether to show verbose output
/// * `parallel` - Whether to run scenarios in parallel
/// * `concurrency` - Maximum number of concurrent scenarios when parallel
/// * `provider` - LLM provider to use
/// * `model` - Optional model override
/// * `output_options` - Optional output configuration
pub async fn run_benchmark(
    benchmark: &str,
    problems: Option<&str>,
    json_output: bool,
    verbose: bool,
    parallel: bool,
    concurrency: usize,
    provider: EvalProvider,
    model: Option<&str>,
    output_options: Option<EvalOutputOptions>,
) -> Result<()> {
    // Initialize tracing for evals - always use error level to suppress noise
    // We handle our own verbose output display
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("qbit=error".parse().unwrap())
                .add_directive("qbit_evals=error".parse().unwrap())
                .add_directive("qbit_ai=error".parse().unwrap())
                .add_directive("qbit_benchmarks=error".parse().unwrap()),
        )
        .try_init();

    let scenarios = qbit_benchmarks::get_benchmark_scenarios(benchmark, problems)?;

    if scenarios.is_empty() {
        anyhow::bail!(
            "No problems found for benchmark '{}' with filter '{}'",
            benchmark,
            problems.unwrap_or("none")
        );
    }

    if !json_output {
        println!(
            "Running {} benchmark ({} problems)",
            benchmark,
            scenarios.len()
        );
        println!("Provider: {}\n", provider);
    }

    // Determine if we should suppress normal output
    let use_new_output = output_options.is_some();
    let opts = output_options.unwrap_or(EvalOutputOptions {
        json: json_output,
        pretty: false,
        output_file: None,
        transcript: false,
    });

    let suppress_intermediate = use_new_output || opts.transcript;

    let summary = if parallel && scenarios.len() > 1 {
        run_parallel_benchmark(
            scenarios,
            opts.json,
            verbose,
            provider,
            model,
            suppress_intermediate,
            concurrency,
        )
        .await?
    } else {
        run_sequential_benchmark(scenarios, opts.json, verbose, provider, model, suppress_intermediate)
            .await?
    };

    // Handle output based on options
    if let Some(ref output_path) = opts.output_file {
        let file = std::fs::File::create(output_path)?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &summary.to_json())?;
        eprintln!("Results saved to: {}", output_path.display());
    }

    if opts.pretty {
        summary.print_ci_summary(&mut std::io::stdout(), &provider.to_string())?;
    } else if opts.json {
        println!("{}", serde_json::to_string(&summary.to_json())?);
    } else if !use_new_output {
        summary.print_summary(&mut std::io::stdout())?;
    }

    // Print final pass rate
    let pass_rate = summary.pass_rate();
    println!();
    if pass_rate < 1.0 {
        println!("{}", color::red_line());
        println!(
            "{}",
            color::red(&format!(
                "  {}: {}/{} passed ({:.1}%)",
                benchmark.to_uppercase(),
                summary.passed_count(),
                summary.reports.len(),
                pass_rate * 100.0
            ))
        );
        println!("{}", color::red_line());
    } else {
        println!("{}", color::green_line());
        println!(
            "{}",
            color::green(&format!(
                "  {}: All {} problems passed (100%)",
                benchmark.to_uppercase(),
                summary.reports.len()
            ))
        );
        println!("{}", color::green_line());
    }

    Ok(())
}

/// Run benchmark scenarios sequentially.
async fn run_sequential_benchmark(
    scenarios: Vec<Box<dyn qbit_evals::scenarios::Scenario>>,
    json_output: bool,
    verbose: bool,
    provider: EvalProvider,
    model: Option<&str>,
    quiet: bool,
) -> Result<EvalSummary> {
    // Enable verbose to show tool calls and reasoning in real-time
    let runner = EvalRunner::new_verbose_with_provider(verbose, provider)?
        .with_model(model.map(|s| s.to_string()));
    let mut summary = EvalSummary::default();

    for scenario in scenarios {
        if !json_output && !quiet {
            println!("\n{}", color::cyan(&format!("=== {} ===", scenario.name())));
            if verbose {
                println!("\n{}:", color::yellow("Prompt"));
                println!("{}", scenario.prompt());
                println!();
            }
        }

        match scenario.run(&runner).await {
            Ok(report) => {
                if json_output && !quiet {
                    println!("{}", serde_json::to_string(&report.to_json())?);
                } else if !quiet {
                    // Show agent response in verbose mode
                    if verbose {
                        println!("{}:", color::yellow("Response"));
                        println!("{}", report.agent_output.response);
                        println!();

                        // Show tool calls
                        if !report.agent_output.tool_calls.is_empty() {
                            println!("{}:", color::yellow("Tool Calls"));
                            for tc in &report.agent_output.tool_calls {
                                let status = if tc.success { "✓" } else { "✗" };
                                println!("  {} {}", status, tc.name);
                            }
                            println!();
                        }
                    }

                    let status = if report.passed {
                        color::green("PASS")
                    } else {
                        color::red("FAIL")
                    };
                    println!("Result: {} ({}ms)", status, report.duration_ms);

                    // Show failure details
                    if !report.passed {
                        for metric in &report.metrics {
                            if !metric.result.passed() {
                                if let qbit_evals::MetricResult::Fail { reason } = &metric.result {
                                    println!("  {} failed: {}", metric.name, reason);
                                }
                            }
                        }
                    }
                }
                summary.add(report);
            }
            Err(e) => {
                eprintln!("Error running {}: {:#}", scenario.name(), e);
            }
        }
    }

    Ok(summary)
}

/// Run benchmark scenarios in parallel with concurrency limiting.
async fn run_parallel_benchmark(
    scenarios: Vec<Box<dyn qbit_evals::scenarios::Scenario>>,
    json_output: bool,
    verbose: bool,
    provider: EvalProvider,
    model: Option<&str>,
    quiet: bool,
    concurrency: usize,
) -> Result<EvalSummary> {
    use qbit_evals::indicatif::{MultiProgress, ProgressBar, ProgressStyle};
    use std::time::Duration;

    let model_owned = model.map(|s| s.to_string());
    let semaphore = Arc::new(Semaphore::new(concurrency));

    // For JSON output or quiet mode, use simple execution
    if json_output || quiet {
        let futures: Vec<_> = scenarios
            .into_iter()
            .map(|scenario| {
                let name = scenario.name().to_string();
                let model_clone = model_owned.clone();
                let sem = semaphore.clone();
                async move {
                    // Acquire semaphore permit to limit concurrency
                    let _permit = sem.acquire().await.unwrap();
                    let runner = match EvalRunner::new_with_provider(provider) {
                        Ok(r) => r.with_model(model_clone),
                        Err(e) => return (name, Err(e)),
                    };
                    let result = scenario.run(&runner).await;
                    (name, result)
                }
            })
            .collect();

        let results = join_all(futures).await;
        let mut summary = EvalSummary::default();

        for (name, result) in results {
            match result {
                Ok(report) => {
                    if !quiet {
                        println!("{}", serde_json::to_string(&report.to_json())?);
                    }
                    summary.add(report);
                }
                Err(e) => {
                    eprintln!("Error running {}: {}", name, e);
                }
            }
        }

        return Ok(summary);
    }

    // Progress bar display
    let multi_progress = MultiProgress::new();
    let header = multi_progress.add(ProgressBar::new_spinner());
    header.set_style(ProgressStyle::default_spinner().template("{msg}").unwrap());
    header.set_message(format!(
        "Running {} problems in parallel (max {} concurrent)",
        scenarios.len(),
        concurrency
    ));
    header.tick();

    let spinner_style = ProgressStyle::default_spinner()
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
        .template("  {spinner:.cyan} {wide_msg}")
        .unwrap();

    // Create progress bars showing "queued" initially
    let progress_bars: Vec<_> = scenarios
        .iter()
        .map(|scenario| {
            let pb = multi_progress.add(ProgressBar::new_spinner());
            pb.set_style(spinner_style.clone());
            pb.set_message(format!("{:<20} queued", scenario.name()));
            pb.enable_steady_tick(Duration::from_millis(100));
            pb
        })
        .collect();

    let futures: Vec<_> = scenarios
        .into_iter()
        .zip(progress_bars.into_iter())
        .map(|(scenario, pb)| {
            let name = scenario.name().to_string();
            let model_clone = model_owned.clone();
            let sem = semaphore.clone();
            async move {
                // Acquire semaphore permit to limit concurrency
                let _permit = sem.acquire().await.unwrap();
                pb.set_message(format!("{:<20} running...", name));

                let runner = match EvalRunner::new_verbose_with_provider(verbose, provider) {
                    Ok(r) => r.with_model(model_clone),
                    Err(e) => {
                        pb.set_style(
                            ProgressStyle::default_spinner()
                                .template("  {msg}")
                                .unwrap(),
                        );
                        pb.finish_with_message(format!(
                            "{} {:<20} error: {}",
                            color::x_mark(),
                            name,
                            e
                        ));
                        return (name, Err(e));
                    }
                };

                let result = scenario.run(&runner).await;

                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("  {msg}")
                        .unwrap(),
                );

                match &result {
                    Ok(report) => {
                        let status = if report.passed {
                            format!(
                                "{} {:<20} {}",
                                color::check_mark(),
                                name,
                                color::green("passed")
                            )
                        } else {
                            format!(
                                "{} {:<20} {}",
                                color::x_mark(),
                                name,
                                color::red("failed")
                            )
                        };
                        pb.finish_with_message(status);
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "{} {:<20} {}: {}",
                            color::x_mark(),
                            name,
                            color::red("error"),
                            e
                        ));
                    }
                }

                (name, result)
            }
        })
        .collect();

    let results = join_all(futures).await;
    header.finish_and_clear();

    let mut summary = EvalSummary::default();
    for (_name, result) in results {
        if let Ok(report) = result {
            summary.add(report);
        }
    }

    println!();
    Ok(summary)
}

/// Run OpenAI model connectivity tests.
///
/// Tests each OpenAI model (or a specific one) with a simple hello world
/// prompt to verify configuration and connectivity.
pub async fn run_openai_model_tests(
    model_filter: Option<&str>,
    json_output: bool,
    verbose: bool,
    parallel: bool,
) -> Result<()> {
    // Initialize tracing for evals
    let log_level = if verbose { "debug" } else { "warn" };
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("qbit={}", log_level).parse().unwrap())
                .add_directive(format!("qbit_evals={}", log_level).parse().unwrap())
                .add_directive(format!("qbit_ai={}", log_level).parse().unwrap()),
        )
        .try_init();

    let scenarios = if let Some(model_id) = model_filter {
        match get_openai_model_scenario(model_id) {
            Some(s) => vec![s],
            None => {
                eprintln!("Unknown OpenAI model: {}", model_id);
                eprintln!("Available models:");
                for (id, name) in list_openai_models() {
                    eprintln!("  {} - {}", id, name);
                }
                anyhow::bail!("Unknown OpenAI model: {}", model_id);
            }
        }
    } else {
        openai_model_scenarios()
    };

    if !json_output {
        println!(
            "Testing OpenAI model connectivity ({} models)",
            scenarios.len()
        );
        println!("Provider: openai\n");
    }

    // OpenAI model tests always use OpenAI provider
    let provider = EvalProvider::OpenAi;

    let summary = if parallel && scenarios.len() > 1 {
        run_parallel(scenarios, json_output, verbose, provider, false).await?
    } else {
        run_sequential(scenarios, json_output, verbose, provider, false).await?
    };

    if json_output {
        println!("{}", serde_json::to_string(&summary.to_json())?);
    } else {
        summary.print_summary(&mut std::io::stdout())?;
    }

    // Print final PASS/FAIL result for GitHub Actions
    println!();
    if summary.failed_count() > 0 {
        println!("{}", color::red_line());
        println!(
            "{}",
            color::red(&format!(
                "  FAIL: {} of {} models failed connectivity test",
                summary.failed_count(),
                summary.reports.len()
            ))
        );
        println!("{}", color::red_line());
        anyhow::bail!(
            "{} of {} models failed connectivity test",
            summary.failed_count(),
            summary.reports.len()
        );
    } else {
        println!("{}", color::green_line());
        println!(
            "{}",
            color::green(&format!(
                "  PASS: All {} models passed connectivity test",
                summary.reports.len()
            ))
        );
        println!("{}", color::green_line());
    }

    Ok(())
}

/// Helper to save an individual eval report to the results directory.
fn save_instance_result(results_dir: &std::path::Path, report: &EvalReport) -> Result<()> {
    let filename = format!("{}.json", report.scenario.replace('/', "_").replace('\\', "_"));
    let path = results_dir.join(&filename);

    let detailed_json = report.to_detailed_json();
    let file = std::fs::File::create(&path)?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &detailed_json)?;

    Ok(())
}

/// Check which instances already have results in the directory.
fn get_completed_instances(results_dir: &std::path::Path) -> std::collections::HashSet<String> {
    let mut completed = std::collections::HashSet::new();

    if let Ok(entries) = std::fs::read_dir(results_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(stem) = path.file_stem() {
                    // The filename is the scenario name (with slashes replaced by underscores)
                    // For SWE-bench, the scenario name is the instance ID
                    let name = stem.to_string_lossy().to_string();
                    completed.insert(name);
                }
            }
        }
    }

    completed
}

/// Run SWE-bench scenarios with incremental result saving.
async fn run_swebench_sequential_with_saving(
    scenarios: Vec<Box<dyn qbit_evals::scenarios::Scenario>>,
    verbose: bool,
    provider: EvalProvider,
    model: Option<&str>,
    results_dir: &std::path::Path,
    resume: bool,
) -> Result<EvalSummary> {
    let runner = EvalRunner::new_verbose_with_provider(verbose, provider)?
        .with_model(model.map(|s| s.to_string()));
    let mut summary = EvalSummary::default();

    // Get list of already completed instances if resuming
    let completed = if resume {
        get_completed_instances(results_dir)
    } else {
        std::collections::HashSet::new()
    };

    let total = scenarios.len();
    let mut skipped = 0;

    for (idx, scenario) in scenarios.into_iter().enumerate() {
        let name = scenario.name().to_string();

        // Skip if already completed (when resuming)
        if completed.contains(&name) {
            skipped += 1;
            eprintln!(
                "[{}/{}] Skipping {} (already completed)",
                idx + 1,
                total,
                name
            );
            continue;
        }

        eprintln!(
            "\n[{}/{}] Running {}...",
            idx + 1,
            total,
            name
        );

        match scenario.run(&runner).await {
            Ok(report) => {
                // Save result immediately
                if let Err(e) = save_instance_result(results_dir, &report) {
                    eprintln!("  Warning: Failed to save result for {}: {}", name, e);
                } else {
                    eprintln!("  Saved result to {}.json", name);
                }

                // Show result status
                let status = if report.passed {
                    color::green("SOLVED")
                } else {
                    color::red("FAILED")
                };
                eprintln!("  Result: {} ({}ms)", status, report.duration_ms);

                summary.add(report);
            }
            Err(e) => {
                eprintln!("  Error: {:#}", e);
                // Save error result
                let error_json = serde_json::json!({
                    "scenario": name,
                    "error": format!("{:#}", e),
                    "passed": false,
                });
                let filename = format!("{}.error.json", name);
                let path = results_dir.join(&filename);
                if let Ok(file) = std::fs::File::create(&path) {
                    let _ = serde_json::to_writer_pretty(file, &error_json);
                }
            }
        }
    }

    if skipped > 0 {
        eprintln!("\nSkipped {} already-completed instances", skipped);
    }

    Ok(summary)
}

/// Run SWE-bench Lite benchmark.
///
/// # Arguments
/// * `filter` - Optional instance filter (e.g., "django__django-11133" or "0-10")
/// * `json_output` - Whether to output JSON
/// * `verbose` - Whether to show verbose output
/// * `parallel` - Whether to run scenarios in parallel
/// * `concurrency` - Maximum number of concurrent scenarios when parallel
/// * `provider` - LLM provider to use
/// * `model` - Optional model override
/// * `output_options` - Optional output configuration
/// * `workspace_dir` - Optional persistent workspace directory (for debugging)
/// * `test_only` - Skip agent, only run Docker tests (requires workspace_dir)
/// * `results_dir` - Optional directory to save per-instance detailed JSON results
pub async fn run_swebench(
    filter: Option<&str>,
    json_output: bool,
    verbose: bool,
    parallel: bool,
    concurrency: usize,
    provider: EvalProvider,
    model: Option<&str>,
    output_options: Option<EvalOutputOptions>,
    workspace_dir: Option<PathBuf>,
    test_only: bool,
    results_dir: Option<PathBuf>,
) -> Result<()> {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("qbit=error".parse().unwrap())
                .add_directive("qbit_evals=error".parse().unwrap())
                .add_directive("qbit_ai=error".parse().unwrap())
                .add_directive("qbit_swebench=error".parse().unwrap()),
        )
        .try_init();

    // Check Docker availability
    if !qbit_swebench::check_docker().await? {
        anyhow::bail!(
            "Docker is not available. Please ensure Docker is installed and running.\n\
             SWE-bench requires Docker for test execution."
        );
    }

    // Handle test-only mode (skip agent, run Docker tests on existing workspace)
    if test_only {
        let workspace = workspace_dir.ok_or_else(|| {
            anyhow::anyhow!("--test-only requires --workspace-dir to specify the workspace location")
        })?;

        let instance_id = filter.ok_or_else(|| {
            anyhow::anyhow!("--test-only requires --instance to specify which instance to test")
        })?;

        println!("Running tests only (skipping agent)");
        println!("  Instance: {}", instance_id);
        println!("  Workspace: {}\n", workspace.display());

        let result = qbit_swebench::run_tests_only(instance_id, &workspace).await?;

        // Print final result
        if result.is_solved() {
            println!("{}", color::green_line());
            println!("{}", color::green("  SWE-BENCH: Instance SOLVED"));
            println!("{}", color::green_line());
        } else {
            println!("{}", color::red_line());
            println!("{}", color::red("  SWE-BENCH: Instance FAILED"));
            println!("{}", color::red_line());
        }

        return Ok(());
    }

    let scenarios = qbit_swebench::get_benchmark_scenarios(filter).await?;

    if scenarios.is_empty() {
        anyhow::bail!(
            "No instances found for filter '{}'",
            filter.unwrap_or("none")
        );
    }

    // Create results directory (use provided or create timestamped default)
    let results_dir = results_dir.unwrap_or_else(|| {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".qbit")
            .join("swebench-results")
            .join(timestamp.to_string())
    });
    std::fs::create_dir_all(&results_dir)?;

    // Check for existing results (for resume capability)
    let completed = get_completed_instances(&results_dir);
    let resume = !completed.is_empty();

    if !json_output {
        let (name, desc, _) = qbit_swebench::benchmark_info();
        println!("Running {} benchmark ({} instances)", name, scenarios.len());
        println!("{}", desc);
        println!("Provider: {}", provider);
        println!("Results: {}", results_dir.display());
        if resume {
            println!("Resuming: {} instances already completed", completed.len());
        }
        println!();
    }

    // Determine if we should suppress normal output
    let use_new_output = output_options.is_some();
    let opts = output_options.unwrap_or(EvalOutputOptions {
        json: json_output,
        pretty: false,
        output_file: None,
        transcript: false,
    });

    // Use the new incremental saving function for SWE-bench (sequential only for now)
    // This saves results after each instance completes, so progress isn't lost on interruption
    let summary = if parallel && scenarios.len() > 1 && !resume {
        // Parallel mode doesn't support resume yet - fall back to old behavior
        // TODO: Add parallel support with incremental saving
        eprintln!("Warning: Parallel mode doesn't save results incrementally. Consider using sequential mode with --no-parallel for long runs.");
        let suppress_intermediate = use_new_output || opts.transcript;
        run_parallel_benchmark(
            scenarios,
            opts.json,
            verbose,
            provider,
            model,
            suppress_intermediate,
            concurrency,
        )
        .await?
    } else {
        // Sequential mode with incremental saving
        run_swebench_sequential_with_saving(
            scenarios,
            verbose,
            provider,
            model,
            &results_dir,
            resume,
        )
        .await?
    };

    // Summary file is always written at the end
    let summary_path = results_dir.join("summary.json");
    let summary_file = std::fs::File::create(&summary_path)?;
    serde_json::to_writer_pretty(summary_file, &summary.to_json())?;
    eprintln!("Summary saved to: {}", summary_path.display());

    // Handle output based on options
    if let Some(ref output_path) = opts.output_file {
        let file = std::fs::File::create(output_path)?;
        let mut writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, &summary.to_json())?;
        eprintln!("Results saved to: {}", output_path.display());
    }

    if opts.pretty {
        summary.print_ci_summary(&mut std::io::stdout(), &provider.to_string())?;
    } else if opts.json {
        println!("{}", serde_json::to_string(&summary.to_json())?);
    } else if !use_new_output {
        summary.print_summary(&mut std::io::stdout())?;
    }

    // Print final pass rate
    let pass_rate = summary.pass_rate();
    println!();
    if pass_rate < 1.0 {
        println!("{}", color::red_line());
        println!(
            "{}",
            color::red(&format!(
                "  SWE-BENCH: {}/{} solved ({:.1}%)",
                summary.passed_count(),
                summary.reports.len(),
                pass_rate * 100.0
            ))
        );
        println!("{}", color::red_line());
    } else {
        println!("{}", color::green_line());
        println!(
            "{}",
            color::green(&format!(
                "  SWE-BENCH: All {} instances solved (100%)",
                summary.reports.len()
            ))
        );
        println!("{}", color::green_line());
    }

    Ok(())
}
