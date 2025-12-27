use async_trait::async_trait;
use parking_lot::RwLock;
use qbit_core::events::AiEvent;
use qbit_core::hitl::RiskLevel;
use qbit_core::runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};
use serde::Serialize;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Serialize)]
struct TerminalOutputEvent {
    session_id: String,
    data: String,
}

pub struct TauriRuntime {
    app_handle: AppHandle,
    pending_approvals: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalResult>>>>,
}

impl TauriRuntime {
    pub fn new(app_handle: AppHandle) -> Self {
        Self {
            app_handle,
            pending_approvals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Called by frontend when user responds to approval dialog
    ///
    /// This is exposed as a Tauri command:
    /// ```rust,ignore
    /// #[tauri::command]
    /// pub async fn respond_to_tool_approval(
    ///     app_state: tauri::State<'_, AppState>,
    ///     request_id: String,
    ///     approved: bool,
    /// ) -> Result<(), String> {
    ///     let decision = if approved {
    ///         ApprovalResult::Approved
    ///     } else {
    ///         ApprovalResult::Denied
    ///     };
    ///     app_state.runtime.respond_to_approval(&request_id, decision);
    ///     Ok(())
    /// }
    /// ```
    pub fn respond_to_approval(&self, request_id: &str, decision: ApprovalResult) {
        let pending_count = self.pending_approvals.read().len();

        if let Some(tx) = self.pending_approvals.write().remove(request_id) {
            match tx.send(decision) {
                Ok(()) => tracing::debug!(
                    request_id = %request_id,
                    decision = ?decision,
                    remaining_pending = pending_count - 1,
                    "Approval response delivered"
                ),
                Err(_) => tracing::warn!(
                    request_id = %request_id,
                    decision = ?decision,
                    "Approval response failed - receiver dropped (likely timed out)"
                ),
            }
        } else {
            tracing::warn!(
                request_id = %request_id,
                pending_count = pending_count,
                "Approval response for unknown request_id"
            );
        }
    }
}

/// AI event payload with session_id for routing
#[derive(Debug, Clone, Serialize)]
struct AiEventPayload<'a> {
    session_id: &'a str,
    #[serde(flatten)]
    event: &'a qbit_core::events::AiEvent,
}

#[async_trait]
impl QbitRuntime for TauriRuntime {
    fn emit(&self, event: RuntimeEvent) -> Result<(), RuntimeError> {
        // Emit with appropriate event name based on the RuntimeEvent variant
        match &event {
            RuntimeEvent::Ai {
                session_id,
                event: ai_event,
            } => {
                // AI events go to ai-event channel with session_id for routing
                let event_type = ai_event.event_type();
                let payload = AiEventPayload {
                    session_id,
                    event: ai_event,
                };
                self.app_handle.emit("ai-event", &payload).map_err(|e| {
                    tracing::error!(
                        channel = "ai-event",
                        session_id = %session_id,
                        event_type = %event_type,
                        error = %e,
                        "Failed to emit AI event"
                    );
                    RuntimeError::EmitFailed(e.to_string())
                })?;
                tracing::trace!(
                    channel = "ai-event",
                    session_id = %session_id,
                    event_type = %event_type,
                    "Emitted AI event"
                );
            }
            RuntimeEvent::TerminalOutput { session_id, data } => {
                // Terminal output goes to terminal_output channel
                let output_str = String::from_utf8_lossy(data).to_string();
                let byte_count = data.len();
                self.app_handle
                    .emit(
                        "terminal_output",
                        TerminalOutputEvent {
                            session_id: session_id.clone(),
                            data: output_str,
                        },
                    )
                    .map_err(|e| {
                        tracing::error!(
                            channel = "terminal_output",
                            session_id = %session_id,
                            bytes = byte_count,
                            error = %e,
                            "Failed to emit terminal output"
                        );
                        RuntimeError::EmitFailed(e.to_string())
                    })?;
                // Terminal output is high-frequency, only log large chunks
                if byte_count > 1024 {
                    tracing::trace!(
                        channel = "terminal_output",
                        session_id = %session_id,
                        bytes = byte_count,
                        "Emitted large terminal output"
                    );
                }
            }
            RuntimeEvent::TerminalExit { session_id, code } => {
                // Session ended goes to session_ended channel
                self.app_handle
                    .emit(
                        "session_ended",
                        serde_json::json!({
                            "sessionId": session_id
                        }),
                    )
                    .map_err(|e| {
                        tracing::error!(
                            channel = "session_ended",
                            session_id = %session_id,
                            error = %e,
                            "Failed to emit session ended"
                        );
                        RuntimeError::EmitFailed(e.to_string())
                    })?;
                tracing::info!(
                    channel = "session_ended",
                    session_id = %session_id,
                    exit_code = ?code,
                    "Emitted session ended"
                );
            }
            RuntimeEvent::Custom { name, payload } => {
                // Custom events use the specified name
                self.app_handle.emit(name, payload).map_err(|e| {
                    tracing::error!(
                        channel = %name,
                        error = %e,
                        "Failed to emit custom event"
                    );
                    RuntimeError::EmitFailed(e.to_string())
                })?;
                tracing::trace!(
                    channel = %name,
                    "Emitted custom event"
                );
            }
        }
        Ok(())
    }

    async fn request_approval(
        &self,
        request_id: String,
        tool_name: String,
        args: serde_json::Value,
        risk_level: String,
    ) -> Result<ApprovalResult, RuntimeError> {
        let pending_count = self.pending_approvals.read().len();

        tracing::debug!(
            request_id = %request_id,
            tool = %tool_name,
            risk = %risk_level,
            pending_count = pending_count,
            "Requesting tool approval"
        );

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Insert into map (lock dropped immediately)
        {
            self.pending_approvals
                .write()
                .insert(request_id.clone(), tx);
        }

        // Parse risk level from string (default to High if unknown)
        let risk = match risk_level.to_lowercase().as_str() {
            "low" => RiskLevel::Low,
            "medium" => RiskLevel::Medium,
            "high" => RiskLevel::High,
            "critical" => RiskLevel::Critical,
            _ => {
                tracing::warn!(
                    request_id = %request_id,
                    provided_level = %risk_level,
                    "Unknown risk level, defaulting to High"
                );
                RiskLevel::High
            }
        };

        // Emit approval request to frontend
        // Note: This approval path doesn't have session context - use "unknown" as placeholder.
        // The main approval flow goes through AgentBridge::emit_event() which has proper session_id.
        self.emit(RuntimeEvent::Ai {
            session_id: "unknown".to_string(),
            event: Box::new(AiEvent::ToolApprovalRequest {
                request_id: request_id.clone(),
                tool_name: tool_name.clone(),
                args,
                stats: None,
                risk_level: risk,
                can_learn: true,
                suggestion: None,
                source: Default::default(),
            }),
        })?;

        // Wait for response with 5-minute timeout
        let start = std::time::Instant::now();
        match tokio::time::timeout(Duration::from_secs(300), rx).await {
            Ok(Ok(decision)) => {
                tracing::debug!(
                    request_id = %request_id,
                    tool = %tool_name,
                    decision = ?decision,
                    wait_ms = start.elapsed().as_millis(),
                    "Approval decision received"
                );
                Ok(decision)
            }
            Ok(Err(_)) => {
                // Sender dropped without sending - shouldn't happen
                tracing::warn!(
                    request_id = %request_id,
                    tool = %tool_name,
                    wait_ms = start.elapsed().as_millis(),
                    "Approval channel dropped unexpectedly"
                );
                self.pending_approvals.write().remove(&request_id);
                Err(RuntimeError::ApprovalTimeout(300))
            }
            Err(_) => {
                // Timeout - clean up pending approval
                tracing::warn!(
                    request_id = %request_id,
                    tool = %tool_name,
                    timeout_secs = 300,
                    "Approval request timed out"
                );
                self.pending_approvals.write().remove(&request_id);
                Err(RuntimeError::ApprovalTimeout(300))
            }
        }
    }

    fn is_interactive(&self) -> bool {
        true // Tauri always has UI
    }

    fn auto_approve(&self) -> bool {
        false // Tauri uses UI-based approval
    }

    async fn shutdown(&self) -> Result<(), RuntimeError> {
        // Cancel all pending approvals
        let pending = {
            let mut approvals = self.pending_approvals.write();
            std::mem::take(&mut *approvals)
        };

        for (_, tx) in pending {
            let _ = tx.send(ApprovalResult::Timeout);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
