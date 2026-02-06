//! Characterization tests for AiEvent serialization.
//!
//! These tests capture the EXACT JSON format that the frontend expects.
//! They serve as regression tests - any changes to the serialization format
//! will cause these tests to fail, preventing accidental breaking changes.
//!
//! DO NOT modify snapshots without careful consideration of frontend impact!

use chrono::{DateTime, Utc};
use qbit_ai::planner::{PlanStep, PlanSummary, StepStatus};
use qbit_core::events::{AiEvent, ToolSource};
use qbit_core::hitl::{ApprovalPattern, RiskLevel};
use serde_json::json;

/// Test serialization of Started event
#[test]
fn test_started_event_serialization() {
    let event = AiEvent::Started {
        turn_id: "turn-123".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of TextDelta event
#[test]
fn test_text_delta_event_serialization() {
    let event = AiEvent::TextDelta {
        delta: "Hello".to_string(),
        accumulated: "Hello world".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolRequest event with Main source
#[test]
fn test_tool_request_main_source_serialization() {
    let event = AiEvent::ToolRequest {
        tool_name: "read_file".to_string(),
        args: json!({"path": "/src/main.rs"}),
        request_id: "req-456".to_string(),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolRequest event with SubAgent source
#[test]
fn test_tool_request_sub_agent_source_serialization() {
    let event = AiEvent::ToolRequest {
        tool_name: "read_file".to_string(),
        args: json!({"path": "/src/lib.rs"}),
        request_id: "req-789".to_string(),
        source: ToolSource::SubAgent {
            agent_id: "agent-001".to_string(),
            agent_name: "analyzer".to_string(),
        },
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolRequest event with Workflow source
#[test]
fn test_tool_request_workflow_source_serialization() {
    let event = AiEvent::ToolRequest {
        tool_name: "write_file".to_string(),
        args: json!({"path": "/output.txt", "content": "test"}),
        request_id: "req-999".to_string(),
        source: ToolSource::Workflow {
            workflow_id: "wf-001".to_string(),
            workflow_name: "git_commit".to_string(),
            step_name: Some("analyze".to_string()),
            step_index: Some(0),
        },
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolApprovalRequest event with full stats
#[test]
fn test_tool_approval_request_with_stats_serialization() {
    let event = AiEvent::ToolApprovalRequest {
        request_id: "req-approval-1".to_string(),
        tool_name: "write_file".to_string(),
        args: json!({"path": "/src/lib.rs", "content": "// code"}),
        stats: Some(ApprovalPattern {
            tool_name: "write_file".to_string(),
            total_requests: 5,
            approvals: 4,
            denials: 1,
            always_allow: false,
            last_updated: DateTime::<Utc>::from_timestamp(1700000000, 0).unwrap(),
            justifications: vec!["User approved".to_string(), "Safe operation".to_string()],
        }),
        risk_level: RiskLevel::Medium,
        can_learn: true,
        suggestion: Some("1 more approval for auto-approve".to_string()),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolApprovalRequest event without stats
#[test]
fn test_tool_approval_request_without_stats_serialization() {
    let event = AiEvent::ToolApprovalRequest {
        request_id: "req-approval-2".to_string(),
        tool_name: "shell_exec".to_string(),
        args: json!({"command": "ls -la"}),
        stats: None,
        risk_level: RiskLevel::High,
        can_learn: false,
        suggestion: None,
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolAutoApproved event
#[test]
fn test_tool_auto_approved_serialization() {
    let event = AiEvent::ToolAutoApproved {
        request_id: "req-auto-1".to_string(),
        tool_name: "read_file".to_string(),
        args: json!({"path": "/readme.md"}),
        reason: "Always allowed by user".to_string(),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolDenied event
#[test]
fn test_tool_denied_serialization() {
    let event = AiEvent::ToolDenied {
        request_id: "req-denied-1".to_string(),
        tool_name: "shell_exec".to_string(),
        args: json!({"command": "rm -rf /"}),
        reason: "Dangerous command blocked".to_string(),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolResult event with success
#[test]
fn test_tool_result_success_serialization() {
    let event = AiEvent::ToolResult {
        tool_name: "read_file".to_string(),
        result: json!({"content": "file contents here"}),
        success: true,
        request_id: "req-result-1".to_string(),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolResult event with failure
#[test]
fn test_tool_result_failure_serialization() {
    let event = AiEvent::ToolResult {
        tool_name: "write_file".to_string(),
        result: json!({"error": "Permission denied"}),
        success: false,
        request_id: "req-result-2".to_string(),
        source: ToolSource::Main,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of Reasoning event
#[test]
fn test_reasoning_event_serialization() {
    let event = AiEvent::Reasoning {
        content: "Let me think about this... I should first check the file structure.".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of Completed event with all fields
#[test]
fn test_completed_event_with_all_fields_serialization() {
    let event = AiEvent::Completed {
        response: "Task completed successfully.".to_string(),
        reasoning: Some("Let me think about this...".to_string()),
        input_tokens: Some(1000),
        output_tokens: Some(500),
        duration_ms: Some(2500),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of Completed event with optional fields as None
#[test]
fn test_completed_event_with_none_fields_serialization() {
    let event = AiEvent::Completed {
        response: "Done".to_string(),
        reasoning: None,
        input_tokens: None,
        output_tokens: None,
        duration_ms: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of Error event
#[test]
fn test_error_event_serialization() {
    let event = AiEvent::Error {
        message: "Connection timeout".to_string(),
        error_type: "network".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of SubAgentStarted event
#[test]
fn test_sub_agent_started_serialization() {
    let event = AiEvent::SubAgentStarted {
        agent_id: "agent-001".to_string(),
        agent_name: "analyzer".to_string(),
        task: "Analyze the codebase structure".to_string(),
        depth: 1,
        parent_request_id: "parent-req-001".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of SubAgentToolRequest event
#[test]
fn test_sub_agent_tool_request_serialization() {
    let event = AiEvent::SubAgentToolRequest {
        agent_id: "agent-001".to_string(),
        tool_name: "read_file".to_string(),
        args: json!({"path": "/config.json"}),
        request_id: "req-sub-1".to_string(),
        parent_request_id: "parent-req-001".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of SubAgentToolResult event
#[test]
fn test_sub_agent_tool_result_serialization() {
    let event = AiEvent::SubAgentToolResult {
        agent_id: "agent-001".to_string(),
        tool_name: "read_file".to_string(),
        success: true,
        result: json!({"content": "config data"}),
        request_id: "req-sub-1".to_string(),
        parent_request_id: "parent-req-001".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of SubAgentCompleted event
#[test]
fn test_sub_agent_completed_serialization() {
    let event = AiEvent::SubAgentCompleted {
        agent_id: "agent-001".to_string(),
        response: "Analysis complete".to_string(),
        duration_ms: 5000,
        parent_request_id: "parent-req-001".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of SubAgentError event
#[test]
fn test_sub_agent_error_serialization() {
    let event = AiEvent::SubAgentError {
        agent_id: "agent-001".to_string(),
        error: "Failed to parse response".to_string(),
        parent_request_id: "parent-req-001".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ContextWarning event
#[test]
fn test_context_warning_serialization() {
    let event = AiEvent::ContextWarning {
        utilization: 0.85,
        total_tokens: 170000,
        max_tokens: 200000,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolResponseTruncated event
#[test]
fn test_tool_response_truncated_serialization() {
    let event = AiEvent::ToolResponseTruncated {
        tool_name: "read_file".to_string(),
        original_tokens: 50000,
        truncated_tokens: 10000,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of LoopWarning event
#[test]
fn test_loop_warning_serialization() {
    let event = AiEvent::LoopWarning {
        tool_name: "list_files".to_string(),
        current_count: 8,
        max_count: 10,
        message: "Approaching loop limit".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of LoopBlocked event
#[test]
fn test_loop_blocked_serialization() {
    let event = AiEvent::LoopBlocked {
        tool_name: "list_files".to_string(),
        repeat_count: 10,
        max_count: 10,
        message: "Loop detected, blocking further calls".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of MaxIterationsReached event
#[test]
fn test_max_iterations_reached_serialization() {
    let event = AiEvent::MaxIterationsReached {
        iterations: 50,
        max_iterations: 50,
        message: "Maximum tool iterations reached".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowStarted event
#[test]
fn test_workflow_started_serialization() {
    let event = AiEvent::WorkflowStarted {
        workflow_id: "wf-001".to_string(),
        workflow_name: "git_commit".to_string(),
        session_id: "session-123".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowStepStarted event
#[test]
fn test_workflow_step_started_serialization() {
    let event = AiEvent::WorkflowStepStarted {
        workflow_id: "wf-001".to_string(),
        step_name: "analyze".to_string(),
        step_index: 0,
        total_steps: 4,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowStepCompleted event with output
#[test]
fn test_workflow_step_completed_with_output_serialization() {
    let event = AiEvent::WorkflowStepCompleted {
        workflow_id: "wf-001".to_string(),
        step_name: "analyze".to_string(),
        output: Some("Analysis complete".to_string()),
        duration_ms: 1500,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowStepCompleted event without output
#[test]
fn test_workflow_step_completed_without_output_serialization() {
    let event = AiEvent::WorkflowStepCompleted {
        workflow_id: "wf-001".to_string(),
        step_name: "prepare".to_string(),
        output: None,
        duration_ms: 500,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowCompleted event
#[test]
fn test_workflow_completed_serialization() {
    let event = AiEvent::WorkflowCompleted {
        workflow_id: "wf-001".to_string(),
        final_output: "Commit created successfully".to_string(),
        total_duration_ms: 8500,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowError event with step name
#[test]
fn test_workflow_error_with_step_serialization() {
    let event = AiEvent::WorkflowError {
        workflow_id: "wf-001".to_string(),
        step_name: Some("commit".to_string()),
        error: "Git commit failed".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of WorkflowError event without step name
#[test]
fn test_workflow_error_without_step_serialization() {
    let event = AiEvent::WorkflowError {
        workflow_id: "wf-001".to_string(),
        step_name: None,
        error: "Workflow initialization failed".to_string(),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of PlanUpdated event with all fields
#[test]
fn test_plan_updated_with_explanation_serialization() {
    let event = AiEvent::PlanUpdated {
        version: 2,
        summary: PlanSummary {
            total: 4,
            completed: 1,
            in_progress: 1,
            pending: 2,
        },
        steps: vec![
            PlanStep {
                step: "Analyze the codebase".to_string(),
                status: StepStatus::Completed,
            },
            PlanStep {
                step: "Implement the feature".to_string(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                step: "Write tests".to_string(),
                status: StepStatus::Pending,
            },
            PlanStep {
                step: "Update documentation".to_string(),
                status: StepStatus::Pending,
            },
        ],
        explanation: Some("Updated plan based on code analysis results".to_string()),
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of PlanUpdated event without explanation
#[test]
fn test_plan_updated_without_explanation_serialization() {
    let event = AiEvent::PlanUpdated {
        version: 1,
        summary: PlanSummary {
            total: 2,
            completed: 0,
            in_progress: 0,
            pending: 2,
        },
        steps: vec![
            PlanStep {
                step: "Research the problem".to_string(),
                status: StepStatus::Pending,
            },
            PlanStep {
                step: "Implement solution".to_string(),
                status: StepStatus::Pending,
            },
        ],
        explanation: None,
    };
    let json = serde_json::to_value(&event).unwrap();
    insta::assert_json_snapshot!(json);
}

// ============================================================================
// ToolSource Serialization Tests
// ============================================================================

/// Test serialization of ToolSource::Main
#[test]
fn test_tool_source_main_serialization() {
    let source = ToolSource::Main;
    let json = serde_json::to_value(&source).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolSource::SubAgent
#[test]
fn test_tool_source_sub_agent_serialization() {
    let source = ToolSource::SubAgent {
        agent_id: "agent-001".to_string(),
        agent_name: "analyzer".to_string(),
    };
    let json = serde_json::to_value(&source).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolSource::Workflow with step info
#[test]
fn test_tool_source_workflow_with_step_serialization() {
    let source = ToolSource::Workflow {
        workflow_id: "wf-001".to_string(),
        workflow_name: "git_commit".to_string(),
        step_name: Some("analyze".to_string()),
        step_index: Some(0),
    };
    let json = serde_json::to_value(&source).unwrap();
    insta::assert_json_snapshot!(json);
}

/// Test serialization of ToolSource::Workflow without step info
#[test]
fn test_tool_source_workflow_without_step_serialization() {
    let source = ToolSource::Workflow {
        workflow_id: "wf-001".to_string(),
        workflow_name: "git_commit".to_string(),
        step_name: None,
        step_index: None,
    };
    let json = serde_json::to_value(&source).unwrap();
    insta::assert_json_snapshot!(json);
}

// ============================================================================
// Roundtrip Tests (Serialization → Deserialization)
// ============================================================================

/// Test that all events can roundtrip without loss of data
#[test]
fn test_all_events_roundtrip() {
    let test_events = vec![
        AiEvent::Started {
            turn_id: "turn-1".to_string(),
        },
        AiEvent::TextDelta {
            delta: "Hello".to_string(),
            accumulated: "Hello world".to_string(),
        },
        AiEvent::ToolRequest {
            tool_name: "read_file".to_string(),
            args: json!({"path": "/test"}),
            request_id: "req-1".to_string(),
            source: ToolSource::Main,
        },
        AiEvent::ToolApprovalRequest {
            request_id: "req-2".to_string(),
            tool_name: "write_file".to_string(),
            args: json!({}),
            stats: None,
            risk_level: RiskLevel::High,
            can_learn: false,
            suggestion: None,
            source: ToolSource::Main,
        },
        AiEvent::ToolAutoApproved {
            request_id: "req-3".to_string(),
            tool_name: "read_file".to_string(),
            args: json!({}),
            reason: "Always allowed".to_string(),
            source: ToolSource::Main,
        },
        AiEvent::ToolDenied {
            request_id: "req-4".to_string(),
            tool_name: "shell".to_string(),
            args: json!({}),
            reason: "Blocked".to_string(),
            source: ToolSource::Main,
        },
        AiEvent::ToolResult {
            tool_name: "read_file".to_string(),
            result: json!("content"),
            success: true,
            request_id: "req-5".to_string(),
            source: ToolSource::Main,
        },
        AiEvent::Reasoning {
            content: "Thinking...".to_string(),
        },
        AiEvent::Completed {
            response: "Done".to_string(),
            reasoning: None,
            input_tokens: Some(60),
            output_tokens: Some(40),
            duration_ms: Some(500),
        },
        AiEvent::Error {
            message: "Failed".to_string(),
            error_type: "api".to_string(),
        },
        AiEvent::SubAgentStarted {
            agent_id: "a1".to_string(),
            agent_name: "analyzer".to_string(),
            task: "analyze".to_string(),
            depth: 1,
            parent_request_id: "parent-req-1".to_string(),
        },
        AiEvent::SubAgentToolRequest {
            agent_id: "a1".to_string(),
            tool_name: "read_file".to_string(),
            args: json!({}),
            request_id: "req-1".to_string(),
            parent_request_id: "parent-req-1".to_string(),
        },
        AiEvent::SubAgentToolResult {
            agent_id: "a1".to_string(),
            tool_name: "read_file".to_string(),
            success: true,
            result: json!({"content": "file contents"}),
            request_id: "req-1".to_string(),
            parent_request_id: "parent-req-1".to_string(),
        },
        AiEvent::SubAgentCompleted {
            agent_id: "a1".to_string(),
            response: "Done".to_string(),
            duration_ms: 1000,
            parent_request_id: "parent-req-1".to_string(),
        },
        AiEvent::SubAgentError {
            agent_id: "a1".to_string(),
            error: "Failed".to_string(),
            parent_request_id: "parent-req-1".to_string(),
        },
        AiEvent::ContextWarning {
            utilization: 0.85,
            total_tokens: 170000,
            max_tokens: 200000,
        },
        AiEvent::ToolResponseTruncated {
            tool_name: "read_file".to_string(),
            original_tokens: 50000,
            truncated_tokens: 10000,
        },
        AiEvent::LoopWarning {
            tool_name: "list".to_string(),
            current_count: 8,
            max_count: 10,
            message: "Warning".to_string(),
        },
        AiEvent::LoopBlocked {
            tool_name: "list".to_string(),
            repeat_count: 10,
            max_count: 10,
            message: "Blocked".to_string(),
        },
        AiEvent::MaxIterationsReached {
            iterations: 50,
            max_iterations: 50,
            message: "Max reached".to_string(),
        },
        AiEvent::WorkflowStarted {
            workflow_id: "wf1".to_string(),
            workflow_name: "git_commit".to_string(),
            session_id: "s1".to_string(),
        },
        AiEvent::WorkflowStepStarted {
            workflow_id: "wf1".to_string(),
            step_name: "analyze".to_string(),
            step_index: 0,
            total_steps: 4,
        },
        AiEvent::WorkflowStepCompleted {
            workflow_id: "wf1".to_string(),
            step_name: "analyze".to_string(),
            output: Some("Done".to_string()),
            duration_ms: 1000,
        },
        AiEvent::WorkflowCompleted {
            workflow_id: "wf1".to_string(),
            final_output: "Complete".to_string(),
            total_duration_ms: 5000,
        },
        AiEvent::WorkflowError {
            workflow_id: "wf1".to_string(),
            step_name: Some("commit".to_string()),
            error: "Failed".to_string(),
        },
        AiEvent::PlanUpdated {
            version: 1,
            summary: PlanSummary {
                total: 2,
                completed: 0,
                in_progress: 1,
                pending: 1,
            },
            steps: vec![
                PlanStep {
                    step: "Step 1".to_string(),
                    status: StepStatus::InProgress,
                },
                PlanStep {
                    step: "Step 2".to_string(),
                    status: StepStatus::Pending,
                },
            ],
            explanation: None,
        },
    ];

    for event in test_events {
        // Serialize to JSON string
        let json_str = serde_json::to_string(&event).expect("serialize failed");

        // Deserialize back to AiEvent
        let roundtrip: AiEvent = serde_json::from_str(&json_str).expect("deserialize failed");

        // Verify roundtrip produces identical JSON
        let original_json = serde_json::to_value(&event).unwrap();
        let roundtrip_json = serde_json::to_value(&roundtrip).unwrap();

        assert_eq!(
            original_json,
            roundtrip_json,
            "Roundtrip failed for event type: {}",
            event.event_type()
        );
    }
}

/// Test deserialization from actual JSON strings (simulating frontend)
#[test]
fn test_deserialize_from_json_strings() {
    // Started event
    let json = r#"{"type":"started","turn_id":"turn-123"}"#;
    let event: AiEvent = serde_json::from_str(json).expect("failed to deserialize Started");
    match event {
        AiEvent::Started { turn_id } => assert_eq!(turn_id, "turn-123"),
        _ => panic!("Wrong event type"),
    }

    // TextDelta event
    let json = r#"{"type":"text_delta","delta":"Hi","accumulated":"Hi there"}"#;
    let event: AiEvent = serde_json::from_str(json).expect("failed to deserialize TextDelta");
    match event {
        AiEvent::TextDelta { delta, accumulated } => {
            assert_eq!(delta, "Hi");
            assert_eq!(accumulated, "Hi there");
        }
        _ => panic!("Wrong event type"),
    }

    // Error event
    let json = r#"{"type":"error","message":"Something went wrong","error_type":"unknown"}"#;
    let event: AiEvent = serde_json::from_str(json).expect("failed to deserialize Error");
    match event {
        AiEvent::Error {
            message,
            error_type,
        } => {
            assert_eq!(message, "Something went wrong");
            assert_eq!(error_type, "unknown");
        }
        _ => panic!("Wrong event type"),
    }
}
