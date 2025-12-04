//! Workflow execution commands for Tauri.
//!
//! These commands provide the interface for starting and running
//! graph-flow based multi-agent workflows.

use std::collections::HashMap;
use std::sync::Arc;

use graph_flow::{InMemorySessionStorage, SessionStorage};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;

use crate::ai::events::AiEvent;
use crate::ai::workflow::{
    create_git_commit_workflow, git_commit, GitCommitResult, GitCommitState, WorkflowLlmExecutor,
    WorkflowRegistry, WorkflowRunner, WorkflowStatus,
};
use crate::state::AppState;

use super::AI_NOT_INITIALIZED_ERROR;

/// State for workflow management.
pub struct WorkflowState {
    /// Registry of workflow graphs
    pub registry: RwLock<WorkflowRegistry>,
    /// Session storage for workflows
    pub storage: Arc<dyn SessionStorage + Send + Sync>,
    /// Active workflow runners keyed by session_id
    pub runners: RwLock<HashMap<String, Arc<WorkflowRunner>>>,
}

impl Default for WorkflowState {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowState {
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(WorkflowRegistry::new()),
            storage: Arc::new(InMemorySessionStorage::new()),
            runners: RwLock::new(HashMap::new()),
        }
    }
}

/// Adapter that implements WorkflowLlmExecutor by delegating to the AgentBridge.
pub struct BridgeLlmExecutor {
    event_tx: tokio::sync::mpsc::UnboundedSender<AiEvent>,
}

impl BridgeLlmExecutor {
    pub fn new(event_tx: tokio::sync::mpsc::UnboundedSender<AiEvent>) -> Self {
        Self { event_tx }
    }
}

#[async_trait::async_trait]
impl WorkflowLlmExecutor for BridgeLlmExecutor {
    async fn complete(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        _context: HashMap<String, serde_json::Value>,
    ) -> anyhow::Result<String> {
        // For now, emit an event and return a placeholder
        // In a full implementation, this would call the actual LLM
        let _ = self.event_tx.send(AiEvent::WorkflowStepStarted {
            workflow_id: "workflow".to_string(),
            step_name: "llm_completion".to_string(),
            step_index: 0,
            total_steps: 1,
        });

        // TODO: Integrate with actual LLM client
        // For now, return a structured response that the tasks can parse
        tracing::debug!(
            "WorkflowLlmExecutor called with system_prompt: {:.100}..., user_prompt: {:.100}...",
            system_prompt,
            user_prompt
        );

        // This is a placeholder - the real implementation would call the LLM
        Err(anyhow::anyhow!(
            "LLM integration not yet implemented for workflows. \
             This requires wiring up the rig-anthropic-vertex client to the workflow executor."
        ))
    }
}

/// Response from starting a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowResponse {
    pub session_id: String,
    pub workflow_name: String,
}

/// Response from a workflow step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepResponse {
    pub output: Option<String>,
    pub status: String,
    pub next_task_id: Option<String>,
}

/// Input for starting a git commit workflow.
#[derive(Debug, Clone, Deserialize)]
pub struct GitCommitWorkflowInput {
    pub git_status: String,
    pub git_diff: String,
}

/// Start a git commit workflow.
///
/// This creates a new workflow session and returns the session ID.
/// Use `step_workflow` or `run_workflow_to_completion` to execute it.
#[tauri::command]
pub async fn start_git_commit_workflow(
    state: State<'_, AppState>,
    input: GitCommitWorkflowInput,
) -> Result<StartWorkflowResponse, String> {
    // Check that AI is initialized (we need the event channel)
    let bridge_guard = state.ai_state.bridge.read().await;
    let bridge = bridge_guard
        .as_ref()
        .ok_or(AI_NOT_INITIALIZED_ERROR)?;

    // Create the LLM executor using the bridge's event channel
    let executor: Arc<dyn WorkflowLlmExecutor> =
        Arc::new(BridgeLlmExecutor::new(bridge.event_tx.clone()));

    // Create the workflow graph
    let graph = create_git_commit_workflow(executor);

    // Create a runner
    let workflow_state = &state.workflow_state;
    let runner = WorkflowRunner::new(graph.clone(), workflow_state.storage.clone());

    // Start the session
    let session_id = runner
        .start_session("", "initialize")
        .await
        .map_err(|e| e.to_string())?;

    // Set initial input in session context
    if let Ok(Some(session)) = workflow_state.storage.get(&session_id).await {
        session
            .context
            .set("git_status_input", input.git_status)
            .await;
        session.context.set("git_diff_input", input.git_diff).await;
        workflow_state
            .storage
            .save(session)
            .await
            .map_err(|e| format!("Failed to save session: {}", e))?;
    }

    // Store the runner
    workflow_state
        .runners
        .write()
        .await
        .insert(session_id.clone(), Arc::new(runner));

    // Emit workflow started event
    let _ = bridge.event_tx.send(AiEvent::WorkflowStarted {
        workflow_id: session_id.clone(),
        workflow_name: "git_commit".to_string(),
        session_id: session_id.clone(),
    });

    Ok(StartWorkflowResponse {
        session_id,
        workflow_name: "git_commit".to_string(),
    })
}

/// Execute the next step in a workflow.
#[tauri::command]
pub async fn step_workflow(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<WorkflowStepResponse, String> {
    let workflow_state = &state.workflow_state;
    let runners = workflow_state.runners.read().await;

    let runner = runners
        .get(&session_id)
        .ok_or_else(|| format!("No workflow found with session_id: {}", session_id))?;

    let result = runner.step(&session_id).await.map_err(|e| e.to_string())?;

    let (status, next_task_id) = match &result.status {
        WorkflowStatus::Paused { next_task_id } => {
            ("paused".to_string(), Some(next_task_id.clone()))
        }
        WorkflowStatus::WaitingForInput => ("waiting_for_input".to_string(), None),
        WorkflowStatus::Completed => ("completed".to_string(), None),
        WorkflowStatus::Error(e) => (format!("error: {}", e), None),
    };

    Ok(WorkflowStepResponse {
        output: result.output,
        status,
        next_task_id,
    })
}

/// Run a workflow to completion.
#[tauri::command]
pub async fn run_workflow_to_completion(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<String, String> {
    let workflow_state = &state.workflow_state;
    let runners = workflow_state.runners.read().await;

    let runner = runners
        .get(&session_id)
        .ok_or_else(|| format!("No workflow found with session_id: {}", session_id))?;

    let result = runner
        .run_to_completion(&session_id)
        .await
        .map_err(|e| e.to_string())?;

    // Emit workflow completed event
    if let Ok(bridge_guard) = state.ai_state.bridge.try_read() {
        if let Some(bridge) = bridge_guard.as_ref() {
            let _ = bridge.event_tx.send(AiEvent::WorkflowCompleted {
                workflow_id: session_id.clone(),
                final_output: result.clone(),
                total_duration_ms: 0, // TODO: track duration
            });
        }
    }

    // Cleanup the runner
    drop(runners);
    workflow_state.runners.write().await.remove(&session_id);

    Ok(result)
}

/// Get the current state of a git commit workflow.
#[tauri::command]
pub async fn get_git_commit_workflow_state(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<GitCommitState, String> {
    let workflow_state = &state.workflow_state;

    let session = workflow_state
        .storage
        .get(&session_id)
        .await
        .map_err(|e| format!("Failed to get session: {}", e))?
        .ok_or_else(|| format!("No session found with id: {}", session_id))?;

    let state: GitCommitState = session
        .context
        .get(git_commit::STATE_KEY)
        .await
        .unwrap_or_default();

    Ok(state)
}

/// Get the result of a completed git commit workflow.
#[tauri::command]
pub async fn get_git_commit_workflow_result(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<GitCommitResult, String> {
    let workflow_state = &state.workflow_state;

    let session = workflow_state
        .storage
        .get(&session_id)
        .await
        .map_err(|e| format!("Failed to get session: {}", e))?
        .ok_or_else(|| format!("No session found with id: {}", session_id))?;

    let state: GitCommitState = session
        .context
        .get(git_commit::STATE_KEY)
        .await
        .unwrap_or_default();

    Ok(GitCommitResult::from(state))
}

/// List active workflow sessions.
#[tauri::command]
pub async fn list_workflow_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let workflow_state = &state.workflow_state;
    let runners = workflow_state.runners.read().await;
    Ok(runners.keys().cloned().collect())
}

/// Cancel a workflow session.
#[tauri::command]
pub async fn cancel_workflow(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let workflow_state = &state.workflow_state;

    // Emit workflow error/cancelled event
    if let Ok(bridge_guard) = state.ai_state.bridge.try_read() {
        if let Some(bridge) = bridge_guard.as_ref() {
            let _ = bridge.event_tx.send(AiEvent::WorkflowError {
                workflow_id: session_id.clone(),
                step_name: None,
                error: "Workflow cancelled by user".to_string(),
            });
        }
    }

    // Remove the runner
    workflow_state.runners.write().await.remove(&session_id);

    Ok(())
}
