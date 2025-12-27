//! Plan management commands for retrieving the current task plan.
//!
//! These commands allow the frontend to query the current plan state.

use tauri::State;

use crate::state::AppState;
use crate::tools::TaskPlan;

use super::ai_session_not_initialized_error;

/// Get the current task plan for a session.
///
/// # Arguments
/// * `session_id` - The session ID to get the plan for
///
/// # Returns
/// The current TaskPlan with version, summary, and steps
#[tauri::command]
pub async fn get_plan(session_id: String, state: State<'_, AppState>) -> Result<TaskPlan, String> {
    let bridges = state.ai_state.bridges.read().await;
    let bridge = bridges
        .get(&session_id)
        .ok_or_else(|| ai_session_not_initialized_error(&session_id))?;

    // Get the current plan from the bridge's plan_manager
    let plan = bridge.plan_manager().snapshot().await;
    Ok(plan)
}
