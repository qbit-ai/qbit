//! CLI execution runner.
//!
//! Handles prompt execution with the agent bridge.

use std::path::Path;

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::runtime::CliRuntime;
use qbit_core::runtime::RuntimeEvent;

use super::bootstrap::CliContext;
use crate::cli_output::{run_event_loop, truncate};

/// Execute a single prompt and wait for completion.
///
/// This spawns the event loop in a background task and calls
/// the agent bridge to process the prompt.
pub async fn execute_once(ctx: &mut CliContext, prompt: &str) -> Result<()> {
    // Create a fresh channel for this execution
    let (event_tx, event_rx) = mpsc::unbounded_channel::<RuntimeEvent>();

    // Update the runtime's sender so events flow to our new receiver
    // We need to downcast to CliRuntime to access replace_event_tx
    if let Some(cli_runtime) = ctx.runtime.as_any().downcast_ref::<CliRuntime>() {
        cli_runtime.replace_event_tx(event_tx);
    } else {
        // Fallback for non-CLI runtimes (shouldn't happen in CLI mode)
        tracing::warn!("Runtime is not CliRuntime, events may not be received");
    }

    // Spawn the event loop handler
    let json_mode = ctx.args.json;
    let quiet_mode = ctx.args.quiet;

    let output_handle: JoinHandle<Result<()>> =
        tokio::spawn(async move { run_event_loop(event_rx, json_mode, quiet_mode).await });

    // Execute the prompt via the agent bridge
    let result = {
        let bridge_guard = ctx.bridge().await;
        let bridge = bridge_guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Agent not initialized"))?;

        bridge.execute(prompt).await
    };

    // Wait for the output handler to finish
    // It will exit when it sees Completed or Error events
    match output_handle.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            tracing::warn!("Output handler error: {}", e);
        }
        Err(e) => {
            tracing::warn!("Output handler panicked: {}", e);
        }
    }

    // Persist prompt history (best-effort)
    if let Some(history) = &ctx.history {
        let provider = ctx
            .bridge()
            .await
            .as_ref()
            .map(|b| b.provider_name().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let model = ctx
            .bridge()
            .await
            .as_ref()
            .map(|b| b.model_name().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let success = result.is_ok();
        let _ = history.add_prompt(
            "cli".to_string(),
            prompt.to_string(),
            model,
            provider,
            0,
            0,
            success,
        );
    }

    // Return the execution result
    result.map(|_| ())
}

/// Execute prompts from a file, one per line.
///
/// Each non-empty, non-comment line is executed sequentially.
/// Lines starting with `#` are treated as comments.
/// Execution stops on first error unless continue_on_error is set.
pub async fn execute_batch(ctx: &mut CliContext, file_path: &Path) -> Result<()> {
    let content = tokio::fs::read_to_string(file_path)
        .await
        .with_context(|| format!("Failed to read prompt file: {}", file_path.display()))?;

    let prompts: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    if prompts.is_empty() {
        anyhow::bail!("No prompts found in file: {}", file_path.display());
    }

    let total = prompts.len();
    if !ctx.args.quiet {
        eprintln!(
            "[batch] Executing {} prompt(s) from {}",
            total,
            file_path.display()
        );
    }

    for (i, prompt) in prompts.iter().enumerate() {
        if !ctx.args.quiet {
            eprintln!(
                "\n[batch] [{}/{}] Executing: {}",
                i + 1,
                total,
                truncate(prompt, 50)
            );
        }

        // execute_once handles creating fresh event channels internally
        execute_once(ctx, prompt).await?;

        if !ctx.args.quiet {
            eprintln!("[batch] [{}/{}] Complete", i + 1, total);
        }
    }

    if !ctx.args.quiet {
        eprintln!("\n[batch] All {} prompt(s) completed successfully", total);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    // Tests would require mocking the agent bridge
    // For now, we rely on integration tests
}
