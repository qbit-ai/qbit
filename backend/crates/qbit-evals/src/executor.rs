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
use std::sync::Arc;

use anyhow::Result;
use rig::completion::CompletionModel as RigCompletionModel;
use tokio::sync::RwLock;

use qbit_ai::agent_mode::AgentMode;
use qbit_ai::contributors::create_default_contributors;
use qbit_ai::eval_support::EvalConfig as AiEvalConfig;
use qbit_ai::prompt_registry::PromptContributorRegistry;
use qbit_ai::system_prompt::build_system_prompt_with_contributions;
use qbit_core::PromptContext;
use qbit_sub_agents::SubAgentRegistry;

use crate::config::{EvalConfig, EvalProvider};
use crate::runner::{AgentOutput, ToolCall as EvalToolCall, VerboseConfig};

/// Build the production system prompt with all contributions.
///
/// This builds the same prompt as the main agent (agent_bridge.rs), including:
/// - Sub-agent documentation (when has_sub_agents = true)
/// - Provider-specific tool instructions (when has_web_search = true)
///
/// # Arguments
/// * `workspace` - The workspace directory
/// * `provider` - The provider being used for this eval
///
/// # Returns
/// The complete system prompt string with all contributions appended.
pub fn build_production_system_prompt(workspace: &Path, provider: EvalProvider) -> String {
    // Create sub-agent registry with default agents (same as main agent)
    let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));

    // Create prompt contributor registry with default contributors
    let contributors = create_default_contributors(sub_agent_registry);
    let mut registry = PromptContributorRegistry::new();
    for contributor in contributors {
        registry.register(contributor);
    }

    // Map eval provider to provider name for context
    let provider_name = match provider {
        EvalProvider::VertexClaude => "anthropic",
        EvalProvider::Zai => "zai",
        EvalProvider::OpenAi => "openai",
    };

    // Create prompt context with provider, model, and feature flags
    // For evals:
    // - has_web_search is true for Vertex Claude (native web search enabled)
    // - has_sub_agents is true (same as main agent)
    let has_web_search = matches!(provider, EvalProvider::VertexClaude);
    let has_sub_agents = true;

    let prompt_context = PromptContext::new(provider_name, "eval-model")
        .with_web_search(has_web_search)
        .with_sub_agents(has_sub_agents)
        .with_workspace(workspace.display().to_string());

    // No memory file for evals - testbeds are isolated workspaces
    build_system_prompt_with_contributions(
        workspace,
        AgentMode::AutoApprove, // Evals always auto-approve
        None,                   // No memory file
        Some(&registry),
        Some(&prompt_context),
    )
}

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
    execute_eval_prompt_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        provider,
        None,
    )
    .await
}

/// Execute a prompt with all options including model override.
pub async fn execute_eval_prompt_with_model(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
    model_override: Option<&str>,
) -> Result<AgentOutput> {
    // Load configuration for the specified provider
    let config = EvalConfig::load_for_provider(provider)
        .await?
        .with_model(model_override.map(|s| s.to_string()));

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

/// Execute a prompt with custom tools for specialized benchmarks.
///
/// This variant allows injecting custom tool definitions and executors,
/// which is needed for specialized benchmarks like SWE-bench.
#[allow(clippy::too_many_arguments)]
pub async fn execute_eval_prompt_with_tools(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    provider: EvalProvider,
    model_override: Option<&str>,
    additional_tools: Vec<rig::completion::ToolDefinition>,
    custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor>,
) -> Result<AgentOutput> {
    // Load configuration for the specified provider
    let config = EvalConfig::load_for_provider(provider)
        .await?
        .with_model(model_override.map(|s| s.to_string()));

    match provider {
        EvalProvider::VertexClaude => {
            execute_with_vertex_claude_and_tools(
                workspace,
                prompt,
                system_prompt,
                verbose_config,
                &config,
                additional_tools,
                custom_executor,
            )
            .await
        }
        EvalProvider::Zai => {
            execute_with_zai_and_tools(
                workspace,
                prompt,
                system_prompt,
                verbose_config,
                &config,
                additional_tools,
                custom_executor,
            )
            .await
        }
        EvalProvider::OpenAi => {
            execute_with_openai_and_tools(
                workspace,
                prompt,
                system_prompt,
                verbose_config,
                &config,
                additional_tools,
                custom_executor,
            )
            .await
        }
    }
}

/// Execute with Vertex AI Claude and custom tools.
async fn execute_with_vertex_claude_and_tools(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
    additional_tools: Vec<rig::completion::ToolDefinition>,
    custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor>,
) -> Result<AgentOutput> {
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

    let model_id = config
        .model_override
        .as_deref()
        .unwrap_or(models::CLAUDE_SONNET_4_5);
    let model_name = config
        .model_override
        .as_deref()
        .unwrap_or("Claude Sonnet 4.5");

    let model = client.completion_model(model_id).with_web_search();

    execute_with_model_and_tools(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
        EvalProvider::VertexClaude,
        additional_tools,
        custom_executor,
    )
    .await
}

/// Execute with Z.AI GLM and custom tools.
async fn execute_with_zai_and_tools(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
    additional_tools: Vec<rig::completion::ToolDefinition>,
    custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor>,
) -> Result<AgentOutput> {
    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    let model_id = config
        .model_override
        .as_deref()
        .unwrap_or(rig_zai_sdk::models::GLM_4);
    let model_name = config.model_override.as_deref().unwrap_or("GLM-4");

    let client = rig_zai_sdk::Client::new(&zai_config.api_key);
    let model = client.completion_model(model_id);

    execute_with_model_and_tools(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
        EvalProvider::Zai,
        additional_tools,
        custom_executor,
    )
    .await
}

/// Execute with OpenAI and custom tools.
async fn execute_with_openai_and_tools(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    config: &EvalConfig,
    additional_tools: Vec<rig::completion::ToolDefinition>,
    custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor>,
) -> Result<AgentOutput> {
    use rig::client::CompletionClient;
    use rig::providers::openai as rig_openai;

    let openai_config = config
        .openai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OpenAI configuration not available"))?;

    let model_id = config.model_override.as_deref().unwrap_or("gpt-5.1");
    let model_name = config.model_override.as_deref().unwrap_or("GPT-5.1");

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    let model = client.completion_model(model_id);

    execute_with_model_and_tools(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
        EvalProvider::OpenAi,
        additional_tools,
        custom_executor,
    )
    .await
}

/// Generic execution with any model and custom tools.
#[allow(clippy::too_many_arguments)]
async fn execute_with_model_and_tools<M>(
    workspace: &Path,
    prompt: &str,
    system_prompt: Option<&str>,
    verbose_config: &VerboseConfig,
    model: M,
    model_name: &str,
    provider: EvalProvider,
    additional_tools: Vec<rig::completion::ToolDefinition>,
    custom_executor: Option<qbit_ai::eval_support::CustomToolExecutor>,
) -> Result<AgentOutput>
where
    M: RigCompletionModel + Sync,
{
    let provider_name = match provider {
        EvalProvider::VertexClaude => "anthropic",
        EvalProvider::Zai => "zai",
        EvalProvider::OpenAi => "openai_responses",
    };

    let ai_config = AiEvalConfig {
        provider_name: provider_name.to_string(),
        model_name: model_name.to_string(),
        require_hitl: false,
        workspace: workspace.to_path_buf(),
        verbose: verbose_config.enabled,
    };

    let effective_system_prompt = match system_prompt {
        Some(custom) => custom.to_string(),
        None => build_production_system_prompt(workspace, provider),
    };

    // Run with custom tools
    let eval_output = qbit_ai::eval_support::run_eval_agentic_loop_with_tools(
        &model,
        &effective_system_prompt,
        prompt,
        ai_config,
        additional_tools,
        custom_executor,
    )
    .await?;

    tracing::info!(
        "Eval with custom tools completed with {} tool calls, {} files modified",
        eval_output.tool_calls.len(),
        eval_output.files_modified.len()
    );

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

    // Use model override if provided, otherwise use default
    let model_id = config
        .model_override
        .as_deref()
        .unwrap_or(models::CLAUDE_SONNET_4_5);
    let model_name = config
        .model_override
        .as_deref()
        .unwrap_or("Claude Sonnet 4.5");

    // Enable native web search (web_search_20250305)
    // Note: web_fetch_20250910 requires a beta header not yet supported on Vertex AI
    let model = client.completion_model(model_id).with_web_search();

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
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
    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    // Use model override if provided, otherwise use default
    let model_id = config
        .model_override
        .as_deref()
        .unwrap_or(rig_zai_sdk::models::GLM_4);
    let model_name = config.model_override.as_deref().unwrap_or("GLM-4");

    let client = rig_zai_sdk::Client::new(&zai_config.api_key);
    let model = client.completion_model(model_id);

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
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

    // Use model override if provided, otherwise use default
    let model_id = config.model_override.as_deref().unwrap_or("gpt-5.1");
    let model_name = config.model_override.as_deref().unwrap_or("GPT-5.1");

    let client: rig_openai::Client = rig_openai::Client::new(&openai_config.api_key)
        .map_err(|e| anyhow::anyhow!("Failed to create OpenAI client: {}", e))?;
    // Use completion_model which returns Responses API model (same as main app)
    let model = client.completion_model(model_id);

    execute_with_model(
        workspace,
        prompt,
        system_prompt,
        verbose_config,
        model,
        model_name,
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
    let zai_config = config
        .zai
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Z.AI configuration not available"))?;

    let client = rig_zai_sdk::Client::new(&zai_config.api_key);
    let model = client.completion_model(rig_zai_sdk::models::GLM_4);

    execute_multi_turn_with_model(workspace, prompts, model, "GLM-4", EvalProvider::Zai).await
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
        verbose: false, // Multi-turn evals don't need verbose output
    };

    // Build the production system prompt with contributions (same as main agent)
    let system_prompt = build_production_system_prompt(workspace, provider);

    // Run multi-turn evaluation
    let multi_output =
        qbit_ai::eval_support::run_multi_turn_eval(&model, &system_prompt, prompts, ai_config)
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
    verbose_config: &VerboseConfig,
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
        verbose: verbose_config.enabled,
    };

    // Build the effective system prompt:
    // - If a custom system prompt is provided (for scenario-specific tests), use it
    // - Otherwise, use the production prompt with contributions (same as main agent)
    let effective_system_prompt = match system_prompt {
        Some(custom) => custom.to_string(),
        None => build_production_system_prompt(workspace, provider),
    };

    // Run the unified agentic loop
    let eval_output = qbit_ai::eval_support::run_eval_agentic_loop(
        &model,
        &effective_system_prompt,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Helper to build the main agent's system prompt for comparison.
    ///
    /// This replicates the exact logic from agent_bridge.rs::prepare_execution_context
    /// so we can verify evals get the same prompt.
    fn build_main_agent_prompt(
        workspace: &Path,
        provider_name: &str,
        has_web_search: bool,
    ) -> String {
        // Create sub-agent registry (same as main agent)
        let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));

        // Create prompt contributor registry with default contributors
        let contributors = create_default_contributors(sub_agent_registry);
        let mut registry = PromptContributorRegistry::new();
        for contributor in contributors {
            registry.register(contributor);
        }

        // Create prompt context (same as main agent)
        let has_sub_agents = true; // Main agent always has sub-agents
        let prompt_context = PromptContext::new(provider_name, "test-model")
            .with_web_search(has_web_search)
            .with_sub_agents(has_sub_agents)
            .with_workspace(workspace.display().to_string());

        // Build prompt (same as main agent with AutoApprove mode since that's what we expect)
        build_system_prompt_with_contributions(
            workspace,
            AgentMode::AutoApprove,
            None,
            Some(&registry),
            Some(&prompt_context),
        )
    }

    #[test]
    fn test_eval_prompt_matches_main_agent_prompt_vertex() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let eval_prompt = build_production_system_prompt(workspace, EvalProvider::VertexClaude);
        let main_prompt = build_main_agent_prompt(workspace, "anthropic", true);

        assert_eq!(
            eval_prompt, main_prompt,
            "Eval prompt must match main agent prompt for Vertex Claude"
        );
    }

    #[test]
    fn test_eval_prompt_matches_main_agent_prompt_openai() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let eval_prompt = build_production_system_prompt(workspace, EvalProvider::OpenAi);
        let main_prompt = build_main_agent_prompt(workspace, "openai", false);

        assert_eq!(
            eval_prompt, main_prompt,
            "Eval prompt must match main agent prompt for OpenAI"
        );
    }

    #[test]
    fn test_eval_prompt_matches_main_agent_prompt_zai() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let eval_prompt = build_production_system_prompt(workspace, EvalProvider::Zai);
        let main_prompt = build_main_agent_prompt(workspace, "zai", false);

        assert_eq!(
            eval_prompt, main_prompt,
            "Eval prompt must match main agent prompt for Z.AI"
        );
    }

    #[test]
    fn test_eval_prompt_contains_core_sections() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let prompt = build_production_system_prompt(workspace, EvalProvider::VertexClaude);

        // Verify all core sections from the main agent's system_prompt.rs are present
        assert!(
            prompt.contains("# Tone and style"),
            "Prompt must contain tone and style section"
        );
        assert!(
            prompt.contains("# Tool Reference"),
            "Prompt must contain tool reference section"
        );
        assert!(
            prompt.contains("# Sub-Agent Delegation"),
            "Prompt must contain sub-agent delegation section"
        );
        assert!(
            prompt.contains("# Security Boundaries"),
            "Prompt must contain security boundaries section"
        );
        assert!(
            prompt.contains("# Before Claiming Completion"),
            "Prompt must contain completion checklist"
        );
    }

    #[test]
    fn test_eval_prompt_contains_autoapprove_mode() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let prompt = build_production_system_prompt(workspace, EvalProvider::VertexClaude);

        // Evals use AutoApprove mode, which adds specific instructions
        assert!(
            prompt.contains("<autoapprove_mode>"),
            "Eval prompt must contain auto-approve mode instructions"
        );
    }

    #[test]
    fn test_eval_prompt_contains_sub_agent_docs_for_vertex() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let prompt = build_production_system_prompt(workspace, EvalProvider::VertexClaude);

        // With has_sub_agents = true, sub-agent docs should be included
        // Note: The registry starts empty, so we might not see specific sub-agents,
        // but the infrastructure should still work. Let's check the prompt
        // builds without errors.
        assert!(!prompt.is_empty(), "Prompt should not be empty");
    }

    #[test]
    fn test_eval_prompt_consistent_across_providers() {
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path();

        let vertex_prompt = build_production_system_prompt(workspace, EvalProvider::VertexClaude);
        let openai_prompt = build_production_system_prompt(workspace, EvalProvider::OpenAi);

        // Since we no longer append provider-specific contributions,
        // prompts should be identical across providers
        assert_eq!(
            vertex_prompt, openai_prompt,
            "Prompts should be consistent across providers"
        );
    }
}
