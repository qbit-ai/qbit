//! CLI evaluation runner.
//!
//! Provides the entry point for running evals from the command line.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use qbit_evals::indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use qbit_evals::outcome::{EvalReport, EvalSummary};
use qbit_evals::runner::EvalRunner;
use qbit_evals::scenarios::{all_scenarios, get_scenario, Scenario};
use qbit_evals::EvalProvider;
use tracing_subscriber::EnvFilter;

/// List all available scenarios.
pub fn list_scenarios() {
    println!("Available evaluation scenarios:\n");
    for scenario in all_scenarios() {
        println!("  {} - {}", scenario.name(), scenario.description());
    }
    println!();
}

/// Run evaluation scenarios.
pub async fn run_evals(
    scenario_filter: Option<&str>,
    json_output: bool,
    verbose: bool,
    parallel: bool,
    provider: EvalProvider,
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
        all_scenarios()
    };

    if !json_output {
        println!("Using LLM provider: {}", provider);
    }

    let summary = if parallel && scenarios.len() > 1 {
        run_parallel(scenarios, json_output, verbose, provider).await?
    } else {
        run_sequential(scenarios, json_output, verbose, provider).await?
    };

    if json_output {
        println!("{}", serde_json::to_string(&summary.to_json())?);
    } else {
        summary.print_summary(&mut std::io::stdout())?;
    }

    if summary.failed_count() > 0 {
        anyhow::bail!(
            "{} of {} scenarios failed",
            summary.failed_count(),
            summary.reports.len()
        );
    }

    Ok(())
}

/// Run scenarios sequentially.
async fn run_sequential(
    scenarios: Vec<Box<dyn Scenario>>,
    json_output: bool,
    verbose: bool,
    provider: EvalProvider,
) -> Result<EvalSummary> {
    let runner = EvalRunner::new_verbose_with_provider(verbose, provider)?;
    let mut summary = EvalSummary::default();

    for scenario in scenarios {
        if !json_output {
            println!("Running scenario: {}", scenario.name());
        }

        match scenario.run(&runner).await {
            Ok(report) => {
                if json_output {
                    println!("{}", serde_json::to_string(&report.to_json())?);
                } else {
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
) -> Result<EvalSummary> {
    // Create log directory for verbose output if needed
    let log_dir = if verbose {
        let dir = std::env::temp_dir().join("qbit-eval-logs");
        std::fs::create_dir_all(&dir)?;
        Some(Arc::new(dir))
    } else {
        None
    };

    // For JSON output, use simple execution without progress bars
    if json_output {
        return run_parallel_simple(scenarios, log_dir, verbose, provider).await;
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
                                "\x1b[31m✗\x1b[0m {:<20} error: {}",
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
                                "\x1b[31m✗\x1b[0m {:<20} error: {}",
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
                                "\x1b[32m✓\x1b[0m {:<20} \x1b[32mpassed\x1b[0m ({}/{} metrics) [{:.1}s]",
                                name, passed, total, duration_secs
                            )
                        } else {
                            format!(
                                "\x1b[31m✗\x1b[0m {:<20} \x1b[31mfailed\x1b[0m ({}/{} metrics) [{:.1}s]",
                                name, passed, total, duration_secs
                            )
                        };
                        pb.finish_with_message(status);
                    }
                    Err(e) => {
                        pb.finish_with_message(format!(
                            "\x1b[31m✗\x1b[0m {:<20} \x1b[31merror\x1b[0m: {}",
                            name,
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

    for (name, result, log_path) in results {
        match result {
            Ok(report) => {
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

/// Simple parallel execution without progress bars (for JSON output).
async fn run_parallel_simple(
    scenarios: Vec<Box<dyn Scenario>>,
    log_dir: Option<Arc<PathBuf>>,
    _verbose: bool,
    provider: EvalProvider,
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

    for (name, result) in results {
        match result {
            Ok(report) => {
                println!("{}", serde_json::to_string(&report.to_json())?);
                summary.add(report);
            }
            Err(e) => {
                eprintln!("Error running scenario {}: {}", name, e);
            }
        }
    }

    Ok(summary)
}
