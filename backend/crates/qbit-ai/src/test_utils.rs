//! Test utilities for the AI agent system.
//!
//! This module provides mock implementations and helpers for testing the
//! agentic loop, HITL approval flows, and tool routing logic.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::stream::{self, BoxStream};
use futures::StreamExt;
use rig::completion::{
    self, AssistantContent, CompletionError, CompletionRequest, CompletionResponse, GetTokenUsage,
    Usage,
};
use rig::message::{Reasoning, Text, ToolCall, ToolFunction};
use rig::one_or_many::OneOrMany;
use rig::streaming::{RawStreamingChoice, RawStreamingToolCall, StreamingCompletionResponse};
use serde::{Deserialize, Serialize};

/// A mock response that the MockCompletionModel will return.
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// Text content to return (if any)
    pub text: Option<String>,
    /// Tool calls to return (if any)
    pub tool_calls: Vec<MockToolCall>,
    /// Thinking/reasoning content to return (if any)
    pub thinking: Option<String>,
}

impl Default for MockResponse {
    fn default() -> Self {
        Self {
            text: Some("Mock response".to_string()),
            tool_calls: vec![],
            thinking: None,
        }
    }
}

impl MockResponse {
    /// Create a text-only response.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: Some(content.into()),
            tool_calls: vec![],
            thinking: None,
        }
    }

    /// Create a response with a tool call.
    pub fn tool_call(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            text: None,
            tool_calls: vec![MockToolCall {
                name: name.into(),
                args,
            }],
            thinking: None,
        }
    }

    /// Create a response with multiple tool calls.
    pub fn tool_calls(calls: Vec<MockToolCall>) -> Self {
        Self {
            text: None,
            tool_calls: calls,
            thinking: None,
        }
    }

    /// Create a response with thinking content.
    pub fn with_thinking(mut self, thinking: impl Into<String>) -> Self {
        self.thinking = Some(thinking.into());
        self
    }

    /// Create a response with text and thinking.
    pub fn text_with_thinking(text: impl Into<String>, thinking: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: vec![],
            thinking: Some(thinking.into()),
        }
    }
}

/// A mock tool call.
#[derive(Debug, Clone)]
pub struct MockToolCall {
    pub name: String,
    pub args: serde_json::Value,
}

impl MockToolCall {
    pub fn new(name: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}

/// Streaming response data for the mock model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockStreamingResponseData {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl Default for MockStreamingResponseData {
    fn default() -> Self {
        Self {
            text: String::new(),
            input_tokens: 100,
            output_tokens: 50,
        }
    }
}

impl GetTokenUsage for MockStreamingResponseData {
    fn token_usage(&self) -> Option<Usage> {
        Some(Usage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            total_tokens: self.input_tokens + self.output_tokens,
        })
    }
}

/// A mock CompletionModel for testing agentic loop behavior.
///
/// This model returns predefined responses in sequence, allowing
/// multi-turn testing of the agentic loop.
#[derive(Debug, Clone)]
pub struct MockCompletionModel {
    responses: Arc<Vec<MockResponse>>,
    current_index: Arc<AtomicUsize>,
}

impl MockCompletionModel {
    /// Create a new mock model with a sequence of responses.
    pub fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            responses: Arc::new(responses),
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Create a mock model that returns a single text response.
    pub fn with_text(text: impl Into<String>) -> Self {
        Self::new(vec![MockResponse::text(text)])
    }

    /// Create a mock model that returns a single tool call then text.
    pub fn with_tool_call_then_text(
        tool_name: impl Into<String>,
        tool_args: serde_json::Value,
        final_text: impl Into<String>,
    ) -> Self {
        Self::new(vec![
            MockResponse::tool_call(tool_name, tool_args),
            MockResponse::text(final_text),
        ])
    }

    /// Get the next response in the sequence.
    fn next_response(&self) -> MockResponse {
        let index = self.current_index.fetch_add(1, Ordering::SeqCst);
        if index < self.responses.len() {
            self.responses[index].clone()
        } else {
            // Return empty text response if we've exhausted all responses
            MockResponse::text("")
        }
    }

    /// Reset the response index to start from the beginning.
    pub fn reset(&self) {
        self.current_index.store(0, Ordering::SeqCst);
    }

    /// Get the number of times a response has been requested.
    pub fn call_count(&self) -> usize {
        self.current_index.load(Ordering::SeqCst)
    }

    /// Build a CompletionResponse from a MockResponse.
    fn build_completion_response(
        &self,
        mock_response: &MockResponse,
        call_count: usize,
    ) -> CompletionResponse<MockStreamingResponseData> {
        let mut content: Vec<AssistantContent> = vec![];

        // Add thinking content first (if any)
        if let Some(thinking) = &mock_response.thinking {
            content.push(AssistantContent::Reasoning(
                Reasoning::new(thinking).optional_id(Some(format!("mock-thinking-{}", call_count))),
            ));
        }

        // Add text content (if any)
        if let Some(text) = &mock_response.text {
            content.push(AssistantContent::Text(Text { text: text.clone() }));
        }

        // Add tool calls (if any)
        for (i, tool_call) in mock_response.tool_calls.iter().enumerate() {
            let id = format!("mock-tool-{}-{}", call_count, i);
            content.push(AssistantContent::ToolCall(ToolCall {
                id: id.clone(),
                call_id: Some(id),
                function: ToolFunction {
                    name: tool_call.name.clone(),
                    arguments: tool_call.args.clone(),
                },
                signature: None,
                additional_params: None,
            }));
        }

        let choice = if content.len() == 1 {
            OneOrMany::one(content.pop().unwrap())
        } else if content.is_empty() {
            OneOrMany::one(AssistantContent::Text(Text {
                text: String::new(),
            }))
        } else {
            OneOrMany::many(content).unwrap()
        };

        CompletionResponse {
            choice,
            usage: Usage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            },
            raw_response: MockStreamingResponseData::default(),
        }
    }

    /// Build streaming chunks from a MockResponse.
    fn build_stream_chunks(
        mock_response: &MockResponse,
        call_count: usize,
    ) -> Vec<RawStreamingChoice<MockStreamingResponseData>> {
        let mut chunks: Vec<RawStreamingChoice<MockStreamingResponseData>> = vec![];

        // Add thinking content first (if any)
        if let Some(thinking) = &mock_response.thinking {
            chunks.push(RawStreamingChoice::Reasoning {
                id: Some(format!("mock-thinking-{}", call_count)),
                reasoning: thinking.clone(),
                signature: Some("mock-signature".to_string()),
            });
        }

        // Add text content (if any)
        if let Some(text) = &mock_response.text {
            chunks.push(RawStreamingChoice::Message(text.clone()));
        }

        // Add tool calls (if any)
        for (i, tool_call) in mock_response.tool_calls.iter().enumerate() {
            let id = format!("mock-tool-{}-{}", call_count, i);
            chunks.push(RawStreamingChoice::ToolCall(RawStreamingToolCall {
                id: id.clone(),
                call_id: Some(id),
                name: tool_call.name.clone(),
                arguments: tool_call.args.clone(),
                signature: None,
                additional_params: None,
            }));
        }

        // Add final response
        chunks.push(RawStreamingChoice::FinalResponse(
            MockStreamingResponseData {
                text: mock_response.text.clone().unwrap_or_default(),
                input_tokens: 100,
                output_tokens: 50,
            },
        ));

        chunks
    }
}

impl completion::CompletionModel for MockCompletionModel {
    type Response = MockStreamingResponseData;
    type StreamingResponse = MockStreamingResponseData;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>) -> Self {
        Self::new(vec![MockResponse::default()])
    }

    async fn completion(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionResponse<Self::Response>, CompletionError> {
        let mock_response = self.next_response();
        let call_count = self.call_count();
        Ok(self.build_completion_response(&mock_response, call_count))
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let mock_response = self.next_response();
        let call_count = self.call_count();
        let chunks = Self::build_stream_chunks(&mock_response, call_count);

        // Convert to stream of RawStreamingChoice
        let stream: BoxStream<
            'static,
            Result<RawStreamingChoice<MockStreamingResponseData>, CompletionError>,
        > = stream::iter(chunks.into_iter().map(Ok)).boxed();

        Ok(StreamingCompletionResponse::stream(Box::pin(stream)))
    }
}

// ============================================================================
// Test Context Infrastructure
// ============================================================================

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::{mpsc, oneshot, RwLock};

use qbit_context::context_manager::ContextTrimConfig;
use qbit_context::token_budget::TokenBudgetConfig;
use qbit_context::{CompactionState, ContextManager};
use qbit_core::events::AiEvent;
use qbit_core::hitl::ApprovalDecision;
use qbit_hitl::ApprovalRecorder;
use qbit_llm_providers::LlmClient;
use qbit_loop_detection::LoopDetector;
use qbit_planner::PlanManager;
use qbit_sub_agents::SubAgentRegistry;
use qbit_tool_policy::{ToolPolicy, ToolPolicyConfig, ToolPolicyManager};
use qbit_tools::ToolRegistry;

use crate::agent_mode::AgentMode;
use crate::agentic_loop::{AgenticLoopContext, LoopCaptureContext};
use crate::tool_definitions::ToolConfig;

// ============================================================================
// Mock Runtime for Testing
// ============================================================================

use async_trait::async_trait;
use qbit_core::runtime::{ApprovalResult, QbitRuntime, RuntimeError, RuntimeEvent};
use std::any::Any;

/// A mock runtime for testing HITL approval flows.
#[derive(Debug)]
pub struct MockRuntime {
    auto_approve: bool,
    interactive: bool,
}

impl MockRuntime {
    /// Create a new mock runtime.
    pub fn new() -> Self {
        Self {
            auto_approve: false,
            interactive: true,
        }
    }

    /// Create a mock runtime with auto-approve enabled.
    pub fn with_auto_approve() -> Self {
        Self {
            auto_approve: true,
            interactive: true,
        }
    }

    /// Set whether auto-approve is enabled.
    pub fn set_auto_approve(&mut self, auto_approve: bool) {
        self.auto_approve = auto_approve;
    }
}

impl Default for MockRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl QbitRuntime for MockRuntime {
    fn emit(&self, _event: RuntimeEvent) -> Result<(), RuntimeError> {
        Ok(())
    }

    async fn request_approval(
        &self,
        _request_id: String,
        _tool_name: String,
        _args: serde_json::Value,
        _risk_level: String,
    ) -> Result<ApprovalResult, RuntimeError> {
        // In tests, we control approval via other mechanisms
        // Timeout of 0 indicates immediate timeout for testing
        Err(RuntimeError::ApprovalTimeout(0))
    }

    fn is_interactive(&self) -> bool {
        self.interactive
    }

    fn auto_approve(&self) -> bool {
        self.auto_approve
    }

    async fn shutdown(&self) -> Result<(), RuntimeError> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Builder for creating test contexts for the agentic loop.
pub struct TestContextBuilder {
    workspace: PathBuf,
    agent_mode: AgentMode,
    runtime: Option<Arc<dyn QbitRuntime>>,
    denied_tools: Vec<String>,
    allowed_tools: Vec<String>,
}

impl Default for TestContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestContextBuilder {
    /// Create a new test context builder with default settings.
    pub fn new() -> Self {
        Self {
            workspace: PathBuf::from("/tmp/qbit-test"),
            agent_mode: AgentMode::default(),
            runtime: None,
            denied_tools: vec![],
            allowed_tools: vec![],
        }
    }

    /// Set the workspace path.
    pub fn workspace(mut self, path: impl Into<PathBuf>) -> Self {
        self.workspace = path.into();
        self
    }

    /// Set the agent mode.
    pub fn agent_mode(mut self, mode: AgentMode) -> Self {
        self.agent_mode = mode;
        self
    }

    /// Set a runtime for testing.
    pub fn runtime(mut self, runtime: Arc<dyn QbitRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    /// Add a tool that should be denied by policy.
    pub fn deny_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.denied_tools.push(tool_name.into());
        self
    }

    /// Add a tool that should be allowed by policy (bypasses HITL).
    pub fn allow_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.allowed_tools.push(tool_name.into());
        self
    }

    /// Build the test context with all required dependencies.
    pub async fn build(self) -> TestContext {
        // Create temp directory for test data
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let storage_dir = temp_dir.path().to_path_buf();

        // Use the temp dir as the workspace (unless explicitly set)
        let workspace_path = if self.workspace.as_path() == std::path::Path::new("/tmp/qbit-test") {
            temp_dir.path().to_path_buf()
        } else {
            self.workspace.clone()
        };

        // Create all required components
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace_path.clone()).await));
        let sub_agent_registry = Arc::new(RwLock::new(SubAgentRegistry::new()));
        let approval_recorder = Arc::new(ApprovalRecorder::new(storage_dir.clone()).await);
        let pending_approvals = Arc::new(RwLock::new(HashMap::new()));

        // Create tool policy config with custom policies
        let mut policy_config = ToolPolicyConfig::default();
        for tool in &self.denied_tools {
            policy_config
                .policies
                .insert(tool.clone(), ToolPolicy::Deny);
        }
        for tool in &self.allowed_tools {
            policy_config
                .policies
                .insert(tool.clone(), ToolPolicy::Allow);
        }
        let tool_policy_manager = Arc::new(ToolPolicyManager::with_config(
            policy_config,
            workspace_path.join(".qbit").join("tool-policy.json"),
        ));

        let context_manager = Arc::new(ContextManager::new(
            TokenBudgetConfig::default(),
            ContextTrimConfig::default(),
        ));
        let compaction_state = Arc::new(RwLock::new(CompactionState::new()));
        let loop_detector = Arc::new(RwLock::new(LoopDetector::with_defaults()));
        let workspace = Arc::new(RwLock::new(workspace_path));
        let agent_mode = Arc::new(RwLock::new(self.agent_mode));
        let plan_manager = Arc::new(PlanManager::new());
        let tool_config = ToolConfig::default();

        TestContext {
            event_tx,
            event_rx,
            tool_registry,
            sub_agent_registry,
            approval_recorder,
            pending_approvals,
            tool_policy_manager,
            context_manager,
            compaction_state,
            loop_detector,
            workspace,
            agent_mode,
            plan_manager,
            tool_config,
            runtime: self.runtime,
            _temp_dir: temp_dir,
        }
    }
}

/// Test context holding all dependencies needed for agentic loop tests.
pub struct TestContext {
    pub event_tx: mpsc::UnboundedSender<AiEvent>,
    pub event_rx: mpsc::UnboundedReceiver<AiEvent>,
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    pub sub_agent_registry: Arc<RwLock<SubAgentRegistry>>,
    pub approval_recorder: Arc<ApprovalRecorder>,
    pub pending_approvals: Arc<RwLock<HashMap<String, oneshot::Sender<ApprovalDecision>>>>,
    pub tool_policy_manager: Arc<ToolPolicyManager>,
    pub context_manager: Arc<ContextManager>,
    pub compaction_state: Arc<RwLock<CompactionState>>,
    pub loop_detector: Arc<RwLock<LoopDetector>>,
    pub workspace: Arc<RwLock<PathBuf>>,
    pub agent_mode: Arc<RwLock<AgentMode>>,
    pub plan_manager: Arc<PlanManager>,
    pub tool_config: ToolConfig,
    /// Optional runtime for testing auto-approve flag
    pub runtime: Option<Arc<dyn QbitRuntime>>,
    // Keep temp dir alive for the duration of the test
    _temp_dir: tempfile::TempDir,
}

impl TestContext {
    /// Create an AgenticLoopContext from this test context.
    ///
    /// Note: The `client` field in AgenticLoopContext is required but we need
    /// to provide one externally since LlmClient is an enum without a default variant.
    pub fn as_agentic_context_with_client<'a>(
        &'a self,
        client: &'a Arc<RwLock<LlmClient>>,
    ) -> AgenticLoopContext<'a> {
        AgenticLoopContext {
            event_tx: &self.event_tx,
            tool_registry: &self.tool_registry,
            sub_agent_registry: &self.sub_agent_registry,
            indexer_state: None,
            workspace: &self.workspace,
            client,
            approval_recorder: &self.approval_recorder,
            pending_approvals: &self.pending_approvals,
            tool_policy_manager: &self.tool_policy_manager,
            context_manager: &self.context_manager,
            compaction_state: &self.compaction_state,
            loop_detector: &self.loop_detector,
            tool_config: &self.tool_config,
            sidecar_state: None,
            runtime: self.runtime.as_ref(),
            agent_mode: &self.agent_mode,
            plan_manager: &self.plan_manager,
            provider_name: "mock",
            model_name: "mock-model",
            openai_web_search_config: None,
            openai_reasoning_effort: None,
            model_factory: None,
            session_id: None,
            transcript_writer: None,
            transcript_base_dir: None,
            additional_tool_definitions: vec![],
            custom_tool_executor: None,
            coordinator: None, // Tests use legacy path
        }
    }

    /// Collect all events that have been emitted.
    pub fn collect_events(&mut self) -> Vec<AiEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        events
    }

    /// Create a LoopCaptureContext for testing.
    pub fn create_capture_context(&self) -> LoopCaptureContext {
        LoopCaptureContext::new(None)
    }

    /// Get workspace path.
    pub async fn workspace_path(&self) -> PathBuf {
        self.workspace.read().await.clone()
    }

    /// Find events of a specific type.
    pub fn find_events<F>(&mut self, predicate: F) -> Vec<AiEvent>
    where
        F: Fn(&AiEvent) -> bool,
    {
        self.collect_events()
            .into_iter()
            .filter(predicate)
            .collect()
    }

    /// Check if any event matches the predicate.
    pub fn has_event<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(&AiEvent) -> bool,
    {
        self.collect_events().iter().any(predicate)
    }

    /// Add a tool to the always-approve list in the approval recorder.
    pub async fn always_approve_tool(&self, tool_name: &str) {
        let _ = self.approval_recorder.add_always_allow(tool_name).await;
    }

    /// Record a manual approval for a tool (to test learned patterns).
    pub async fn record_tool_approval(&self, tool_name: &str, approved: bool) {
        let _ = self
            .approval_recorder
            .record_approval(tool_name, approved, None, false)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::completion::CompletionModel;
    use rig::streaming::StreamedAssistantContent;

    #[test]
    fn test_mock_response_text() {
        let response = MockResponse::text("Hello");
        assert_eq!(response.text, Some("Hello".to_string()));
        assert!(response.tool_calls.is_empty());
        assert!(response.thinking.is_none());
    }

    #[test]
    fn test_mock_response_tool_call() {
        let response = MockResponse::tool_call("read_file", serde_json::json!({"path": "/test"}));
        assert!(response.text.is_none());
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "read_file");
    }

    #[test]
    fn test_mock_response_with_thinking() {
        let response = MockResponse::text("Response").with_thinking("Thinking about this...");
        assert_eq!(response.text, Some("Response".to_string()));
        assert_eq!(
            response.thinking,
            Some("Thinking about this...".to_string())
        );
    }

    #[test]
    fn test_mock_model_response_sequence() {
        let model = MockCompletionModel::new(vec![
            MockResponse::text("First"),
            MockResponse::text("Second"),
            MockResponse::text("Third"),
        ]);

        assert_eq!(model.call_count(), 0);

        let r1 = model.next_response();
        assert_eq!(r1.text, Some("First".to_string()));
        assert_eq!(model.call_count(), 1);

        let r2 = model.next_response();
        assert_eq!(r2.text, Some("Second".to_string()));
        assert_eq!(model.call_count(), 2);

        let r3 = model.next_response();
        assert_eq!(r3.text, Some("Third".to_string()));
        assert_eq!(model.call_count(), 3);

        // Exhausted - returns empty string
        let r4 = model.next_response();
        assert_eq!(r4.text, Some("".to_string()));
    }

    #[test]
    fn test_mock_model_reset() {
        let model = MockCompletionModel::new(vec![
            MockResponse::text("First"),
            MockResponse::text("Second"),
        ]);

        let _ = model.next_response();
        let _ = model.next_response();
        assert_eq!(model.call_count(), 2);

        model.reset();
        assert_eq!(model.call_count(), 0);

        let r1 = model.next_response();
        assert_eq!(r1.text, Some("First".to_string()));
    }

    #[tokio::test]
    async fn test_mock_model_completion() {
        let model = MockCompletionModel::with_text("Test response");
        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(rig::completion::Message::User {
                content: OneOrMany::one(rig::message::UserContent::Text(Text {
                    text: "Hello".to_string(),
                })),
            }),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let response = model.completion(request).await.unwrap();
        assert!(matches!(
            response.choice.iter().next().unwrap(),
            AssistantContent::Text(Text { text }) if text == "Test response"
        ));
    }

    #[tokio::test]
    async fn test_mock_model_stream() {
        let model = MockCompletionModel::with_text("Streamed response");
        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(rig::completion::Message::User {
                content: OneOrMany::one(rig::message::UserContent::Text(Text {
                    text: "Hello".to_string(),
                })),
            }),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let mut stream = model.stream(request).await.unwrap();
        let mut found_text = false;
        let mut found_final = false;

        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                StreamedAssistantContent::Text(t) => {
                    assert_eq!(t.text, "Streamed response");
                    found_text = true;
                }
                StreamedAssistantContent::Final(_) => {
                    found_final = true;
                }
                _ => {}
            }
        }

        assert!(found_text);
        assert!(found_final);
    }

    #[tokio::test]
    async fn test_mock_model_tool_call_stream() {
        let model = MockCompletionModel::new(vec![MockResponse::tool_call(
            "read_file",
            serde_json::json!({"path": "/test.txt"}),
        )]);

        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(rig::completion::Message::User {
                content: OneOrMany::one(rig::message::UserContent::Text(Text {
                    text: "Read the file".to_string(),
                })),
            }),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let mut stream = model.stream(request).await.unwrap();
        let mut found_tool_call = false;

        while let Some(chunk) = stream.next().await {
            if let StreamedAssistantContent::ToolCall(tc) = chunk.unwrap() {
                assert_eq!(tc.function.name, "read_file");
                found_tool_call = true;
            }
        }

        assert!(found_tool_call);
    }

    #[tokio::test]
    async fn test_mock_model_with_thinking() {
        let model = MockCompletionModel::new(vec![MockResponse::text_with_thinking(
            "Final answer",
            "Let me think about this...",
        )]);

        let request = CompletionRequest {
            preamble: None,
            chat_history: OneOrMany::one(rig::completion::Message::User {
                content: OneOrMany::one(rig::message::UserContent::Text(Text {
                    text: "What is 2+2?".to_string(),
                })),
            }),
            documents: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
            tool_choice: None,
            additional_params: None,
        };

        let mut stream = model.stream(request).await.unwrap();
        let mut found_reasoning = false;
        let mut found_text = false;

        while let Some(chunk) = stream.next().await {
            match chunk.unwrap() {
                StreamedAssistantContent::Reasoning(r) => {
                    assert_eq!(r.reasoning, vec!["Let me think about this...".to_string()]);
                    found_reasoning = true;
                }
                StreamedAssistantContent::Text(t) => {
                    assert_eq!(t.text, "Final answer");
                    found_text = true;
                }
                _ => {}
            }
        }

        assert!(found_reasoning);
        assert!(found_text);
    }

    // ========================================================================
    // Test Context Builder Tests
    // ========================================================================

    #[tokio::test]
    async fn test_context_builder_creates_valid_context() {
        let test_ctx = TestContextBuilder::new().build().await;

        // Verify all components are initialized
        assert!(test_ctx
            .event_tx
            .send(AiEvent::Started {
                turn_id: "test".to_string()
            })
            .is_ok());

        // Collect the event
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        assert_eq!(events.len(), 1);

        // Verify it's the event we sent
        match &events[0] {
            AiEvent::Started { turn_id } => assert_eq!(turn_id, "test"),
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_context_builder_with_planning_mode() {
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::Planning)
            .build()
            .await;

        // Verify agent mode is set correctly
        let mode = test_ctx.agent_mode.read().await;
        assert!(mode.is_planning());
    }

    #[tokio::test]
    async fn test_context_builder_with_auto_approve_mode() {
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Verify agent mode is set correctly
        let mode = test_ctx.agent_mode.read().await;
        assert!(mode.is_auto_approve());
    }

    // ========================================================================
    // HITL Approval Flow Tests (Phase 0.2)
    // ========================================================================

    use crate::agentic_loop::execute_with_hitl_generic;
    use qbit_sub_agents::SubAgentContext;

    /// Helper to create a minimal SubAgentContext for tests.
    fn test_sub_agent_context() -> SubAgentContext {
        SubAgentContext {
            original_request: "Test request".to_string(),
            conversation_summary: None,
            variables: std::collections::HashMap::new(),
            depth: 0,
        }
    }

    /// Helper to create a minimal LlmClient for tests.
    /// Uses a mock client since we're testing HITL logic, not LLM calls.
    fn test_llm_client() -> Arc<RwLock<LlmClient>> {
        Arc::new(RwLock::new(LlmClient::Mock))
    }

    #[tokio::test]
    async fn test_hitl_planning_mode_blocks_write_tools() {
        // In planning mode, write tools (like write_file) should be blocked
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::Planning)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Try to execute a write tool (should be blocked in planning mode)
        let result = execute_with_hitl_generic(
            "write_file",
            &serde_json::json!({"path": "test.txt", "content": "hello"}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should fail with planning_mode_denied
        assert!(!result.success);
        assert!(result.value.get("planning_mode_denied").is_some());
        assert!(result.value["error"]
            .as_str()
            .unwrap()
            .contains("not allowed in planning mode"));
    }

    #[tokio::test]
    async fn test_hitl_planning_mode_allows_read_tools() {
        // In planning mode, read tools (like read_file) should be allowed
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::Planning)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Create a file in the workspace first
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("test.txt"), "hello world").unwrap();

        // Try to execute a read tool (should be allowed in planning mode)
        // read_file is in ALLOW_TOOLS so should bypass HITL entirely
        let result = execute_with_hitl_generic(
            "read_file",
            &serde_json::json!({"path": "test.txt"}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should succeed (auto-approved by policy)
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_hitl_denied_by_policy() {
        // Tools explicitly denied by policy should fail immediately
        let test_ctx = TestContextBuilder::new()
            .deny_tool("custom_dangerous_tool")
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        let result = execute_with_hitl_generic(
            "custom_dangerous_tool",
            &serde_json::json!({}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should fail with denied_by_policy
        assert!(!result.success);
        assert!(result.value.get("denied_by_policy").is_some());
    }

    #[tokio::test]
    async fn test_hitl_allowed_by_policy_bypasses_approval() {
        // Tools allowed by policy should bypass HITL entirely
        let test_ctx = TestContextBuilder::new()
            .allow_tool("custom_safe_tool")
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // The tool doesn't exist in the registry, but we can check that
        // it attempts to execute (and fails at execution, not approval)
        let _result = execute_with_hitl_generic(
            "custom_safe_tool",
            &serde_json::json!({}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should get to execution (and fail there since tool doesn't exist)
        // But importantly, no approval request should be emitted
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_approval_request = events
            .iter()
            .any(|e| matches!(e, AiEvent::ToolApprovalRequest { .. }));
        assert!(
            !has_approval_request,
            "Should not have approval request for allowed tool"
        );

        // Should have auto-approved event
        let has_auto_approved = events.iter().any(
            |e| matches!(e, AiEvent::ToolAutoApproved { reason, .. } if reason.contains("policy")),
        );
        assert!(has_auto_approved, "Should have auto-approved event");
    }

    #[tokio::test]
    async fn test_hitl_auto_approve_from_learned_patterns() {
        // Tools that have been approved consistently should be auto-approved
        let test_ctx = TestContextBuilder::new().build().await;

        // Add the tool to the always-approve list
        test_ctx.always_approve_tool("learned_tool").await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        let _result = execute_with_hitl_generic(
            "learned_tool",
            &serde_json::json!({}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should have auto-approved event for learned patterns
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_auto_approved = events.iter().any(|e| {
            matches!(e, AiEvent::ToolAutoApproved { reason, .. }
                if reason.contains("learned patterns") || reason.contains("always-allow"))
        });
        assert!(
            has_auto_approved,
            "Should have auto-approved event for learned tool"
        );
    }

    #[tokio::test]
    async fn test_hitl_auto_approve_from_agent_mode() {
        // AgentMode::AutoApprove should auto-approve all tools
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        let _result = execute_with_hitl_generic(
            "some_random_tool",
            &serde_json::json!({}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should have auto-approved event via agent mode
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_auto_approved = events.iter().any(|e| {
            matches!(e, AiEvent::ToolAutoApproved { reason, .. }
                if reason.contains("agent mode"))
        });
        assert!(
            has_auto_approved,
            "Should have auto-approved event via agent mode"
        );
    }

    #[tokio::test]
    async fn test_hitl_auto_approve_from_runtime_flag() {
        // Runtime with auto_approve=true should auto-approve all tools
        let test_ctx = TestContextBuilder::new()
            .runtime(Arc::new(MockRuntime::with_auto_approve()))
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        let _result = execute_with_hitl_generic(
            "some_random_tool",
            &serde_json::json!({}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should have auto-approved event via runtime flag
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_auto_approved = events.iter().any(|e| {
            matches!(e, AiEvent::ToolAutoApproved { reason, .. }
                if reason.contains("--auto-approve"))
        });
        assert!(
            has_auto_approved,
            "Should have auto-approved event via runtime flag"
        );
    }

    #[tokio::test]
    async fn test_hitl_constraint_violation_denied() {
        // Tools that violate constraints should be denied
        // The default policy has blocked hosts for web_fetch
        let test_ctx = TestContextBuilder::new().build().await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Try to fetch localhost (blocked by default constraints)
        let result = execute_with_hitl_generic(
            "web_fetch",
            &serde_json::json!({"url": "http://localhost:8080/api"}),
            "test-tool-id",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should fail with constraint_violated
        assert!(!result.success);
        assert!(result.value.get("constraint_violated").is_some());
    }

    #[tokio::test]
    async fn test_hitl_approval_request_emitted() {
        // When approval is needed, a ToolApprovalRequest event should be emitted
        let test_ctx = TestContextBuilder::new().build().await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Use a tokio::select with a short timeout to avoid hanging
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            execute_with_hitl_generic(
                "edit_file", // A prompt tool that requires approval
                &serde_json::json!({"path": "test.txt", "edits": []}),
                "test-tool-id",
                &ctx,
                &mut capture_ctx,
                &model,
                &sub_ctx,
            ),
        )
        .await;

        // The call should timeout (because we don't respond to the approval request)
        // But we should have emitted an approval request event
        assert!(result.is_err(), "Should timeout waiting for approval");

        // Check for approval request event
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_approval_request = events.iter().any(|e| {
            matches!(e, AiEvent::ToolApprovalRequest { tool_name, .. }
                if tool_name == "edit_file")
        });
        assert!(
            has_approval_request,
            "Should have emitted ToolApprovalRequest event"
        );
    }

    #[tokio::test]
    async fn test_hitl_approval_timeout() {
        // When approval times out, the tool should fail with timeout error
        // Note: We use a custom short timeout for testing
        let test_ctx = TestContextBuilder::new().build().await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let _capture_ctx = test_ctx.create_capture_context();
        let _model = MockCompletionModel::with_text("Done");
        let _sub_ctx = test_sub_agent_context();

        // The default APPROVAL_TIMEOUT_SECS is 300 (5 minutes), which is too long for tests.
        // We'll test the timeout behavior by using tokio::select with a short timeout
        // and verifying the pending_approvals state.

        // Start the approval request in a task
        let pending_approvals = ctx.pending_approvals.clone();
        let event_tx = ctx.event_tx.clone();

        let tool_name = "edit_file";
        let tool_id = "timeout-test-id";

        // Manually simulate what execute_with_hitl_generic does for approval request
        // (This avoids the 300 second timeout in tests)
        let _ = event_tx.send(AiEvent::ToolApprovalRequest {
            request_id: tool_id.to_string(),
            tool_name: tool_name.to_string(),
            args: serde_json::json!({}),
            stats: None,
            risk_level: qbit_core::hitl::RiskLevel::Medium,
            can_learn: true,
            suggestion: None,
            source: qbit_core::events::ToolSource::Main,
        });

        // Create and store the oneshot sender
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = pending_approvals.write().await;
            pending.insert(tool_id.to_string(), tx);
        }

        // Verify the pending approval is registered
        {
            let pending = pending_approvals.read().await;
            assert!(
                pending.contains_key(tool_id),
                "Should have pending approval"
            );
        }

        // Wait a very short time (simulating timeout behavior)
        let result = tokio::time::timeout(std::time::Duration::from_millis(10), rx).await;

        // Should timeout
        assert!(result.is_err(), "Should timeout waiting for approval");

        // Clean up (as the timeout handler would)
        {
            let mut pending = pending_approvals.write().await;
            pending.remove(tool_id);
        }

        // Verify cleanup
        {
            let pending = pending_approvals.read().await;
            assert!(
                !pending.contains_key(tool_id),
                "Should have cleaned up pending approval after timeout"
            );
        }
    }

    // ========================================================================
    // Tool Routing Tests (Phase 0.3)
    // ========================================================================

    #[tokio::test]
    async fn test_tool_routing_to_file_operations() {
        // Verify file tools (read_file, write_file, edit_file) route correctly
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Create a test file in the workspace
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("routing_test.txt"), "test content").unwrap();

        // Test read_file routing
        let result = execute_with_hitl_generic(
            "read_file",
            &serde_json::json!({"path": "routing_test.txt"}),
            "test-tool-id-read",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should succeed and contain the content
        assert!(result.success, "read_file should succeed");
        assert!(
            result.value.get("content").is_some() || result.value.get("error").is_none(),
            "read_file should return content or not error"
        );

        // Test write_file routing (routes through tool registry)
        let write_result = execute_with_hitl_generic(
            "write_file",
            &serde_json::json!({
                "path": "routing_test_write.txt",
                "content": "new content"
            }),
            "test-tool-id-write",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should succeed (file operations are routed through registry)
        assert!(
            write_result.success,
            "write_file should succeed: {:?}",
            write_result.value
        );

        // Test edit_file routing (requires existing file with content to edit)
        std::fs::write(ws.join("edit_test.txt"), "line 1\nline 2\nline 3").unwrap();
        let edit_result = execute_with_hitl_generic(
            "edit_file",
            &serde_json::json!({
                "path": "edit_test.txt",
                "old_text": "line 2",
                "new_text": "modified line 2"
            }),
            "test-tool-id-edit",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // edit_file routes through the registry
        // Success depends on actual edit implementation
        assert!(
            edit_result.value.get("error").is_none() || edit_result.success,
            "edit_file should route correctly: {:?}",
            edit_result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_to_shell_execution() {
        // Verify run_pty_cmd and run_command route to shell executor
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Test run_pty_cmd routing
        let result = execute_with_hitl_generic(
            "run_pty_cmd",
            &serde_json::json!({"command": "echo hello"}),
            "test-tool-id-pty",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should route to shell execution (may or may not succeed depending on environment)
        // Key thing is that it routes correctly and doesn't return "unknown tool"
        assert!(
            !result.value.to_string().contains("unknown tool"),
            "run_pty_cmd should be recognized: {:?}",
            result.value
        );

        // Test run_command routing (alias for run_pty_cmd)
        let cmd_result = execute_with_hitl_generic(
            "run_command",
            &serde_json::json!({"command": "echo world"}),
            "test-tool-id-cmd",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // run_command should be mapped to run_pty_cmd internally
        assert!(
            !cmd_result.value.to_string().contains("unknown tool"),
            "run_command should be recognized (mapped to run_pty_cmd): {:?}",
            cmd_result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_unknown_tool_returns_error() {
        // Verify unknown tools fail gracefully
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Try to execute a tool that doesn't exist
        let result = execute_with_hitl_generic(
            "completely_nonexistent_tool_xyz123",
            &serde_json::json!({"some_arg": "value"}),
            "test-tool-id-unknown",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should fail with an error (not panic)
        assert!(
            !result.success,
            "Unknown tool should not succeed: {:?}",
            result.value
        );
        assert!(
            result.value.get("error").is_some(),
            "Unknown tool should return error field: {:?}",
            result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_sub_agent_tool() {
        // Verify sub-agent tools are recognized (execute_sub_agent pattern)
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Test sub-agent tool routing (sub_agent_<id> pattern)
        // This will fail because the sub-agent doesn't exist, but it should
        // be recognized as a sub-agent tool and routed appropriately
        let result = execute_with_hitl_generic(
            "sub_agent_test_agent",
            &serde_json::json!({"task": "test task", "context": "test context"}),
            "test-tool-id-subagent",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should be recognized as sub-agent tool and return "not found" error
        // (not "unknown tool" error)
        assert!(!result.success, "Non-existent sub-agent should not succeed");
        let error_str = result.value.to_string();
        assert!(
            error_str.contains("not found") || error_str.contains("Sub-agent"),
            "Should indicate sub-agent not found, got: {:?}",
            result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_web_tools() {
        // Verify web_fetch and web_search route correctly
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Test web_fetch routing
        // Note: This will actually try to fetch, so use a non-blocked URL
        // The constraint violation test already covers localhost blocking
        let result = execute_with_hitl_generic(
            "web_fetch",
            &serde_json::json!({"url": "https://example.com", "prompt": "summarize"}),
            "test-tool-id-webfetch",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // web_fetch should be routed correctly (success depends on network)
        // Key thing is it's recognized and not "unknown tool"
        let error_str = result.value.to_string().to_lowercase();
        assert!(
            !error_str.contains("unknown tool"),
            "web_fetch should be recognized: {:?}",
            result.value
        );

        // Test web_search routing (requires Tavily state, which is None in test)
        let search_result = execute_with_hitl_generic(
            "web_search",
            &serde_json::json!({"query": "test query"}),
            "test-tool-id-websearch",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // web_search routes through Tavily handler
        // Without Tavily configured, should fail with "not available" or similar
        let search_error_str = search_result.value.to_string().to_lowercase();
        assert!(
            search_error_str.contains("not available")
                || search_error_str.contains("tavily")
                || search_error_str.contains("not configured")
                || !search_result.success,
            "web_search should be routed to Tavily handler: {:?}",
            search_result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_indexer_tools() {
        // Verify indexer_search_code and similar tools route correctly
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Test indexer_search_code routing
        let result = execute_with_hitl_generic(
            "indexer_search_code",
            &serde_json::json!({"pattern": "test.*pattern"}),
            "test-tool-id-indexer-search",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should be routed to indexer tool handler
        // Without indexer state, should return appropriate error (not "unknown tool")
        let error_str = result.value.to_string().to_lowercase();
        assert!(
            error_str.contains("indexer")
                || error_str.contains("not available")
                || error_str.contains("not initialized")
                || !error_str.contains("unknown tool"),
            "indexer_search_code should be routed to indexer handler: {:?}",
            result.value
        );

        // Test indexer_search_files routing
        let files_result = execute_with_hitl_generic(
            "indexer_search_files",
            &serde_json::json!({"pattern": "*.rs"}),
            "test-tool-id-indexer-files",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should also be routed to indexer handler (starts with "indexer_")
        let files_error_str = files_result.value.to_string().to_lowercase();
        assert!(
            files_error_str.contains("indexer")
                || files_error_str.contains("not available")
                || files_error_str.contains("not initialized")
                || !files_error_str.contains("unknown tool"),
            "indexer_search_files should be routed to indexer handler: {:?}",
            files_result.value
        );

        // Test indexer_analyze_file routing
        let analyze_result = execute_with_hitl_generic(
            "indexer_analyze_file",
            &serde_json::json!({"file_path": "test.rs"}),
            "test-tool-id-indexer-analyze",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // Should be recognized as indexer tool
        let analyze_error_str = analyze_result.value.to_string().to_lowercase();
        assert!(
            analyze_error_str.contains("indexer")
                || analyze_error_str.contains("not available")
                || analyze_error_str.contains("not initialized")
                || !analyze_error_str.contains("unknown tool"),
            "indexer_analyze_file should be routed to indexer handler: {:?}",
            analyze_result.value
        );
    }

    #[tokio::test]
    async fn test_tool_routing_planner_tools() {
        // Verify update_plan routes correctly
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let mut capture_ctx = test_ctx.create_capture_context();
        let model = MockCompletionModel::with_text("Done");
        let sub_ctx = test_sub_agent_context();

        // Test update_plan routing with valid plan structure
        // The plan structure requires "step" (not "task") field
        let result = execute_with_hitl_generic(
            "update_plan",
            &serde_json::json!({
                "plan": [
                    {"step": "Step 1", "status": "pending"},
                    {"step": "Step 2", "status": "pending"}
                ]
            }),
            "test-tool-id-plan",
            &ctx,
            &mut capture_ctx,
            &model,
            &sub_ctx,
        )
        .await
        .unwrap();

        // update_plan should be routed to plan handler
        // The plan manager is initialized in test context, so this should work
        assert!(
            result.success || result.value.get("plan").is_some(),
            "update_plan should be routed correctly and succeed: {:?}",
            result.value
        );

        // Verify that the plan was actually updated by checking events
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_plan_event = events
            .iter()
            .any(|e| matches!(e, AiEvent::PlanUpdated { .. }));

        // If successful, should have emitted a plan event
        if result.success {
            assert!(
                has_plan_event,
                "Successful update_plan should emit PlanUpdated event"
            );
        }
    }

    // ========================================================================
    // Agentic Loop Integration Tests (Phase 0.4)
    // ========================================================================
    //
    // These tests verify the higher-level agentic loop behavior, focusing on
    // scenarios not covered by the behavioral equivalence tests in Phase 0.6.

    use crate::agentic_loop::run_agentic_loop_generic;
    use rig::completion::Message;
    use rig::message::UserContent;
    use rig::one_or_many::OneOrMany;

    /// Helper to create initial chat history with a user message.
    fn initial_history_phase04(user_message_text: &str) -> Vec<Message> {
        vec![Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: user_message_text.to_string(),
            })),
        }]
    }

    #[tokio::test]
    async fn test_agentic_loop_simple_text_response() {
        // Test: Model returns text only, loop completes with that response
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let model = MockCompletionModel::with_text("Hello! This is a simple text response.");
        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Say hello");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should complete successfully");
        let (response, _reasoning, final_history, usage) = result.unwrap();

        // Verify the response text
        assert_eq!(response, "Hello! This is a simple text response.");

        // Verify token usage was tracked
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert!(usage.input_tokens > 0 || usage.output_tokens > 0);

        // Verify history contains the original user message
        assert!(!final_history.is_empty());

        // Verify model was called exactly once
        assert_eq!(model.call_count(), 1);
    }

    #[tokio::test]
    async fn test_agentic_loop_single_tool_call() {
        // Test: Model returns one tool call, executes it, then returns text
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Create a test file in the workspace
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("test.txt"), "Hello from test file").unwrap();

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model: first returns tool call, then returns text response
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call("read_file", serde_json::json!({"path": "test.txt"})),
            MockResponse::text("I read the file. It contains: Hello from test file"),
        ]);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Read the test.txt file");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should complete successfully");
        let (response, _reasoning, final_history, _usage) = result.unwrap();

        // Verify the final response
        assert!(response.contains("Hello from test file") || response.contains("I read the file"));

        // Verify model was called twice (tool call + final response)
        assert_eq!(model.call_count(), 2);

        // Verify history grew (user + assistant with tool + user with result + assistant final)
        assert!(final_history.len() >= 3);
    }

    #[tokio::test]
    async fn test_agentic_loop_multiple_tool_calls() {
        // Test: Model returns multiple tool calls in sequence (one per iteration)
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Create test files
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("file1.txt"), "Content of file 1").unwrap();
        std::fs::write(ws.join("file2.txt"), "Content of file 2").unwrap();

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model: calls two tools in sequence, then returns final response
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call("read_file", serde_json::json!({"path": "file1.txt"})),
            MockResponse::tool_call("read_file", serde_json::json!({"path": "file2.txt"})),
            MockResponse::text("I read both files successfully."),
        ]);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Read both file1.txt and file2.txt");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should complete successfully");
        let (response, _reasoning, _final_history, _usage) = result.unwrap();

        // Verify the final response
        assert!(response.contains("both files") || response.contains("successfully"));

        // Verify model was called three times
        assert_eq!(model.call_count(), 3);
    }

    #[tokio::test]
    async fn test_agentic_loop_tool_then_text() {
        // Test: Model calls tool, receives result, then returns text incorporating the result
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Create a file with specific content
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("data.json"), r#"{"key": "value123"}"#).unwrap();

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        let model = MockCompletionModel::with_tool_call_then_text(
            "read_file",
            serde_json::json!({"path": "data.json"}),
            "The file contains JSON with key='value123'.",
        );

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("What's in data.json?");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok());
        let (response, _reasoning, _final_history, _usage) = result.unwrap();

        // Verify the response incorporates the expected content
        assert!(response.contains("value123") || response.contains("JSON"));
    }

    #[tokio::test]
    async fn test_agentic_loop_max_iterations_reached() {
        // Test: Loop stops when max iterations are hit (MAX_TOOL_ITERATIONS = 100)
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Create file for read_file tool
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("endless.txt"), "keep reading me").unwrap();

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Create a model that always returns tool calls (would loop forever without limit)
        let mut responses: Vec<MockResponse> = Vec::new();
        for _ in 0..150 {
            responses.push(MockResponse::tool_call(
                "read_file",
                serde_json::json!({"path": "endless.txt"}),
            ));
        }
        let model = MockCompletionModel::new(responses);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Keep reading the file");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        // The loop should complete (not hang)
        assert!(
            result.is_ok(),
            "Loop should complete even when hitting max iterations"
        );

        // Collect events to verify max iterations event was emitted
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let has_max_iterations_error = events.iter().any(
            |e| matches!(e, AiEvent::Error { error_type, .. } if error_type == "max_iterations"),
        );
        assert!(
            has_max_iterations_error,
            "Should emit max_iterations error event"
        );
    }

    #[tokio::test]
    async fn test_agentic_loop_context_events_emitted() {
        // Test: Verify TextDelta events are emitted during streaming
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let model = MockCompletionModel::with_text("Event test response");
        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Test events");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;
        assert!(result.is_ok());

        // Collect all events
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();

        // Verify TextDelta events were emitted
        let text_delta_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::TextDelta { .. }))
            .collect();
        assert!(
            !text_delta_events.is_empty(),
            "Should have emitted TextDelta events"
        );

        // Verify the accumulated text matches
        let final_text_delta = text_delta_events.last();
        if let Some(AiEvent::TextDelta { accumulated, .. }) = final_text_delta {
            assert_eq!(accumulated, "Event test response");
        }
    }

    #[tokio::test]
    async fn test_agentic_loop_tool_error_handling() {
        // Test: Tool returns error, loop handles it gracefully and continues
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Don't create the file - read_file will fail
        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model tries to read non-existent file, then responds to the error
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call("read_file", serde_json::json!({"path": "nonexistent.txt"})),
            MockResponse::text("The file doesn't exist, I received an error."),
        ]);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Read nonexistent.txt");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        // Should complete successfully (error is passed back to LLM)
        assert!(result.is_ok(), "Loop should handle tool errors gracefully");
        let (response, _reasoning, _final_history, _usage) = result.unwrap();

        // The model should have received the error and responded
        assert!(response.contains("error") || response.contains("doesn't exist"));

        // Verify ToolResult event shows failure
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let tool_results: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::ToolResult { success: false, .. }))
            .collect();
        assert!(
            !tool_results.is_empty(),
            "Should have a failed tool result event"
        );
    }

    #[tokio::test]
    async fn test_agentic_loop_with_thinking() {
        // Test: Model returns thinking/reasoning content, properly handled
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model returns text with thinking
        let model = MockCompletionModel::new(vec![MockResponse::text_with_thinking(
            "The answer is 42.",
            "Let me think about this carefully... The question of life, the universe, and everything...",
        )]);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("What is the meaning of life?");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok());
        let (response, _reasoning, _final_history, _usage) = result.unwrap();

        // Verify the text response (not the thinking)
        assert_eq!(response, "The answer is 42.");

        // Verify Reasoning events were emitted
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let reasoning_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::Reasoning { .. }))
            .collect();
        assert!(
            !reasoning_events.is_empty(),
            "Should have emitted Reasoning events for thinking content"
        );
    }

    #[tokio::test]
    async fn test_agentic_loop_empty_response() {
        // Test: Model returns empty response, loop handles gracefully
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model returns empty text
        let model = MockCompletionModel::with_text("");

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Say nothing");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Loop should handle empty responses");
        let (response, _reasoning, _final_history, _usage) = result.unwrap();

        // Empty response is valid
        assert_eq!(response, "");
    }

    #[tokio::test]
    async fn test_agentic_loop_cancellation_via_timeout() {
        // Test: Cancellation behavior via external timeout
        // The agentic loop respects external cancellation via tokio timeout/select
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Create a model that returns a simple response
        let model = MockCompletionModel::with_text("This should complete quickly.");

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Quick test");

        // Run with a timeout to verify the loop can complete within reasonable time
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            run_agentic_loop_generic(
                &model,
                "You are a helpful assistant.",
                history,
                sub_ctx,
                &ctx,
            ),
        )
        .await;

        // Should complete within timeout
        assert!(result.is_ok(), "Loop should complete within timeout");
        let inner_result = result.unwrap();
        assert!(inner_result.is_ok(), "Loop result should be successful");
    }

    #[tokio::test]
    async fn test_agentic_loop_multiple_tool_calls_in_single_response() {
        // Test: Model returns multiple tool calls in a single response (parallel tool calling)
        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        // Create test files
        let ws = test_ctx.workspace_path().await;
        std::fs::write(ws.join("a.txt"), "Content A").unwrap();
        std::fs::write(ws.join("b.txt"), "Content B").unwrap();

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);

        // Model returns multiple tool calls at once
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_calls(vec![
                MockToolCall::new("read_file", serde_json::json!({"path": "a.txt"})),
                MockToolCall::new("read_file", serde_json::json!({"path": "b.txt"})),
            ]),
            MockResponse::text("I read both files: A and B."),
        ]);

        let sub_ctx = test_sub_agent_context();
        let history = initial_history_phase04("Read a.txt and b.txt simultaneously");

        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok());
        let (response, _reasoning, _final_history, _usage) = result.unwrap();
        assert!(response.contains("both files") || response.contains("A and B"));

        // Verify model was called twice (multi-tool + final)
        assert_eq!(model.call_count(), 2);

        // Verify we got two tool result events
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();
        let tool_results: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::ToolResult { .. }))
            .collect();
        assert_eq!(tool_results.len(), 2, "Should have two tool result events");
    }

    // ========================================================================
    // Behavioral Equivalence Tests (Phase 0.6)
    // ========================================================================
    //
    // These tests verify that the generic agentic loop functions produce the
    // same behavior as their specialized counterparts. This is critical for
    // the consolidation effort, as we will eventually deprecate the specialized
    // implementations in favor of the generic ones.
    //
    // Note: Uses imports from Phase 0.4 section above (run_agentic_loop_generic,
    // Message, UserContent)

    /// Helper to create a simple user message for testing.
    fn user_message(text: &str) -> Message {
        Message::User {
            content: OneOrMany::one(UserContent::Text(Text {
                text: text.to_string(),
            })),
        }
    }

    #[tokio::test]
    async fn test_behavioral_equivalence_text_response() {
        // Verify that the generic agentic loop produces the same text response
        // as the specialized version would.
        //
        // This test uses MockCompletionModel which returns predefined responses,
        // allowing us to verify that text streaming and accumulation work identically.

        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove) // Auto-approve to simplify testing
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let sub_ctx = test_sub_agent_context();

        // Create model that returns a simple text response
        let expected_text = "This is a test response from the model.";
        let model = MockCompletionModel::with_text(expected_text);

        // Run the generic agentic loop
        let initial_history = vec![user_message("Hello, how are you?")];
        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            initial_history.clone(),
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should succeed");
        let (response_text, _reasoning, final_history, token_usage) = result.unwrap();

        // Verify the response text matches expected
        assert_eq!(
            response_text, expected_text,
            "Response text should match expected"
        );

        // For text-only responses (no tool calls), history contains original messages
        // The final response is returned separately, not appended to history
        assert!(!final_history.is_empty(), "History should contain messages");

        // Verify token usage was tracked
        assert!(token_usage.is_some(), "Token usage should be tracked");
        let usage = token_usage.unwrap();
        assert!(usage.input_tokens > 0, "Input tokens should be non-zero");
        assert!(usage.output_tokens > 0, "Output tokens should be non-zero");

        // Verify correct events were emitted
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();

        // Should have TextDelta events
        let text_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::TextDelta { .. }))
            .collect();
        assert!(
            !text_events.is_empty(),
            "Should emit TextDelta events for streaming text"
        );
    }

    #[tokio::test]
    async fn test_behavioral_equivalence_tool_execution() {
        // Verify that tool routing and execution works identically in the
        // generic loop compared to the specialized version.
        //
        // This tests that:
        // 1. Tool calls are correctly parsed from model responses
        // 2. Tools are executed via the tool registry
        // 3. Tool results are correctly added to message history

        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove) // Auto-approve all tools
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let sub_ctx = test_sub_agent_context();

        // Create a file in the workspace to read
        let ws = test_ctx.workspace_path().await;
        let test_file = ws.join("test_file.txt");
        std::fs::write(&test_file, "Hello from test file!").unwrap();

        // Create model that:
        // 1. First returns a read_file tool call
        // 2. Then returns a text response summarizing the file
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call(
                "read_file",
                serde_json::json!({"path": test_file.to_string_lossy()}),
            ),
            MockResponse::text("I read the file and it says: Hello from test file!"),
        ]);

        // Run the generic agentic loop
        let initial_history = vec![user_message("Read the test file")];
        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            initial_history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should succeed");
        let (response_text, _reasoning, final_history, _) = result.unwrap();

        // Verify the final response contains expected text
        assert!(
            response_text.contains("Hello from test file"),
            "Response should reference file contents"
        );

        // Verify history contains tool call and result
        let has_tool_call = final_history.iter().any(|msg| {
            if let Message::Assistant { content, .. } = msg {
                content
                    .iter()
                    .any(|c| matches!(c, AssistantContent::ToolCall(_)))
            } else {
                false
            }
        });
        assert!(has_tool_call, "History should contain tool call");

        let has_tool_result = final_history.iter().any(|msg| {
            if let Message::User { content } = msg {
                content
                    .iter()
                    .any(|c| matches!(c, UserContent::ToolResult(_)))
            } else {
                false
            }
        });
        assert!(has_tool_result, "History should contain tool result");

        // Verify tool-related events were emitted
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();

        // Note: ToolRequest is only captured to sidecar, not emitted to frontend
        // Check for ToolAutoApproved event (emitted via emit_event for policy Allow)
        // Since read_file is in ALLOW_TOOLS, it gets auto-approved by policy (since we're in auto-approve mode)
        let has_auto_approved = events.iter().any(|e| {
            matches!(e, AiEvent::ToolAutoApproved { tool_name, .. } if tool_name == "read_file")
        });
        assert!(has_auto_approved, "Should emit ToolAutoApproved event");

        // Should have ToolResult event
        let has_tool_result_event = events.iter().any(|e| {
            matches!(e, AiEvent::ToolResult { tool_name, success, .. } if tool_name == "read_file" && *success)
        });
        assert!(
            has_tool_result_event,
            "Should emit successful ToolResult event"
        );
    }

    #[tokio::test]
    async fn test_behavioral_equivalence_event_sequence() {
        // Verify that the sequence of events emitted by the generic loop
        // matches the expected behavior pattern.
        //
        // Event sequence for a tool call should be:
        // 1. ToolRequest - when tool call is detected
        // 2. ToolAutoApproved/ToolApprovalRequest - approval decision
        // 3. ToolResult - after execution
        // 4. TextDelta events - for final response

        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let sub_ctx = test_sub_agent_context();

        // Create a file to read
        let ws = test_ctx.workspace_path().await;
        let test_file = ws.join("sequence_test.txt");
        std::fs::write(&test_file, "Sequence test content").unwrap();

        // Model returns tool call then text
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call(
                "read_file",
                serde_json::json!({"path": test_file.to_string_lossy()}),
            ),
            MockResponse::text("Done reading the file."),
        ]);

        let initial_history = vec![user_message("Read the sequence test file")];
        let _ = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            initial_history,
            sub_ctx,
            &ctx,
        )
        .await;

        // Collect and analyze event sequence
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();

        // Find indices of key events
        // Note: ToolRequest is only captured to sidecar, not emitted to frontend channel
        let auto_approved_idx = events
            .iter()
            .position(|e| matches!(e, AiEvent::ToolAutoApproved { .. }));
        let tool_result_idx = events
            .iter()
            .position(|e| matches!(e, AiEvent::ToolResult { .. }));
        let text_delta_idx = events
            .iter()
            .position(|e| matches!(e, AiEvent::TextDelta { .. }));

        // Verify expected frontend events are present
        // read_file is in ALLOW_TOOLS so gets auto-approved by policy
        assert!(
            auto_approved_idx.is_some(),
            "Should have ToolAutoApproved event"
        );
        assert!(tool_result_idx.is_some(), "Should have ToolResult event");
        assert!(text_delta_idx.is_some(), "Should have TextDelta event");

        // Verify event ordering: Approved -> Result
        // (TextDelta can come before or after depending on streaming)
        let approved_idx = auto_approved_idx.unwrap();
        let result_idx = tool_result_idx.unwrap();

        assert!(
            approved_idx < result_idx,
            "ToolAutoApproved should come before ToolResult"
        );
    }

    #[tokio::test]
    async fn test_behavioral_equivalence_error_handling() {
        // Verify that error handling in the generic loop matches expected behavior.
        //
        // Tests:
        // 1. Tool policy denials produce correct error results
        // 2. Planning mode restrictions work correctly
        // 3. Constraint violations are handled properly

        // Test 1: Policy denial (in Default mode, denied tools should be rejected)
        {
            let test_ctx = TestContextBuilder::new()
                .deny_tool("forbidden_tool")
                .agent_mode(AgentMode::Default)
                .build()
                .await;

            let client = test_llm_client();
            let ctx = test_ctx.as_agentic_context_with_client(&client);
            let sub_ctx = test_sub_agent_context();

            // Model tries to call a denied tool
            let model = MockCompletionModel::new(vec![
                MockResponse::tool_call("forbidden_tool", serde_json::json!({})),
                MockResponse::text("Understood, the tool was denied."),
            ]);

            let result = run_agentic_loop_generic(
                &model,
                "You are a helpful assistant.",
                vec![user_message("Use the forbidden tool")],
                sub_ctx,
                &ctx,
            )
            .await;

            assert!(result.is_ok(), "Loop should complete even with denied tool");

            // Verify denial event was emitted
            let mut test_ctx = test_ctx;
            let events = test_ctx.collect_events();
            let has_denied = events.iter().any(|e| {
                matches!(e, AiEvent::ToolDenied { tool_name, .. } if tool_name == "forbidden_tool")
            });
            assert!(has_denied, "Should emit ToolDenied event for policy denial");
        }

        // Test 2: Planning mode restriction
        {
            let test_ctx = TestContextBuilder::new()
                .agent_mode(AgentMode::Planning)
                .build()
                .await;

            let client = test_llm_client();
            let ctx = test_ctx.as_agentic_context_with_client(&client);
            let sub_ctx = test_sub_agent_context();

            // Model tries to call a write tool in planning mode
            let model = MockCompletionModel::new(vec![
                MockResponse::tool_call(
                    "write_file",
                    serde_json::json!({"path": "test.txt", "content": "test"}),
                ),
                MockResponse::text("Cannot write in planning mode."),
            ]);

            let result = run_agentic_loop_generic(
                &model,
                "You are a helpful assistant.",
                vec![user_message("Write a file")],
                sub_ctx,
                &ctx,
            )
            .await;

            assert!(
                result.is_ok(),
                "Loop should complete even with planning mode denial"
            );

            // Verify denial event was emitted
            let mut test_ctx = test_ctx;
            let events = test_ctx.collect_events();
            let has_planning_denied = events.iter().any(|e| {
                matches!(e, AiEvent::ToolDenied { reason, .. } if reason.to_lowercase().contains("planning mode"))
            });
            assert!(
                has_planning_denied,
                "Should emit ToolDenied event for planning mode restriction"
            );
        }

        // Test 3: Constraint violation (e.g., blocked URL in web_fetch)
        {
            let test_ctx = TestContextBuilder::new()
                .agent_mode(AgentMode::AutoApprove)
                .build()
                .await;

            let client = test_llm_client();
            let ctx = test_ctx.as_agentic_context_with_client(&client);
            let sub_ctx = test_sub_agent_context();

            // Model tries to fetch localhost (blocked by default constraints)
            let model = MockCompletionModel::new(vec![
                MockResponse::tool_call(
                    "web_fetch",
                    serde_json::json!({"url": "http://localhost:8080/api"}),
                ),
                MockResponse::text("The URL was blocked."),
            ]);

            let result = run_agentic_loop_generic(
                &model,
                "You are a helpful assistant.",
                vec![user_message("Fetch localhost")],
                sub_ctx,
                &ctx,
            )
            .await;

            assert!(
                result.is_ok(),
                "Loop should complete even with constraint violation"
            );

            // Verify denial event was emitted
            let mut test_ctx = test_ctx;
            let events = test_ctx.collect_events();
            let has_constraint_denied = events
                .iter()
                .any(|e| matches!(e, AiEvent::ToolDenied { .. }));
            assert!(
                has_constraint_denied,
                "Should emit ToolDenied event for constraint violation"
            );
        }
    }

    #[tokio::test]
    async fn test_behavioral_equivalence_context_management() {
        // Verify that context window management works correctly in the generic loop.
        //
        // Tests:
        // 1. Token usage is tracked and accumulated across iterations
        // 2. Context manager is updated with message history
        // 3. Large tool responses are truncated appropriately

        let test_ctx = TestContextBuilder::new()
            .agent_mode(AgentMode::AutoApprove)
            .build()
            .await;

        let client = test_llm_client();
        let ctx = test_ctx.as_agentic_context_with_client(&client);
        let sub_ctx = test_sub_agent_context();

        // Create a file with substantial content
        let ws = test_ctx.workspace_path().await;
        let test_file = ws.join("context_test.txt");
        let large_content = "This is line 1.\n".repeat(100);
        std::fs::write(&test_file, &large_content).unwrap();

        // Model performs multiple tool calls to accumulate tokens
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call(
                "read_file",
                serde_json::json!({"path": test_file.to_string_lossy()}),
            ),
            MockResponse::text("I read the file with 100 lines."),
        ]);

        let initial_history = vec![user_message("Read the context test file")];
        let result = run_agentic_loop_generic(
            &model,
            "You are a helpful assistant.",
            initial_history,
            sub_ctx,
            &ctx,
        )
        .await;

        assert!(result.is_ok(), "Agentic loop should succeed");
        let (_, _reasoning, final_history, token_usage) = result.unwrap();

        // Verify token usage accumulation
        let usage = token_usage.expect("Should have token usage");
        assert!(
            usage.total() > 0,
            "Total tokens should be non-zero after tool execution"
        );

        // Verify message history was properly maintained
        // Should have: initial user message, assistant with tool call, user with tool result, final assistant response
        assert!(
            final_history.len() >= 3,
            "History should contain multiple messages after tool execution"
        );

        // Verify context manager state was updated
        let ctx_stats = ctx.context_manager.stats().await;
        assert!(
            ctx_stats.total_tokens > 0,
            "Context manager should track estimated tokens"
        );

        // Collect events and verify context-related events if any warnings/truncations occurred
        let mut test_ctx = test_ctx;
        let events = test_ctx.collect_events();

        // Check for truncation event (may or may not occur depending on response size)
        let truncation_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, AiEvent::ToolResponseTruncated { .. }))
            .collect();

        // If there was a truncation event, verify it has valid data
        for event in truncation_events {
            if let AiEvent::ToolResponseTruncated {
                original_tokens,
                truncated_tokens,
                ..
            } = event
            {
                assert!(
                    *truncated_tokens <= *original_tokens,
                    "Truncated tokens should be <= original"
                );
            }
        }
    }

    // ========================================================================
    // Sub-Agent Executor Tests (Phase 0.5)
    // ========================================================================

    use qbit_sub_agents::{
        execute_sub_agent, SubAgentDefinition, SubAgentExecutorContext, ToolProvider,
        MAX_AGENT_DEPTH,
    };
    use rig::completion::request::ToolDefinition;

    /// Mock ToolProvider for testing sub-agent execution.
    struct MockToolProvider {
        allowed_tools: Vec<String>,
    }

    impl MockToolProvider {
        fn new() -> Self {
            Self {
                allowed_tools: vec![
                    "read_file".to_string(),
                    "glob".to_string(),
                    "grep".to_string(),
                ],
            }
        }

        fn with_allowed_tools(tools: Vec<String>) -> Self {
            Self {
                allowed_tools: tools,
            }
        }
    }

    #[async_trait::async_trait]
    impl ToolProvider for MockToolProvider {
        fn get_all_tool_definitions(&self) -> Vec<ToolDefinition> {
            // Return minimal tool definitions for testing
            self.allowed_tools
                .iter()
                .map(|name| ToolDefinition {
                    name: name.clone(),
                    description: format!("Mock {} tool", name),
                    parameters: serde_json::json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
                })
                .collect()
        }

        fn filter_tools_by_allowed(
            &self,
            tools: Vec<ToolDefinition>,
            allowed: &[String],
        ) -> Vec<ToolDefinition> {
            if allowed.is_empty() {
                tools
            } else {
                tools
                    .into_iter()
                    .filter(|t| allowed.contains(&t.name))
                    .collect()
            }
        }

        async fn execute_web_fetch_tool(
            &self,
            tool_name: &str,
            _args: &serde_json::Value,
        ) -> (serde_json::Value, bool) {
            (
                serde_json::json!({ "error": format!("Mock web_fetch tool {} not implemented", tool_name) }),
                false,
            )
        }

        fn normalize_run_pty_cmd_args(&self, args: serde_json::Value) -> serde_json::Value {
            args
        }
    }

    /// Create a test sub-agent definition
    fn test_sub_agent_definition_for_executor(id: &str) -> SubAgentDefinition {
        SubAgentDefinition::new(
            id,
            format!("Test Agent {}", id),
            "A test sub-agent for unit testing",
            "You are a test sub-agent. Respond with a simple message.",
        )
        .with_tools(vec!["read_file".to_string(), "glob".to_string()])
        .with_max_iterations(3)
    }

    #[tokio::test]
    async fn test_sub_agent_context_inheritance() {
        // Verify sub-agent inherits parent context correctly
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        // Create parent context with specific values
        let mut parent_variables = std::collections::HashMap::new();
        parent_variables.insert(
            "project_name".to_string(),
            serde_json::json!("test-project"),
        );
        parent_variables.insert("version".to_string(), serde_json::json!("1.0.0"));

        let parent_context = SubAgentContext {
            original_request: "Analyze the codebase".to_string(),
            conversation_summary: Some("User asked to analyze code quality".to_string()),
            variables: parent_variables.clone(),
            depth: 1,
        };

        // Create a simple mock model that returns text immediately (no tool calls)
        let model = MockCompletionModel::with_text("Analysis complete. No issues found.");

        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        let agent_def = test_sub_agent_definition_for_executor("analyzer");
        let tool_provider = MockToolProvider::new();

        let result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({ "task": "Analyze the code" }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();

        // Verify the sub-agent context inherited from parent
        assert_eq!(
            result.context.original_request, parent_context.original_request,
            "Sub-agent should inherit original_request"
        );
        assert_eq!(
            result.context.conversation_summary, parent_context.conversation_summary,
            "Sub-agent should inherit conversation_summary"
        );
        assert_eq!(
            result.context.variables.get("project_name"),
            parent_variables.get("project_name"),
            "Sub-agent should inherit variables"
        );

        // Verify depth was incremented
        assert_eq!(
            result.context.depth,
            parent_context.depth + 1,
            "Sub-agent depth should be parent depth + 1"
        );

        // Verify the agent completed successfully
        assert!(result.success, "Sub-agent should complete successfully");
    }

    #[tokio::test]
    async fn test_sub_agent_max_depth_limit() {
        // Verify depth limit prevents infinite recursion
        // Note: The depth check is done in the main agentic loop, not in execute_sub_agent itself
        // So we test that the depth is properly incremented and can be checked

        let parent_at_max_depth = SubAgentContext {
            original_request: "Test".to_string(),
            conversation_summary: None,
            variables: std::collections::HashMap::new(),
            depth: MAX_AGENT_DEPTH - 1, // One below max
        };

        // Simulate what the agentic loop does: check depth before calling sub-agent
        let can_spawn_sub_agent = parent_at_max_depth.depth < MAX_AGENT_DEPTH - 1;
        assert!(
            !can_spawn_sub_agent,
            "Should not be able to spawn sub-agent at max depth - 1"
        );

        // Verify at depth 0 (normal case) sub-agents are allowed
        let parent_at_zero = SubAgentContext {
            depth: 0,
            ..Default::default()
        };
        assert!(
            parent_at_zero.depth < MAX_AGENT_DEPTH - 1,
            "Should be able to spawn sub-agent at depth 0"
        );

        // Verify the constant is reasonable (compile-time checks)
        const _: () = assert!(MAX_AGENT_DEPTH >= 2);
        const _: () = assert!(MAX_AGENT_DEPTH <= 10);
    }

    #[tokio::test]
    async fn test_sub_agent_result_propagation() {
        // Verify sub-agent results return to parent correctly
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        let parent_context = test_sub_agent_context();
        let model =
            MockCompletionModel::with_text("Task completed successfully with detailed analysis.");

        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        let agent_def = test_sub_agent_definition_for_executor("executor");
        let tool_provider = MockToolProvider::new();

        let result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({
                "task": "Execute the given task",
                "context": "Additional context for the task"
            }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();

        // Verify result structure
        assert_eq!(
            result.agent_id, "executor",
            "Should return correct agent_id"
        );
        assert!(
            result.response.contains("Task completed"),
            "Response should contain model output"
        );
        assert!(result.success, "Should indicate success");
        // Duration is tracked - may be 0 on very fast mock execution
        // The important thing is the field exists and is set

        // Verify context is returned (allows parent to access updated state)
        assert_eq!(
            result.context.depth, 1,
            "Context depth should be incremented"
        );
    }

    #[tokio::test]
    async fn test_sub_agent_events_emitted() {
        // Verify SubAgentStarted, SubAgentCompleted events are emitted
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        let parent_context = test_sub_agent_context();
        let model = MockCompletionModel::with_text("Events test complete.");

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        let agent_def = test_sub_agent_definition_for_executor("event_tester");
        let tool_provider = MockToolProvider::new();

        let _result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({ "task": "Test event emission" }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();

        // Collect all emitted events
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        // Verify SubAgentStarted event was emitted
        let started_event = events.iter().find(|e| {
            matches!(e, AiEvent::SubAgentStarted { agent_id, .. } if agent_id == "event_tester")
        });
        assert!(started_event.is_some(), "Should emit SubAgentStarted event");

        // Verify SubAgentStarted has correct fields
        if let Some(AiEvent::SubAgentStarted {
            agent_id,
            agent_name,
            task,
            depth,
            ..
        }) = started_event
        {
            assert_eq!(agent_id, "event_tester");
            assert!(agent_name.contains("Test Agent"));
            assert_eq!(task, "Test event emission");
            assert_eq!(*depth, 1); // Parent depth was 0
        }

        // Verify SubAgentCompleted event was emitted
        let completed_event = events.iter().find(|e| {
            matches!(e, AiEvent::SubAgentCompleted { agent_id, .. } if agent_id == "event_tester")
        });
        assert!(
            completed_event.is_some(),
            "Should emit SubAgentCompleted event"
        );

        // Verify SubAgentCompleted has correct fields
        if let Some(AiEvent::SubAgentCompleted {
            agent_id,
            response,
            duration_ms: _,
            parent_request_id: _,
        }) = completed_event
        {
            assert_eq!(agent_id, "event_tester");
            assert!(response.contains("Events test complete"));
            // duration_ms may be 0 on very fast mock execution
        }
    }

    #[tokio::test]
    async fn test_sub_agent_error_handling() {
        // Verify errors in sub-agent are handled gracefully
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        let parent_context = test_sub_agent_context();

        // Create a model that simulates an error by returning empty responses repeatedly
        // until max_iterations is hit (which triggers SubAgentError)
        let model = MockCompletionModel::new(vec![
            // Return tool call that will fail
            MockResponse::tool_call("nonexistent_tool", serde_json::json!({ "arg": "value" })),
            // Continue returning tool calls to hit max_iterations
            MockResponse::tool_call("another_nonexistent_tool", serde_json::json!({})),
            MockResponse::tool_call("yet_another_tool", serde_json::json!({})),
            // After max_iterations (3), loop should exit
        ]);

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        // Create agent with very low max_iterations to trigger the error path
        let agent_def = SubAgentDefinition::new(
            "error_tester",
            "Error Test Agent",
            "Tests error handling",
            "You are a test agent.",
        )
        .with_tools(vec![]) // No tools allowed
        .with_max_iterations(3);

        let tool_provider = MockToolProvider::new();

        let result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({ "task": "Trigger error condition" }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();

        // The sub-agent should complete (not panic) even with errors
        // Result is returned with basic structure even if errors occurred
        // Note: The response may be empty and duration 0 on fast mock execution
        assert!(
            result.agent_id == "error_tester",
            "Should return a result with correct agent_id even when errors occur"
        );

        // Check for SubAgentError event (emitted when max_iterations is reached)
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        let error_event = events.iter().find(
            |e| matches!(e, AiEvent::SubAgentError { agent_id, .. } if agent_id == "error_tester"),
        );

        // Error event should be emitted when max_iterations is reached
        assert!(
            error_event.is_some(),
            "Should emit SubAgentError event when max iterations reached"
        );

        if let Some(AiEvent::SubAgentError { error, .. }) = error_event {
            assert!(
                error.contains("Maximum iterations"),
                "Error should mention max iterations"
            );
        }
    }

    #[tokio::test]
    async fn test_sub_agent_tool_restrictions() {
        // Verify sub-agents respect tool policies (allowed_tools)
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        let parent_context = test_sub_agent_context();
        let model = MockCompletionModel::with_text("Tool restriction test complete.");

        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        // Create agent with restricted tools (only read_file allowed)
        let agent_def = SubAgentDefinition::new(
            "restricted_agent",
            "Restricted Agent",
            "Agent with limited tools",
            "You are a restricted agent with only read access.",
        )
        .with_tools(vec!["read_file".to_string()]) // Only read_file allowed
        .with_max_iterations(5);

        // Create tool provider with more tools than allowed
        let tool_provider = MockToolProvider::with_allowed_tools(vec![
            "read_file".to_string(),
            "write_file".to_string(),
            "delete_file".to_string(),
            "glob".to_string(),
        ]);

        // Get filtered tools
        let all_tools = tool_provider.get_all_tool_definitions();
        let filtered_tools =
            tool_provider.filter_tools_by_allowed(all_tools, &agent_def.allowed_tools);

        // Verify tool filtering works correctly
        assert_eq!(
            filtered_tools.len(),
            1,
            "Should only have 1 tool after filtering"
        );
        assert_eq!(
            filtered_tools[0].name, "read_file",
            "Filtered tool should be read_file"
        );

        // Execute sub-agent to verify it works with restricted tools
        let result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({ "task": "Read a file" }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();

        assert!(
            result.success,
            "Sub-agent should succeed with restricted tools"
        );
    }

    #[tokio::test]
    async fn test_sub_agent_timeout_behavior() {
        // Verify sub-agents timeout appropriately (via max_iterations)
        // Note: Sub-agents use max_iterations for timeout control, not wall-clock time
        let test_ctx = TestContextBuilder::new().build().await;
        let workspace = test_ctx.workspace_path().await;

        let parent_context = test_sub_agent_context();

        // Create model that continuously returns tool calls to simulate long-running operation
        let model = MockCompletionModel::new(vec![
            MockResponse::tool_call("read_file", serde_json::json!({ "path": "file1.txt" })),
            MockResponse::tool_call("read_file", serde_json::json!({ "path": "file2.txt" })),
            MockResponse::tool_call("read_file", serde_json::json!({ "path": "file3.txt" })),
            MockResponse::tool_call("read_file", serde_json::json!({ "path": "file4.txt" })),
            MockResponse::tool_call("read_file", serde_json::json!({ "path": "file5.txt" })),
            // More calls than max_iterations
        ]);

        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let tool_registry = Arc::new(RwLock::new(ToolRegistry::new(workspace.clone()).await));

        let sub_ctx = SubAgentExecutorContext {
            event_tx: &event_tx,
            tool_registry: &tool_registry,
            workspace: &Arc::new(RwLock::new(workspace)),
            provider_name: "mock",
            model_name: "mock-model",
            session_id: None,
            transcript_base_dir: None,
        };

        // Create agent with very low max_iterations to simulate timeout
        let agent_def = SubAgentDefinition::new(
            "timeout_tester",
            "Timeout Test Agent",
            "Tests timeout via max_iterations",
            "You are a test agent.",
        )
        .with_tools(vec!["read_file".to_string()])
        .with_max_iterations(2); // Very low to trigger "timeout"

        let tool_provider = MockToolProvider::new();

        let start = std::time::Instant::now();
        let result = execute_sub_agent(
            &agent_def,
            &serde_json::json!({ "task": "Read many files" }),
            &parent_context,
            &model,
            sub_ctx,
            &tool_provider,
            "test-parent-request-id",
        )
        .await
        .unwrap();
        let elapsed = start.elapsed();

        // Collect events
        let mut events = Vec::new();
        while let Ok(event) = event_rx.try_recv() {
            events.push(event);
        }

        // Verify the loop stopped at max_iterations
        let error_event = events.iter().find(|e| {
            matches!(e, AiEvent::SubAgentError { error, .. } if error.contains("Maximum iterations"))
        });
        assert!(
            error_event.is_some(),
            "Should emit error when max_iterations exceeded"
        );

        // Verify it didn't take too long (should be fast since it's mocked)
        assert!(
            elapsed.as_secs() < 5,
            "Sub-agent should complete quickly after hitting max_iterations"
        );

        // Verify the result is returned even when "timed out"
        // Agent ID should match to confirm we got a valid result
        assert_eq!(
            result.agent_id, "timeout_tester",
            "Should return result with correct agent_id when max_iterations hit"
        );
    }
}
