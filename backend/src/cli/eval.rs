//! CLI evaluation runner.
//!
//! Provides the entry point for running evals from the command line.

use anyhow::Result;

use crate::evals::outcome::EvalSummary;
use crate::evals::runner::EvalRunner;
use crate::evals::scenarios::{all_scenarios, get_scenario};

/// List all available scenarios.
pub fn list_scenarios() {
    println!("Available evaluation scenarios:\n");
    for scenario in all_scenarios() {
        println!("  {} - {}", scenario.name(), scenario.description());
    }
    println!();
}

/// Run evaluation scenarios.
pub async fn run_evals(scenario_filter: Option<&str>, json_output: bool) -> Result<()> {
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

    let runner = EvalRunner::new()?;
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
                if !json_output {
                    // Continue to next scenario on error
                }
            }
        }
    }

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
