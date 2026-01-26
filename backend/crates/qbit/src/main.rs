// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Qbit - AI-powered terminal emulator
//!
//! This is the unified entry point for both GUI and CLI modes:
//! - `qbit` or `qbit [path]` - Launches the Tauri GUI application
//! - `qbit --headless [options]` - Runs in headless CLI mode
//! - `qbit -e "prompt"` - Executes a single prompt (implies --headless)
//!
//! # Examples
//!
//! ```bash
//! # Launch GUI (default)
//! qbit
//!
//! # Launch GUI in a specific directory
//! qbit ~/Code/my-project
//!
//! # Headless mode: interactive REPL
//! qbit --headless
//!
//! # Headless mode: execute a single prompt
//! qbit -e "What files are in this directory?"
//!
//! # Headless mode: with auto-approval for testing
//! qbit -e "Read Cargo.toml" --auto-approve
//! ```

use clap::Parser;

use qbit_lib::cli::Args;

fn main() {
    // Parse CLI arguments to determine mode
    let args = Args::parse();

    // Determine if we should run in headless mode:
    // - Explicit --headless flag
    // - Or -e (execute) or -f (file) flags imply headless
    let is_headless = args.headless || args.execute.is_some() || args.file.is_some();

    // Check for eval-related flags that also imply headless
    #[cfg(feature = "evals")]
    let is_headless = is_headless
        || args.eval
        || args.list_scenarios
        || args.list_benchmarks
        || args.benchmark.is_some()
        || args.openai_models
        || args.swebench;

    if is_headless {
        // Run in headless CLI mode
        run_cli(args);
    } else {
        // Run in GUI mode
        // Pass workspace path to GUI if provided and not the default "."
        if args.workspace.to_string_lossy() != "." {
            std::env::set_var("QBIT_WORKSPACE", &args.workspace);
        }
        qbit_lib::run_gui();
    }
}

/// Run in headless CLI mode
fn run_cli(args: Args) {
    // Build a new tokio runtime for CLI mode
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    runtime.block_on(async move {
        if let Err(e) = run_cli_async(args).await {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    });
}

async fn run_cli_async(args: Args) -> anyhow::Result<()> {
    use qbit_lib::cli::{execute_batch, execute_once, initialize, run_repl};

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
            args.concurrency,
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
            args.concurrency,
            provider,
            args.eval_model.as_deref(),
            output_options,
            args.workspace_dir.clone(),
            args.test_only,
            args.results_dir.clone(),
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
