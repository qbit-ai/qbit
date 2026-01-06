//! AI module - re-exports from qbit-ai crate.

#[cfg(feature = "tauri")]
pub mod commands;

// Re-export all qbit-ai types and modules
pub use qbit_ai::*;

#[cfg(feature = "tauri")]
pub use commands::{
    add_tool_always_allow, cancel_workflow, clear_ai_conversation, clear_ai_conversation_session,
    disable_full_auto_mode, disable_loop_detection, enable_full_auto_mode, enable_loop_detection,
    enforce_context_window, execute_ai_tool, export_ai_session_transcript, finalize_ai_session,
    find_ai_session, generate_commit_message, get_agent_mode, get_ai_conversation_length,
    get_ai_conversation_length_session, get_approval_patterns, get_available_tools,
    get_context_summary, get_context_trim_config, get_context_utilization, get_hitl_config,
    get_loop_detector_stats, get_loop_protection_config, get_openai_api_key,
    get_openrouter_api_key, get_plan, get_remaining_tokens, get_session_ai_config,
    get_token_alert_level, get_token_usage_stats, get_tool_approval_pattern, get_tool_policy,
    get_tool_policy_config, get_vertex_ai_config, get_vision_capabilities, get_workflow_state,
    init_ai_agent, init_ai_agent_openai, init_ai_agent_unified, init_ai_agent_vertex,
    init_ai_session, is_ai_initialized, is_ai_session_initialized,
    is_ai_session_persistence_enabled, is_context_management_enabled, is_full_auto_mode_enabled,
    is_loop_detection_enabled, list_ai_sessions, list_sub_agents, list_workflow_sessions,
    list_workflows, load_ai_session, load_env_file, remove_tool_always_allow,
    reset_approval_patterns, reset_context_manager, reset_loop_detector, reset_tool_policies,
    respond_to_tool_approval, restore_ai_session, run_workflow_to_completion, send_ai_prompt,
    send_ai_prompt_session, send_ai_prompt_with_attachments, set_agent_mode,
    set_ai_session_persistence, set_hitl_config, set_loop_protection_config, set_tool_policy,
    set_tool_policy_config, shutdown_ai_agent, shutdown_ai_session, start_workflow, step_workflow,
    update_ai_workspace, AiState, CommitMessageResponse,
};
