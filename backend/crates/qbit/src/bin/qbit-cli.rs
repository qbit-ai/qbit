//! Qbit CLI - Headless interface for the Qbit AI agent
//!
//! This binary provides a command-line interface to the Qbit agent,
//! enabling automated testing, scripting, and headless operation.
//!
//! # Usage
//!
//! ```bash
//! # Build the CLI binary
//! cargo build --package qbit --features cli --no-default-features --bin qbit-cli
//!
//! # Execute a single prompt
//! ./target/debug/qbit-cli -e "What files are in this directory?"
//!
//! # With auto-approval for testing
//! ./target/debug/qbit-cli -e "Read Cargo.toml" --auto-approve
//!
//! # JSON output for scripting
//! ./target/debug/qbit-cli -e "Hello" --json --auto-approve | jq .
//!
//! # Quiet mode - only final response
//! ./target/debug/qbit-cli -e "What is 2+2?" --quiet --auto-approve
//!
//! # Interactive REPL mode (when no -e or -f provided)
//! ./target/debug/qbit-cli --auto-approve
//! ```
//!
//! # Features
//!
//! This binary requires the `cli` feature flag and is mutually exclusive
//! with the `tauri` feature (GUI application).

use anyhow::Result;
use clap::Parser;

use qbit_lib::cli::{execute_batch, execute_once, initialize, run_repl, Args};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Eval mode - run evaluation scenarios
    #[cfg(feature = "evals")]
    if args.list_scenarios {
        qbit_lib::cli::eval::list_scenarios();
        return Ok(());
    }

    #[cfg(feature = "evals")]
    if args.list_benchmarks {
        qbit_lib::cli::eval::list_benchmark_options();
        return Ok(());
    }

    #[cfg(feature = "evals")]
    if let Some(ref benchmark) = args.benchmark {
        use qbit_evals::EvalProvider;
        use qbit_lib::cli::eval::EvalOutputOptions;
        use std::str::FromStr;

        // Parse provider from args (defaults to Vertex Claude)
        let provider = if let Some(ref provider_str) = args.eval_provider {
            EvalProvider::from_str(provider_str)?
        } else {
            EvalProvider::default()
        };

        // Build output options if any new flags are specified
        let output_options = if args.output.is_some() || args.pretty || args.transcript {
            Some(EvalOutputOptions {
                json: args.json,
                pretty: args.pretty,
                output_file: args.output.clone(),
                transcript: args.transcript,
            })
        } else {
            None
        };

        return qbit_lib::cli::eval::run_benchmark(
            benchmark,
            args.problems.as_deref(),
            args.json,
            args.verbose,
            args.parallel,
            provider,
            args.eval_model.as_deref(),
            output_options,
        )
        .await;
    }

    #[cfg(feature = "evals")]
    if args.eval {
        use qbit_evals::EvalProvider;
        use qbit_lib::cli::eval::EvalOutputOptions;
        use std::str::FromStr;

        // Parse provider from args (defaults to Vertex Claude)
        let provider = if let Some(ref provider_str) = args.eval_provider {
            EvalProvider::from_str(provider_str)?
        } else {
            EvalProvider::default()
        };

        // Build output options if any new flags are specified
        let output_options = if args.output.is_some() || args.pretty || args.transcript {
            Some(EvalOutputOptions {
                json: args.json,
                pretty: args.pretty,
                output_file: args.output.clone(),
                transcript: args.transcript,
            })
        } else {
            None
        };

        return qbit_lib::cli::eval::run_evals(
            args.scenario.as_deref(),
            args.json,
            args.verbose,
            args.parallel,
            provider,
            output_options,
        )
        .await;
    }

    // OpenAI model connectivity tests
    #[cfg(feature = "evals")]
    if args.openai_models {
        return qbit_lib::cli::eval::run_openai_model_tests(
            args.openai_model.as_deref(),
            args.json,
            args.verbose,
            args.parallel,
        )
        .await;
    }

    // SWE-bench Lite benchmark
    #[cfg(feature = "evals")]
    if args.swebench {
        use qbit_evals::EvalProvider;
        use qbit_lib::cli::eval::EvalOutputOptions;
        use std::str::FromStr;

        // Parse provider from args (defaults to Vertex Claude)
        let provider = if let Some(ref provider_str) = args.eval_provider {
            EvalProvider::from_str(provider_str)?
        } else {
            EvalProvider::default()
        };

        // Build output options if any new flags are specified
        let output_options = if args.output.is_some() || args.pretty || args.transcript {
            Some(EvalOutputOptions {
                json: args.json,
                pretty: args.pretty,
                output_file: args.output.clone(),
                transcript: args.transcript,
            })
        } else {
            None
        };

        // Use instance filter or problems filter
        let filter = args.instance.as_deref().or(args.problems.as_deref());

        return qbit_lib::cli::eval::run_swebench(
            filter,
            args.json,
            args.verbose,
            args.parallel,
            provider,
            args.eval_model.as_deref(),
            output_options,
        )
        .await;
    }

    // Initialize the full Qbit stack
    let mut ctx = initialize(&args).await?;

    // Execute based on mode
    let result = if let Some(ref prompt) = args.execute {
        // Single prompt execution mode
        execute_once(&mut ctx, prompt).await
    } else if let Some(ref file) = args.file {
        // Batch file execution mode
        execute_batch(&mut ctx, file).await
    } else {
        // No prompt provided - enter interactive REPL mode
        run_repl(&mut ctx).await
    };

    // Graceful shutdown
    ctx.shutdown().await?;

    result
}
