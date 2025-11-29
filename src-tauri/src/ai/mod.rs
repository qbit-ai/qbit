pub mod agent_bridge;
pub mod commands;
pub mod events;
pub mod session;
pub mod sub_agent;
pub mod workflow;

pub use commands::{
    clear_ai_conversation, execute_ai_tool, export_ai_session_transcript, finalize_ai_session,
    find_ai_session, get_ai_conversation_length, get_available_tools, get_openrouter_api_key,
    get_vertex_ai_config, init_ai_agent, init_ai_agent_vertex, is_ai_initialized,
    is_ai_session_persistence_enabled, list_ai_sessions, load_ai_session, load_env_file,
    restore_ai_session, send_ai_prompt, set_ai_session_persistence, shutdown_ai_agent,
    update_ai_workspace, AiState,
};
// Re-export session types for external use
#[allow(unused_imports)]
pub use session::{QbitMessageRole, QbitSessionMessage, QbitSessionSnapshot, SessionListingInfo};
// Re-exports for sub_agent and workflow modules - currently unused but kept for future use
#[allow(unused_imports)]
pub use sub_agent::{SubAgentContext, SubAgentDefinition, SubAgentRegistry, SubAgentResult};
#[allow(unused_imports)]
pub use workflow::{
    AgentWorkflowBuilder, RouterTask, SubAgentExecutor, SubAgentTask, WorkflowRunner,
    WorkflowStatus, WorkflowStepResult, WorkflowStorage,
};
