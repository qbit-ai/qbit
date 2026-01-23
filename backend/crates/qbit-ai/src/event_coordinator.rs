//! Event Coordinator - Single-task message-passing coordinator for AI events.
//!
//! This module provides a centralized event coordinator that owns all event-related state
//! and processes commands in deterministic order. This eliminates deadlock possibilities
//! that can occur with lock-based mutable state.
//!
//! # Architecture
//!
//! ```text
//! AgentBridge                          EventCoordinator (single tokio task)
//! ┌─────────────────┐                  ┌─────────────────────────────────┐
//! │ coordinator:    │───send()───────▶│ Owns:                           │
//! │ CoordinatorHandle                  │  - event_sequence: u64          │
//! └─────────────────┘                  │  - frontend_ready: bool         │
//!                                      │  - event_buffer: Vec<Envelope>  │
//!                                      │  - pending_approvals: HashMap   │
//!                                      │                                 │
//!                                      │ Emits via:                      │
//!                                      │  - runtime: Arc<dyn QbitRuntime>│
//!                                      └─────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! // Create and spawn coordinator
//! let handle = EventCoordinator::spawn(
//!     session_id.to_string(),
//!     runtime.clone(),
//!     Some(transcript_writer.clone()),
//! );
//!
//! // Emit events (fire-and-forget)
//! handle.emit(AiEvent::Started { turn_id: "123".to_string() });
//!
//! // Mark frontend ready (flushes buffered events)
//! handle.mark_frontend_ready();
//!
//! // Register approval request (returns receiver for decision)
//! let decision_rx = handle.register_approval("request_123".to_string());
//!
//! // Resolve approval (from frontend response)
//! handle.resolve_approval(decision);
//!
//! // Query state (for debugging/testing)
//! let state = handle.query_state().await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use qbit_core::events::{AiEvent, AiEventEnvelope};
use qbit_core::hitl::ApprovalDecision;
use qbit_core::runtime::{QbitRuntime, RuntimeEvent};

use crate::transcript::TranscriptWriter;

/// Commands that can be sent to the EventCoordinator.
#[derive(Debug)]
pub enum CoordinatorCommand {
    /// Emit an AI event to the frontend.
    /// Boxed to reduce variant size disparity (AiEvent is large).
    EmitEvent { event: Box<AiEvent> },

    /// Mark the frontend as ready to receive events (flushes buffer).
    MarkFrontendReady,

    /// Register a pending approval request.
    /// The response will be sent back via the oneshot channel.
    RegisterApproval {
        request_id: String,
        response_tx: oneshot::Sender<ApprovalDecision>,
    },

    /// Resolve a pending approval with a decision.
    ResolveApproval { decision: ApprovalDecision },

    /// Query the current coordinator state (for debugging/testing).
    QueryState {
        response_tx: oneshot::Sender<CoordinatorState>,
    },

    /// Shutdown the coordinator.
    Shutdown,
}

/// Snapshot of coordinator state for debugging/testing.
#[derive(Debug, Clone)]
pub struct CoordinatorState {
    /// Current event sequence number.
    pub event_sequence: u64,
    /// Whether the frontend is ready.
    pub frontend_ready: bool,
    /// Number of buffered events.
    pub buffered_event_count: usize,
    /// Number of pending approvals.
    pub pending_approval_count: usize,
    /// List of pending approval request IDs.
    pub pending_approval_ids: Vec<String>,
}

/// Handle for sending commands to the EventCoordinator.
///
/// This handle is cheap to clone and can be passed around freely.
/// Commands are sent via an unbounded channel for fire-and-forget semantics.
#[derive(Clone)]
pub struct CoordinatorHandle {
    tx: mpsc::UnboundedSender<CoordinatorCommand>,
}

impl CoordinatorHandle {
    /// Emit an AI event (fire-and-forget).
    ///
    /// If the frontend is not ready, the event will be buffered.
    pub fn emit(&self, event: AiEvent) {
        let _ = self.tx.send(CoordinatorCommand::EmitEvent {
            event: Box::new(event),
        });
    }

    /// Mark the frontend as ready to receive events.
    ///
    /// This flushes any buffered events in sequence order.
    pub fn mark_frontend_ready(&self) {
        let _ = self.tx.send(CoordinatorCommand::MarkFrontendReady);
    }

    /// Register a pending approval request.
    ///
    /// Returns a receiver that will receive the approval decision
    /// when `resolve_approval` is called with a matching request ID.
    pub fn register_approval(&self, request_id: String) -> oneshot::Receiver<ApprovalDecision> {
        let (response_tx, response_rx) = oneshot::channel();
        let _ = self.tx.send(CoordinatorCommand::RegisterApproval {
            request_id,
            response_tx,
        });
        response_rx
    }

    /// Resolve a pending approval with a decision.
    ///
    /// The decision will be sent to the receiver registered with `register_approval`.
    pub fn resolve_approval(&self, decision: ApprovalDecision) {
        let _ = self.tx.send(CoordinatorCommand::ResolveApproval { decision });
    }

    /// Query the current coordinator state.
    ///
    /// Returns `None` if the coordinator has shut down.
    pub async fn query_state(&self) -> Option<CoordinatorState> {
        let (response_tx, response_rx) = oneshot::channel();
        if self
            .tx
            .send(CoordinatorCommand::QueryState { response_tx })
            .is_err()
        {
            return None;
        }
        response_rx.await.ok()
    }

    /// Shutdown the coordinator.
    pub fn shutdown(&self) {
        let _ = self.tx.send(CoordinatorCommand::Shutdown);
    }

    /// Check if the coordinator is still running.
    pub fn is_alive(&self) -> bool {
        !self.tx.is_closed()
    }
}

/// The EventCoordinator owns all event-related state and processes commands
/// in a single tokio task, ensuring deterministic ordering and eliminating deadlocks.
pub struct EventCoordinator {
    /// Monotonically increasing sequence number for events.
    event_sequence: u64,
    /// Whether the frontend has signaled it is ready to receive events.
    frontend_ready: bool,
    /// Buffer for events emitted before frontend signals ready.
    event_buffer: Vec<AiEventEnvelope>,
    /// Pending approval requests waiting for decisions.
    pending_approvals: HashMap<String, oneshot::Sender<ApprovalDecision>>,
    /// Session ID for event routing.
    session_id: String,
    /// Runtime for emitting events.
    runtime: Arc<dyn QbitRuntime>,
    /// Transcript writer for persisting events (optional).
    transcript_writer: Option<Arc<TranscriptWriter>>,
}

impl EventCoordinator {
    /// Spawn a new EventCoordinator task.
    ///
    /// Returns a handle for sending commands to the coordinator.
    pub fn spawn(
        session_id: String,
        runtime: Arc<dyn QbitRuntime>,
        transcript_writer: Option<Arc<TranscriptWriter>>,
    ) -> CoordinatorHandle {
        let (tx, rx) = mpsc::unbounded_channel();

        let coordinator = Self {
            event_sequence: 0,
            frontend_ready: false,
            event_buffer: Vec::new(),
            pending_approvals: HashMap::new(),
            session_id,
            runtime,
            transcript_writer,
        };

        tokio::spawn(coordinator.run(rx));

        CoordinatorHandle { tx }
    }

    /// Run the coordinator event loop.
    async fn run(mut self, mut rx: mpsc::UnboundedReceiver<CoordinatorCommand>) {
        tracing::debug!(
            session_id = %self.session_id,
            "EventCoordinator started"
        );

        while let Some(command) = rx.recv().await {
            match command {
                CoordinatorCommand::EmitEvent { event } => {
                    self.handle_emit_event(*event).await;
                }
                CoordinatorCommand::MarkFrontendReady => {
                    self.handle_mark_frontend_ready().await;
                }
                CoordinatorCommand::RegisterApproval {
                    request_id,
                    response_tx,
                } => {
                    self.handle_register_approval(request_id, response_tx);
                }
                CoordinatorCommand::ResolveApproval { decision } => {
                    self.handle_resolve_approval(decision);
                }
                CoordinatorCommand::QueryState { response_tx } => {
                    let state = CoordinatorState {
                        event_sequence: self.event_sequence,
                        frontend_ready: self.frontend_ready,
                        buffered_event_count: self.event_buffer.len(),
                        pending_approval_count: self.pending_approvals.len(),
                        pending_approval_ids: self.pending_approvals.keys().cloned().collect(),
                    };
                    let _ = response_tx.send(state);
                }
                CoordinatorCommand::Shutdown => {
                    tracing::debug!(
                        session_id = %self.session_id,
                        "EventCoordinator shutting down"
                    );
                    break;
                }
            }
        }

        tracing::debug!(
            session_id = %self.session_id,
            pending_approvals = self.pending_approvals.len(),
            buffered_events = self.event_buffer.len(),
            "EventCoordinator stopped"
        );
    }

    /// Create an event envelope with sequence number and timestamp.
    fn create_envelope(&mut self, event: AiEvent) -> AiEventEnvelope {
        let seq = self.event_sequence;
        self.event_sequence += 1;
        let ts = chrono::Utc::now().to_rfc3339();
        AiEventEnvelope { seq, ts, event }
    }

    /// Check if an event should be written to the transcript.
    fn should_transcript(event: &AiEvent) -> bool {
        // Skip streaming events and sub-agent internal events
        !matches!(
            event,
            AiEvent::TextDelta { .. }
                | AiEvent::Reasoning { .. }
                | AiEvent::SubAgentToolRequest { .. }
                | AiEvent::SubAgentToolResult { .. }
        )
    }

    /// Write an event to the transcript (if configured).
    async fn write_to_transcript(&self, event: &AiEvent) {
        if let Some(ref writer) = self.transcript_writer {
            if Self::should_transcript(event) {
                if let Err(e) = writer.append(event).await {
                    tracing::warn!("Failed to write to transcript: {}", e);
                }
            }
        }
    }

    /// Emit an envelope to the frontend via the runtime.
    fn emit_envelope(&self, envelope: AiEventEnvelope) {
        tracing::debug!(
            session_id = %self.session_id,
            seq = envelope.seq,
            event_type = envelope.event.event_type(),
            "Emitting event via coordinator"
        );

        if let Err(e) = self.runtime.emit(RuntimeEvent::AiEnvelope {
            session_id: self.session_id.clone(),
            envelope: Box::new(envelope),
        }) {
            tracing::warn!("Failed to emit event through runtime: {}", e);
        }
    }

    /// Handle EmitEvent command.
    async fn handle_emit_event(&mut self, event: AiEvent) {
        // Write to transcript
        self.write_to_transcript(&event).await;

        // Create envelope with sequence number
        let envelope = self.create_envelope(event);

        // If frontend is not ready, buffer the event
        if !self.frontend_ready {
            tracing::debug!(
                session_id = %self.session_id,
                seq = envelope.seq,
                event_type = envelope.event.event_type(),
                "Buffering event (frontend not ready)"
            );
            self.event_buffer.push(envelope);
            return;
        }

        // Emit directly
        self.emit_envelope(envelope);
    }

    /// Handle MarkFrontendReady command.
    async fn handle_mark_frontend_ready(&mut self) {
        let buffered_count = self.event_buffer.len();

        tracing::info!(
            session_id = %self.session_id,
            buffered_events = buffered_count,
            "Marking frontend ready, flushing buffered events"
        );

        // Set ready flag first
        self.frontend_ready = true;

        // Flush buffered events in order
        let buffered_events = std::mem::take(&mut self.event_buffer);
        for envelope in buffered_events {
            self.emit_envelope(envelope);
        }
    }

    /// Handle RegisterApproval command.
    fn handle_register_approval(
        &mut self,
        request_id: String,
        response_tx: oneshot::Sender<ApprovalDecision>,
    ) {
        tracing::debug!(
            session_id = %self.session_id,
            request_id = %request_id,
            "Registering approval request"
        );
        self.pending_approvals.insert(request_id, response_tx);
    }

    /// Handle ResolveApproval command.
    fn handle_resolve_approval(&mut self, decision: ApprovalDecision) {
        let request_id = &decision.request_id;

        if let Some(sender) = self.pending_approvals.remove(request_id) {
            tracing::debug!(
                session_id = %self.session_id,
                request_id = %request_id,
                approved = decision.approved,
                "Resolving approval request"
            );
            let _ = sender.send(decision);
        } else {
            tracing::warn!(
                session_id = %self.session_id,
                request_id = %request_id,
                "No pending approval found for request_id"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A simple mock runtime for testing.
    struct MockRuntime {
        emit_count: AtomicUsize,
    }

    impl MockRuntime {
        fn new() -> Self {
            Self {
                emit_count: AtomicUsize::new(0),
            }
        }

        fn emit_count(&self) -> usize {
            self.emit_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl QbitRuntime for MockRuntime {
        fn emit(&self, _event: RuntimeEvent) -> Result<(), qbit_core::runtime::RuntimeError> {
            self.emit_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn request_approval(
            &self,
            _request_id: String,
            _tool_name: String,
            _args: serde_json::Value,
            _risk_level: String,
        ) -> Result<qbit_core::runtime::ApprovalResult, qbit_core::runtime::RuntimeError> {
            Ok(qbit_core::runtime::ApprovalResult::Approved)
        }

        fn is_interactive(&self) -> bool {
            false
        }

        fn auto_approve(&self) -> bool {
            true
        }

        async fn shutdown(&self) -> Result<(), qbit_core::runtime::RuntimeError> {
            Ok(())
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    #[tokio::test]
    async fn test_event_sequencing() {
        let runtime = Arc::new(MockRuntime::new());
        let handle = EventCoordinator::spawn("test-session".to_string(), runtime.clone(), None);

        // Mark frontend ready first
        handle.mark_frontend_ready();
        tokio::task::yield_now().await;

        // Emit multiple events
        handle.emit(AiEvent::Started {
            turn_id: "1".to_string(),
        });
        handle.emit(AiEvent::Started {
            turn_id: "2".to_string(),
        });
        handle.emit(AiEvent::Started {
            turn_id: "3".to_string(),
        });

        // Give coordinator time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Query state
        let state = handle.query_state().await.unwrap();
        assert_eq!(state.event_sequence, 3);
        assert!(state.frontend_ready);
        assert_eq!(state.buffered_event_count, 0);

        // Check emit count
        assert_eq!(runtime.emit_count(), 3);

        handle.shutdown();
    }

    #[tokio::test]
    async fn test_buffering_before_frontend_ready() {
        let runtime = Arc::new(MockRuntime::new());
        let handle = EventCoordinator::spawn("test-session".to_string(), runtime.clone(), None);

        // Emit events before frontend is ready
        handle.emit(AiEvent::Started {
            turn_id: "1".to_string(),
        });
        handle.emit(AiEvent::Started {
            turn_id: "2".to_string(),
        });

        // Give coordinator time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Events should be buffered
        let state = handle.query_state().await.unwrap();
        assert!(!state.frontend_ready);
        assert_eq!(state.buffered_event_count, 2);
        assert_eq!(runtime.emit_count(), 0);

        // Mark frontend ready
        handle.mark_frontend_ready();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Buffer should be flushed
        let state = handle.query_state().await.unwrap();
        assert!(state.frontend_ready);
        assert_eq!(state.buffered_event_count, 0);
        assert_eq!(runtime.emit_count(), 2);

        handle.shutdown();
    }

    #[tokio::test]
    async fn test_approval_registration_and_resolution() {
        let runtime = Arc::new(MockRuntime::new());
        let handle = EventCoordinator::spawn("test-session".to_string(), runtime, None);

        // Register an approval
        let decision_rx = handle.register_approval("request-123".to_string());

        // Give coordinator time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Check state
        let state = handle.query_state().await.unwrap();
        assert_eq!(state.pending_approval_count, 1);
        assert!(state.pending_approval_ids.contains(&"request-123".to_string()));

        // Resolve the approval
        handle.resolve_approval(ApprovalDecision {
            request_id: "request-123".to_string(),
            approved: true,
            reason: Some("Test approval".to_string()),
            remember: false,
            always_allow: false,
        });

        // Receive the decision
        let decision = decision_rx.await.unwrap();
        assert!(decision.approved);
        assert_eq!(decision.request_id, "request-123");

        // Check state - approval should be removed
        let state = handle.query_state().await.unwrap();
        assert_eq!(state.pending_approval_count, 0);

        handle.shutdown();
    }

    #[tokio::test]
    async fn test_shutdown() {
        let runtime = Arc::new(MockRuntime::new());
        let handle = EventCoordinator::spawn("test-session".to_string(), runtime, None);

        assert!(handle.is_alive());

        handle.shutdown();
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // After shutdown, query_state should return None
        assert!(handle.query_state().await.is_none());
    }
}
