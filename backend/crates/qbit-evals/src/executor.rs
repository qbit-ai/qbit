//! Agent executor for evaluations using the unified agentic loop.
//!
//! This executor runs the same agentic loop as the main application, ensuring
//! evaluations test actual production behavior. It sets up minimal mock
//! dependencies and auto-approves all tool calls.
//!
//! Supports multiple LLM providers:
//! - Vertex AI Claude Sonnet (default)
//! - Z.AI GLM-4.7
//! - OpenAI GPT-5.1

use std::path::Path;

use anyhow::Result;
use rig::completion::CompletionModel as RigCompletionModel;

use qbit_ai::eval_support::EvalConfig as AiEvalConfig;

use crate::config::{EvalConfig, EvalProvider};
use crate::runner::{AgentOutput, ToolCall as EvalToolCall, VerboseConfig};

/// Eval-specific system prompt - minimal and focused
const EVAL_SYSTEM_PROMPT: &str = r#"You are an AI coding assistant being evaluated on your ability to complete software engineering tasks.

You have access to the following tools:
- read_file: Read a file's contents
- write_file: Write or overwrite a file
- create_file: Create a new file (fails if exists)
- edit_file: Edit an existing file with search/replace
- delete_file: Delete a file
- list_files: List files matching a pattern
- list_directory: List directory contents
- grep_file: Search for patterns in files
- run_pty_cmd: Run a shell command

Complete the task efficiently. When done, provide a brief summary of what you accomplished.
Do not ask for clarification - make reasonable assumptions and proceed.
"#;

/// Execute a prompt against the agent in the given workspace using the default provider.
///
/// This is a lightweight executor that:
/// - Uses the configured LLM provider (default: Vertex Claude Sonnet)
/// - Has a minimal set of tools
/// - Runs an agentic loop until completion
/// - Auto-approves all tool calls (no HITL)
///
/// If `verbose_config.enabled` is true, outputs real-time conversation.
/// If `verbose_config.log_file` is set, writes to that file instead of stdout.
pub async fn execute_eval_prompt(
    workspace: &Path,
    prompt: &str,
    verbose_config: &VerboseConfig,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(
        workspace,
        prompt,
        None,
        verbose_config,
        EvalProvider::default(),
    )
    .await
}

/// Execute a prompt with a custom system prompt.
///
/// This variant allows testing how different system prompts affect agent behavior.
/// If `system_prompt` is `None`, uses the default eval system prompt.
pub async fn execute_eval_prompt_with_system(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        EvalProvider::default(),
    )
    .await
}

/// Execute a prompt against the agent using a specific provider.
pub async fn execute_eval_prompt_with_provider(
    workspace: &Path,
    prompt: &str,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
) -> Result<AgentOutput> {
    execute_eval_prompt_with_options(workspace, prompt, None, verbose_config, provider).await
}

/// Execute a prompt with all options: custom system prompt and provider.
pub async fn execute_eval_prompt_with_options(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
) -> Result<AgentOutput> {
    // Load configuration for the specified provider
    let config = EvalConfig::load_for_provider(provider).await?;

    match provider {
        EvalProvider::VertexClaude => {
            execute_with_vertex_claude(workspace, prompt, system_prompt, verbose_config, &config)
                .await
        }
        EvalProvider::Zai => {
            execute_with_zai(workspace, prompt, system_prompt, verbose_config, &config).await
        }
        EvalProvider::OpenAi => {
            execute_with_openai(workspace, prompt, system_prompt, verbose_config, &config).await
        }
    }
}

/// Execute with Vertex AI Claude.
async fn execute_with_vertex_claude(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig_anthropic_vertex::{models, Client};

    let vertex_config = config
        .vertex
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vertex AI configuration not available"))?;

    // Create client using service account credentials if available, otherwise fall back to ADC
    let client = if let Some(ref creds_path) = vertex_config.credentials_path {
        Client::from_service_account(
            creds_path,
            &vertex_config.project_id,
            &vertex_config.location,
        )
        .await?
    } else {
        Client::from_env(&vertex_config.project_id, &vertex_config.location).await?
    };
    // Enable native web search (web_search_20250305)
    // Note: web_fetch_20250910 requires a beta header not yet supported on Vertex AI
    let model = client
        .completion_model(models::CLAUDE_SONNET_4_5)
        .with_web_search();

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "Claude Sonnet 4.5",
        EvalProvider::VertexClaude,
    )
    .await
}

/// Execute with Z.AI GLM.
async fn execute_with_zai(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig::client::CompletionClient;

    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    let client = rig_zai::Client::new(&zai_config.api_key);
    let model = client.completion_model(rig_zai::GLM_4_7);

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "GLM-4.7",
        EvalProvider::Zai,
    )
    .await
}

/// Execute with OpenAI.
async fn execute_with_openai(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<AgentOutput> {
    use rig::client::CompletionClient;
    use rig::providers::openai as rig_openai;

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    // Use completion_model which returns Responses API model (same as main app)
    let model = client.completion_model("gpt-5.1");

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        "GPT-5.1",
        EvalProvider::OpenAi,
    )
    .await
}

/// Output from a multi-turn evaluation.
#[derive(Debug)]
pub struct MultiTurnAgentOutput {
    /// Outputs from each turn in order.
    pub turns: Vec<AgentOutput>,
    /// Total duration of all turns in milliseconds.
    pub total_duration_ms: u64,
}

/// Execute a multi-turn conversation to test reasoning ID preservation.
///
/// This is critical for testing OpenAI Responses API compatibility,
/// as reasoning item errors only manifest across multiple turns.
pub async fn execute_multi_turn_eval(
    workspace: &Path,
    prompts: &[&str],
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
) -> Result<MultiTurnAgentOutput> {
    let config = EvalConfig::load_for_provider(provider).await?;

    match provider {
        EvalProvider::VertexClaude => {
            execute_multi_turn_with_vertex_claude(workspace, prompts, verbose_config, &config).await
        }
        EvalProvider::Zai => {
            execute_multi_turn_with_zai(workspace, prompts, verbose_config, &config).await
        }
        EvalProvider::OpenAi => {
            execute_multi_turn_with_openai(workspace, prompts, verbose_config, &config).await
        }
    }
}

/// Execute multi-turn with Vertex AI Claude.
async fn execute_multi_turn_with_vertex_claude(
    workspace: &Path,
    prompts: &[&str],
    _verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<MultiTurnAgentOutput> {
    use rig_anthropic_vertex::{models, Client};

    let vertex_config = config
        .vertex
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Vertex AI configuration not available"))?;

    let client = if let Some(ref creds_path) = vertex_config.credentials_path {
        Client::from_service_account(
            creds_path,
            &vertex_config.project_id,
            &vertex_config.location,
        )
        .await?
    } else {
        Client::from_env(&vertex_config.project_id, &vertex_config.location).await?
    };
    let model = client
        .completion_model(models::CLAUDE_SONNET_4_5)
        .with_web_search();

    execute_multi_turn_with_model(
        workspace,
        prompts,
        model,
        "Claude Sonnet 4.5",
        EvalProvider::VertexClaude,
    )
    .await
}

/// Execute multi-turn with Z.AI GLM.
async fn execute_multi_turn_with_zai(
    workspace: &Path,
    prompts: &[&str],
    _verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<MultiTurnAgentOutput> {
    use rig::client::CompletionClient;

    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    let client = rig_zai::Client::new(&zai_config.api_key);
    let model = client.completion_model(rig_zai::GLM_4_7);

    execute_multi_turn_with_model(workspace, prompts, model, "GLM-4.7", EvalProvider::Zai).await
}

/// Execute multi-turn with OpenAI.
async fn execute_multi_turn_with_openai(
    workspace: &Path,
    prompts: &[&str],
    _verbose_config: &VerboseConfig,
    config: &EvalConfig,
) -> Result<MultiTurnAgentOutput> {
    use rig::client::CompletionClient;
    use rig::providers::openai as rig_openai;

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    let model = client.completion_model("gpt-5.1");

    execute_multi_turn_with_model(workspace, prompts, model, "GPT-5.1", EvalProvider::OpenAi).await
}

/// Generic multi-turn execution with any model.
async fn execute_multi_turn_with_model<M>(
    workspace: &Path,
    prompts: &[&str],
    model: M,
    model_name: &str,
    provider: EvalProvider,
) -> Result<MultiTurnAgentOutput>
where
    M: RigCompletionModel + Sync,
{
    let provider_name = match provider {
        EvalProvider::VertexClaude => "anthropic",
        EvalProvider::Zai => "zai",
        // Use openai_responses because evals use the Responses API (completion_model returns ResponsesCompletionModel)
        EvalProvider::OpenAi => "openai_responses",
    };

    let ai_config = AiEvalConfig {
        provider_name: provider_name.to_string(),
        model_name: model_name.to_string(),
        require_hitl: false,
        workspace: workspace.to_path_buf(),
    };

    // Run multi-turn evaluation
    let multi_output =
        qbit_ai::eval_support::run_multi_turn_eval(&model, EVAL_SYSTEM_PROMPT, prompts, ai_config)
            .await?;

    tracing::info!(
        "Multi-turn eval completed: {} turns in {}ms",
        multi_output.turns.len(),
        multi_output.total_duration_ms
    );

    // Convert outputs
    let turns = multi_output
        .turns
        .into_iter()
        .map(|turn| {
            let tool_calls = turn
                .tool_calls
                .into_iter()
                .map(|tc| EvalToolCall {
                    name: tc.name,
                    input: tc.input,
                    output: tc.output,
                    success: tc.success,
                })
                .collect();

            AgentOutput {
                response: turn.response,
                tool_calls,
                files_modified: turn.files_modified,
                duration_ms: turn.duration_ms,
                tokens_used: turn.tokens_used,
            }
        })
        .collect();

    Ok(MultiTurnAgentOutput {
        turns,
        total_duration_ms: multi_output.total_duration_ms,
    })
}

/// Generic execution with any model implementing CompletionModel.
///
/// This function now delegates to the unified agentic loop from qbit-ai,
/// ensuring evals test the same code path as the main application.
async fn execute_with_model<M>(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    _verbose_config: &VerboseConfig,
    model: M,
    model_name: &str,
    provider: EvalProvider,
) -> Result<AgentOutput>
where
    M: RigCompletionModel + Sync,
{
    // Map eval provider to provider name for capabilities detection
    let provider_name = match provider {
        EvalProvider::VertexClaude => "anthropic",
        EvalProvider::Zai => "zai",
        // Use openai_responses because evals use the Responses API (completion_model returns ResponsesCompletionModel)
        EvalProvider::OpenAi => "openai_responses",
    };

    // Create eval config for the unified loop
    let ai_config = AiEvalConfig {
        provider_name: provider_name.to_string(),
        model_name: model_name.to_string(),
        require_hitl: false,
        workspace: workspace.to_path_buf(),
    };

    // Run the unified agentic loop
    let eval_output = qbit_ai::eval_support::run_eval_agentic_loop(
        &model,
        system_prompt.unwrap_or(EVAL_SYSTEM_PROMPT),
        prompt,
        ai_config,
    )
    .await?;

    tracing::info!(
        "Eval completed with {} tool calls, {} files modified",
        eval_output.tool_calls.len(),
        eval_output.files_modified.len()
    );

    // Convert from qbit_ai's EvalToolCall to qbit_evals' ToolCall
    let tool_calls = eval_output
        .tool_calls
        .into_iter()
        .map(|tc| EvalToolCall {
            name: tc.name,
            input: tc.input,
            output: tc.output,
            success: tc.success,
        })
        .collect();

    Ok(AgentOutput {
        response: eval_output.response,
        tool_calls,
        files_modified: eval_output.files_modified,
        duration_ms: eval_output.duration_ms,
        tokens_used: eval_output.tokens_used,
    })
}
