use serde::{Deserialize, Serialize};

use super::hitl::{ApprovalPattern, RiskLevel};

/// Source of a tool call - indicates where the tool request originated.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolSource {
    /// Tool called by the main agent
    #[default]
    Main,
    /// Tool called by a sub-agent
    SubAgent {
        agent_id: String,
        agent_name: String,
    },
    /// Tool called by a workflow
    Workflow {
        workflow_id: String,
        workflow_name: String,
        /// Current step name (if within a step)
        #[serde(skip_serializing_if = "Option::is_none")]
        step_name: Option<String>,
        /// Current step index (0-based)
        #[serde(skip_serializing_if = "Option::is_none")]
        step_index: Option<usize>,
    },
}

/// Simplified AI events for the frontend.
/// We emit these directly from AgentBridge instead of converting from vtcode's ThreadEvent,
/// since ThreadEvent uses tuple structs that are harder to work with.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiEvent {
    /// Agent started processing a turn
    Started { turn_id: String },

    /// Streaming text chunk from the LLM
    TextDelta { delta: String, accumulated: String },

    /// Tool execution requested (for approval UI / HITL)
    /// This is the legacy event - kept for backward compatibility
    ToolRequest {
        tool_name: String,
        args: serde_json::Value,
        request_id: String,
        /// Source of this tool call (main agent, sub-agent, or workflow)
        #[serde(default)]
        source: ToolSource,
    },

    /// Tool approval request with HITL metadata
    /// The frontend should show an approval dialog and respond with ToolApprovalResponse
    ToolApprovalRequest {
        request_id: String,
        tool_name: String,
        args: serde_json::Value,
        /// Current approval stats for this tool (if any)
        stats: Option<ApprovalPattern>,
        /// Risk level of this operation
        risk_level: RiskLevel,
        /// Whether this tool can be auto-approved in the future
        can_learn: bool,
        /// Suggestion message (e.g., "2 more approvals needed for auto-approve")
        suggestion: Option<String>,
        /// Source of this tool call (main agent, sub-agent, or workflow)
        #[serde(default)]
        source: ToolSource,
    },

    /// Tool was auto-approved based on learned patterns
    ToolAutoApproved {
        request_id: String,
        tool_name: String,
        args: serde_json::Value,
        /// Reason for auto-approval
        reason: String,
        /// Source of this tool call (main agent, sub-agent, or workflow)
        #[serde(default)]
        source: ToolSource,
    },

    /// Tool was denied by policy or constraint
    ToolDenied {
        request_id: String,
        tool_name: String,
        args: serde_json::Value,
        /// Reason for denial
        reason: String,
        /// Source of this tool call (main agent, sub-agent, or workflow)
        #[serde(default)]
        source: ToolSource,
    },

    /// Tool execution completed
    ToolResult {
        tool_name: String,
        result: serde_json::Value,
        success: bool,
        request_id: String,
        /// Source of this tool call (main agent, sub-agent, or workflow)
        #[serde(default)]
        source: ToolSource,
    },

    /// Agent reasoning/thinking (for models that support extended thinking)
    Reasoning { content: String },

    /// Turn completed successfully
    Completed {
        response: String,
        tokens_used: Option<u32>,
        duration_ms: Option<u64>,
    },

    /// Error occurred during processing
    Error { message: String, error_type: String },

    // Sub-agent events
    /// Sub-agent started executing a task
    SubAgentStarted {
        agent_id: String,
        agent_name: String,
        task: String,
        depth: usize,
    },

    /// Sub-agent tool request (for visibility into sub-agent's tool usage)
    SubAgentToolRequest {
        agent_id: String,
        tool_name: String,
        args: serde_json::Value,
    },

    /// Sub-agent tool result
    SubAgentToolResult {
        agent_id: String,
        tool_name: String,
        success: bool,
    },

    /// Sub-agent completed its task
    SubAgentCompleted {
        agent_id: String,
        response: String,
        duration_ms: u64,
    },

    /// Sub-agent encountered an error
    SubAgentError { agent_id: String, error: String },

    // Context management events
    /// Context was pruned due to token limits
    ContextPruned {
        messages_removed: usize,
        utilization_before: f64,
        utilization_after: f64,
    },

    /// Context warning threshold exceeded
    ContextWarning {
        utilization: f64,
        total_tokens: usize,
        max_tokens: usize,
    },

    /// Tool response was truncated due to size limits
    ToolResponseTruncated {
        tool_name: String,
        original_tokens: usize,
        truncated_tokens: usize,
    },

    // Loop protection events
    /// Warning: approaching loop detection threshold
    LoopWarning {
        tool_name: String,
        current_count: usize,
        max_count: usize,
        message: String,
    },

    /// Tool call blocked due to loop detection
    LoopBlocked {
        tool_name: String,
        repeat_count: usize,
        max_count: usize,
        message: String,
    },

    /// Maximum tool iterations reached for this turn
    MaxIterationsReached {
        iterations: usize,
        max_iterations: usize,
        message: String,
    },

    // Workflow events
    /// Workflow started
    WorkflowStarted {
        workflow_id: String,
        workflow_name: String,
        session_id: String,
    },

    /// Workflow step started
    WorkflowStepStarted {
        workflow_id: String,
        step_name: String,
        step_index: usize,
        total_steps: usize,
    },

    /// Workflow step completed
    WorkflowStepCompleted {
        workflow_id: String,
        step_name: String,
        output: Option<String>,
        duration_ms: u64,
    },

    /// Workflow completed
    WorkflowCompleted {
        workflow_id: String,
        final_output: String,
        total_duration_ms: u64,
    },

    /// Workflow error
    WorkflowError {
        workflow_id: String,
        step_name: Option<String>,
        error: String,
    },
}
