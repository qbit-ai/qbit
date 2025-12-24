import { enableMapSet } from "immer";
import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import type { ApprovalPattern, ReasoningEffort } from "@/lib/ai";
import type { RiskLevel } from "@/lib/tools";

export type { ApprovalPattern, ReasoningEffort, RiskLevel };

// Enable Immer support for Set and Map (needed for processedToolRequests)
enableMapSet();

// Plan types
export type StepStatus = "pending" | "in_progress" | "completed";

export interface PlanStep {
  step: string;
  status: StepStatus;
}

export interface PlanSummary {
  total: number;
  completed: number;
  in_progress: number;
  pending: number;
}

export interface TaskPlan {
  explanation: string | null;
  steps: PlanStep[];
  summary: PlanSummary;
  version: number;
  updated_at: string;
}

// Types
export type SessionMode = "terminal" | "agent";
export type InputMode = "terminal" | "agent";
export type RenderMode = "timeline" | "fullterm";
export type AiStatus = "disconnected" | "initializing" | "ready" | "error";

/**
 * Agent mode determines how tool approvals are handled:
 * - default: Tool approval required based on policy (normal HITL)
 * - auto-approve: All tool calls are automatically approved
 * - planning: Only read-only tools allowed (no modifications)
 */
export type AgentMode = "default" | "auto-approve" | "planning";

export type NotificationType = "info" | "success" | "warning" | "error";

export interface Notification {
  id: string;
  type: NotificationType;
  title: string;
  message?: string;
  timestamp: string;
  read: boolean;
}

export interface AiConfig {
  provider: string;
  model: string;
  status: AiStatus;
  errorMessage?: string;
  // OpenAI specific: reasoning effort level for models like gpt-5.2
  reasoningEffort?: ReasoningEffort;
  // Vertex AI specific config (for model switching)
  vertexConfig?: {
    workspace: string;
    credentialsPath: string;
    projectId: string;
    location: string;
  };
}

export interface Session {
  id: string;
  name: string;
  workingDirectory: string;
  createdAt: string;
  mode: SessionMode;
  inputMode?: InputMode; // Toggle button state for unified input (defaults to "agent")
  agentMode?: AgentMode; // Agent behavior mode (defaults to "default")
  renderMode?: RenderMode; // How to render terminal content (defaults to "timeline")
  customName?: string; // User-defined custom name (set via double-click)
  processName?: string; // Detected running process name
  // Per-session AI configuration (provider + model)
  aiConfig?: AiConfig;
  // Current task plan (if any)
  plan?: TaskPlan;
}

// Unified timeline block types
export type UnifiedBlock =
  | { id: string; type: "command"; timestamp: string; data: CommandBlock }
  | {
      id: string;
      type: "agent_message";
      timestamp: string;
      data: AgentMessage;
    }
  | {
      id: string;
      type: "agent_streaming";
      timestamp: string;
      data: { content: string; toolCalls?: ToolCall[] };
    };

export interface CommandBlock {
  id: string;
  sessionId: string;
  command: string;
  output: string;
  exitCode: number | null;
  startTime: string;
  durationMs: number | null;
  workingDirectory: string;
  isCollapsed: boolean;
}

/** Finalized streaming block for persisted messages */
export type FinalizedStreamingBlock =
  | { type: "text"; content: string }
  | { type: "tool"; toolCall: ToolCall }
  | { type: "udiff_result"; response: string; durationMs: number };

export interface AgentMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
  isStreaming?: boolean;
  toolCalls?: ToolCall[];
  /** Interleaved text and tool call blocks from streaming (preserves order) */
  streamingHistory?: FinalizedStreamingBlock[];
  /** Extended thinking content from the model's reasoning process */
  thinkingContent?: string;
  /** Workflow that was executed during this message (if any) */
  workflow?: ActiveWorkflow;
  /** Sub-agents that were spawned during this message */
  subAgents?: ActiveSubAgent[];
  /** Tokens used for this message (if available) */
  tokensUsed?: number;
}

/** Source of a tool call - indicates which agent initiated it */
export type ToolCallSource =
  | { type: "main" }
  | { type: "sub_agent"; agentId: string; agentName: string }
  | {
      type: "workflow";
      workflowId: string;
      workflowName: string;
      /** Current workflow step name */
      stepName?: string;
      /** Current workflow step index (0-based) */
      stepIndex?: number;
    };

export interface ToolCall {
  id: string;
  name: string;
  args: Record<string, unknown>;
  status: "pending" | "approved" | "denied" | "running" | "completed" | "error";
  result?: unknown;
  /** True if this tool was executed by the agent (vs user-initiated) */
  executedByAgent?: boolean;
  /** Risk level of this tool */
  riskLevel?: RiskLevel;
  /** Approval pattern/stats for this tool (if available) */
  stats?: ApprovalPattern;
  /** Suggestion for auto-approve threshold */
  suggestion?: string;
  /** Whether this tool can be auto-approved in the future */
  canLearn?: boolean;
  /** True if this tool was auto-approved */
  autoApproved?: boolean;
  /** Reason for auto-approval (if auto-approved) */
  autoApprovalReason?: string;
  /** Source of this tool call (main agent, sub-agent, or workflow) */
  source?: ToolCallSource;
}

/** Tool call being actively executed by the agent */
export interface ActiveToolCall {
  id: string;
  name: string;
  args: Record<string, unknown>;
  status: "running" | "completed" | "error";
  result?: unknown;
  startedAt: string;
  completedAt?: string;
  /** True if this tool was executed by the agent (vs user-initiated) */
  executedByAgent?: boolean;
  /** Source of this tool call (main agent, sub-agent, or workflow) */
  source?: ToolCallSource;
}

/** Streaming block types for interleaved text and tool calls */
export type StreamingBlock =
  | { type: "text"; content: string }
  | { type: "tool"; toolCall: ActiveToolCall }
  | { type: "udiff_result"; response: string; durationMs: number };

/** Status of a workflow execution */
export type WorkflowStatus = "idle" | "running" | "completed" | "error";

/** A step in a workflow */
export interface WorkflowStep {
  name: string;
  index: number;
  status: "pending" | "running" | "completed" | "error";
  output?: string | null;
  durationMs?: number;
  startedAt?: string;
  completedAt?: string;
}

/** Active workflow execution state */
export interface ActiveWorkflow {
  workflowId: string;
  workflowName: string;
  sessionId: string;
  status: WorkflowStatus;
  steps: WorkflowStep[];
  currentStepIndex: number;
  totalSteps: number;
  startedAt: string;
  completedAt?: string;
  totalDurationMs?: number;
  finalOutput?: string;
  error?: string;
  /** Tool calls executed during this workflow (persisted after completion) */
  toolCalls?: ActiveToolCall[];
}

/** Sub-agent tool call */
export interface SubAgentToolCall {
  id: string;
  name: string;
  args: Record<string, unknown>;
  status: "running" | "completed" | "error";
  result?: unknown;
  startedAt: string;
  completedAt?: string;
}

/** Active sub-agent execution state */
export interface ActiveSubAgent {
  agentId: string;
  agentName: string;
  task: string;
  depth: number;
  status: "running" | "completed" | "error";
  toolCalls: SubAgentToolCall[];
  response?: string;
  error?: string;
  startedAt: string;
  completedAt?: string;
  durationMs?: number;
}

interface PendingCommand {
  command: string | null;
  output: string;
  startTime: string;
  workingDirectory: string;
}

interface QbitState {
  // Sessions
  sessions: Record<string, Session>;
  activeSessionId: string | null;

  // AI configuration
  aiConfig: AiConfig;

  // Unified timeline (Phase 1)
  timelines: Record<string, UnifiedBlock[]>;

  // Terminal state (kept for backward compatibility)
  commandBlocks: Record<string, CommandBlock[]>;
  pendingCommand: Record<string, PendingCommand | null>;

  // Agent state (kept for backward compatibility)
  agentMessages: Record<string, AgentMessage[]>;
  agentStreaming: Record<string, string>;
  streamingBlocks: Record<string, StreamingBlock[]>; // Interleaved text and tool blocks
  streamingTextOffset: Record<string, number>; // Tracks how much text has been assigned to blocks
  agentInitialized: Record<string, boolean>;
  isAgentThinking: Record<string, boolean>; // True when waiting for first content from agent
  pendingToolApproval: Record<string, ToolCall | null>;
  processedToolRequests: Set<string>; // Track processed request IDs to prevent duplicates
  activeToolCalls: Record<string, ActiveToolCall[]>; // Tool calls currently in progress per session

  // Extended thinking state (for models like Opus 4.5)
  thinkingContent: Record<string, string>; // Accumulated thinking content per session
  isThinkingExpanded: Record<string, boolean>; // Whether thinking section is expanded

  // Notifications state
  notifications: Notification[];
  notificationsExpanded: boolean;

  // Workflow state
  activeWorkflows: Record<string, ActiveWorkflow | null>; // Active workflow per session
  workflowHistory: Record<string, ActiveWorkflow[]>; // Completed workflows per session

  // Sub-agent state
  activeSubAgents: Record<string, ActiveSubAgent[]>; // Active sub-agents per session

  // Terminal clear request (incremented to trigger clear)
  terminalClearRequest: Record<string, number>;

  // Token tracking
  sessionTokenUsage: Record<string, number>; // Accumulated token usage per session

  // Session actions
  addSession: (session: Session) => void;
  removeSession: (sessionId: string) => void;
  setActiveSession: (sessionId: string) => void;
  updateWorkingDirectory: (sessionId: string, path: string) => void;
  setSessionMode: (sessionId: string, mode: SessionMode) => void;
  setInputMode: (sessionId: string, mode: InputMode) => void;
  setAgentMode: (sessionId: string, mode: AgentMode) => void;
  setCustomTabName: (sessionId: string, customName: string | null) => void;
  setProcessName: (sessionId: string, processName: string | null) => void;
  setRenderMode: (sessionId: string, mode: RenderMode) => void;

  // Terminal actions
  handlePromptStart: (sessionId: string) => void;
  handlePromptEnd: (sessionId: string) => void;
  handleCommandStart: (sessionId: string, command: string | null) => void;
  handleCommandEnd: (sessionId: string, exitCode: number) => void;
  appendOutput: (sessionId: string, data: string) => void;
  toggleBlockCollapse: (blockId: string) => void;
  clearBlocks: (sessionId: string) => void;
  requestTerminalClear: (sessionId: string) => void;

  // Agent actions
  addAgentMessage: (sessionId: string, message: AgentMessage) => void;
  updateAgentStreaming: (sessionId: string, content: string) => void;
  clearAgentStreaming: (sessionId: string) => void;
  setAgentInitialized: (sessionId: string, initialized: boolean) => void;
  setAgentThinking: (sessionId: string, thinking: boolean) => void;
  setPendingToolApproval: (sessionId: string, tool: ToolCall | null) => void;
  markToolRequestProcessed: (requestId: string) => void;
  isToolRequestProcessed: (requestId: string) => boolean;
  updateToolCallStatus: (
    sessionId: string,
    toolId: string,
    status: ToolCall["status"],
    result?: unknown
  ) => void;
  clearAgentMessages: (sessionId: string) => void;
  restoreAgentMessages: (sessionId: string, messages: AgentMessage[]) => void;
  addActiveToolCall: (
    sessionId: string,
    toolCall: {
      id: string;
      name: string;
      args: Record<string, unknown>;
      executedByAgent?: boolean;
      source?: ToolCallSource;
    }
  ) => void;
  completeActiveToolCall: (
    sessionId: string,
    toolId: string,
    success: boolean,
    result?: unknown
  ) => void;
  clearActiveToolCalls: (sessionId: string) => void;
  // Streaming blocks actions
  addStreamingToolBlock: (
    sessionId: string,
    toolCall: {
      id: string;
      name: string;
      args: Record<string, unknown>;
      executedByAgent?: boolean;
      source?: ToolCallSource;
    }
  ) => void;
  updateStreamingToolBlock: (
    sessionId: string,
    toolId: string,
    success: boolean,
    result?: unknown
  ) => void;
  clearStreamingBlocks: (sessionId: string) => void;
  addUdiffResultBlock: (sessionId: string, response: string, durationMs: number) => void;

  // Thinking content actions
  appendThinkingContent: (sessionId: string, content: string) => void;
  clearThinkingContent: (sessionId: string) => void;
  setThinkingExpanded: (sessionId: string, expanded: boolean) => void;

  // Timeline actions
  clearTimeline: (sessionId: string) => void;

  // Workflow actions
  startWorkflow: (
    sessionId: string,
    workflow: { workflowId: string; workflowName: string; workflowSessionId: string }
  ) => void;
  workflowStepStarted: (
    sessionId: string,
    step: { stepName: string; stepIndex: number; totalSteps: number }
  ) => void;
  workflowStepCompleted: (
    sessionId: string,
    step: { stepName: string; output: string | null; durationMs: number }
  ) => void;
  completeWorkflow: (
    sessionId: string,
    result: { finalOutput: string; totalDurationMs: number }
  ) => void;
  failWorkflow: (sessionId: string, error: { stepName: string | null; error: string }) => void;
  clearActiveWorkflow: (sessionId: string) => void;
  /** Move workflow tool calls from activeToolCalls into the workflow for persistence */
  preserveWorkflowToolCalls: (sessionId: string) => void;

  // Sub-agent actions
  startSubAgent: (
    sessionId: string,
    agent: { agentId: string; agentName: string; task: string; depth: number }
  ) => void;
  addSubAgentToolCall: (
    sessionId: string,
    agentId: string,
    toolCall: { id: string; name: string; args: Record<string, unknown> }
  ) => void;
  completeSubAgentToolCall: (
    sessionId: string,
    agentId: string,
    toolId: string,
    success: boolean,
    result?: unknown
  ) => void;
  completeSubAgent: (
    sessionId: string,
    agentId: string,
    result: { response: string; durationMs: number }
  ) => void;
  failSubAgent: (sessionId: string, agentId: string, error: string) => void;
  clearActiveSubAgents: (sessionId: string) => void;

  // AI config actions
  setAiConfig: (config: Partial<AiConfig>) => void;
  // Per-session AI config actions
  setSessionAiConfig: (sessionId: string, config: Partial<AiConfig>) => void;
  getSessionAiConfig: (sessionId: string) => AiConfig | undefined;

  // Plan actions
  setPlan: (sessionId: string, plan: TaskPlan) => void;

  // Notification actions
  addNotification: (notification: Omit<Notification, "id" | "timestamp" | "read">) => void;
  markNotificationRead: (notificationId: string) => void;
  markAllNotificationsRead: () => void;
  removeNotification: (notificationId: string) => void;
  clearNotifications: () => void;
  setNotificationsExpanded: (expanded: boolean) => void;
}

export const useStore = create<QbitState>()(
  devtools(
    immer((set, _get) => ({
      sessions: {},
      activeSessionId: null,
      aiConfig: {
        provider: "",
        model: "",
        status: "disconnected" as AiStatus,
      },
      timelines: {},
      commandBlocks: {},
      pendingCommand: {},
      agentMessages: {},
      agentStreaming: {},
      streamingBlocks: {},
      streamingTextOffset: {},
      agentInitialized: {},
      isAgentThinking: {},
      pendingToolApproval: {},
      processedToolRequests: new Set<string>(),
      activeToolCalls: {},
      thinkingContent: {},
      isThinkingExpanded: {},
      notifications: [],
      notificationsExpanded: false,
      activeWorkflows: {},
      workflowHistory: {},
      activeSubAgents: {},
      terminalClearRequest: {},
      sessionTokenUsage: {},

      addSession: (session) =>
        set((state) => {
          state.sessions[session.id] = {
            ...session,
            inputMode: session.inputMode ?? "terminal", // Default to terminal mode
          };
          state.activeSessionId = session.id;
          state.timelines[session.id] = [];
          state.commandBlocks[session.id] = [];
          state.pendingCommand[session.id] = null;
          state.agentMessages[session.id] = [];
          state.agentStreaming[session.id] = "";
          state.streamingBlocks[session.id] = [];
          state.streamingTextOffset[session.id] = 0;
          state.agentInitialized[session.id] = false;
          state.isAgentThinking[session.id] = false;
          state.pendingToolApproval[session.id] = null;
          state.activeToolCalls[session.id] = [];
          state.thinkingContent[session.id] = "";
          state.isThinkingExpanded[session.id] = false;
          state.activeWorkflows[session.id] = null;
          state.workflowHistory[session.id] = [];
          state.activeSubAgents[session.id] = [];
        }),

      removeSession: (sessionId) =>
        set((state) => {
          delete state.sessions[sessionId];
          delete state.timelines[sessionId];
          delete state.commandBlocks[sessionId];
          delete state.pendingCommand[sessionId];
          delete state.agentMessages[sessionId];
          delete state.agentStreaming[sessionId];
          delete state.streamingBlocks[sessionId];
          delete state.streamingTextOffset[sessionId];
          delete state.agentInitialized[sessionId];
          delete state.isAgentThinking[sessionId];
          delete state.pendingToolApproval[sessionId];
          delete state.activeToolCalls[sessionId];
          delete state.thinkingContent[sessionId];
          delete state.isThinkingExpanded[sessionId];

          if (state.activeSessionId === sessionId) {
            const remaining = Object.keys(state.sessions);
            state.activeSessionId = remaining[0] ?? null;
          }
        }),

      setActiveSession: (sessionId) =>
        set((state) => {
          state.activeSessionId = sessionId;
        }),

      updateWorkingDirectory: (sessionId, path) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].workingDirectory = path;
          }
        }),

      setSessionMode: (sessionId, mode) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].mode = mode;
          }
        }),

      setInputMode: (sessionId, mode) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].inputMode = mode;
          }
        }),

      setAgentMode: (sessionId, mode) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].agentMode = mode;
          }
        }),

      setCustomTabName: (sessionId, customName) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].customName = customName ?? undefined;
          }
        }),

      setProcessName: (sessionId, processName) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            // Only set process name if there's no custom name
            if (!state.sessions[sessionId].customName) {
              state.sessions[sessionId].processName = processName ?? undefined;
            }
          }
        }),

      setRenderMode: (sessionId, mode) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].renderMode = mode;
          }
        }),

      handlePromptStart: (sessionId) =>
        set((state) => {
          // Finalize any pending command without exit code
          const pending = state.pendingCommand[sessionId];
          if (pending?.command) {
            const blockId = crypto.randomUUID();
            const block: CommandBlock = {
              id: blockId,
              sessionId,
              command: pending.command,
              output: pending.output,
              exitCode: null,
              startTime: pending.startTime,
              durationMs: null,
              workingDirectory: pending.workingDirectory,
              isCollapsed: false,
            };
            if (!state.commandBlocks[sessionId]) {
              state.commandBlocks[sessionId] = [];
            }
            state.commandBlocks[sessionId].push(block);

            // Also push to unified timeline
            if (!state.timelines[sessionId]) {
              state.timelines[sessionId] = [];
            }
            state.timelines[sessionId].push({
              id: blockId,
              type: "command",
              timestamp: pending.startTime,
              data: block,
            });
          }
          state.pendingCommand[sessionId] = null;
        }),

      handlePromptEnd: (_sessionId) => {
        // Ready for input - nothing to do for now
      },

      handleCommandStart: (sessionId, command) =>
        set((state) => {
          const session = state.sessions[sessionId];
          state.pendingCommand[sessionId] = {
            command,
            output: "",
            startTime: new Date().toISOString(),
            workingDirectory: session?.workingDirectory || "",
          };
        }),

      handleCommandEnd: (sessionId, exitCode) =>
        set((state) => {
          const pending = state.pendingCommand[sessionId];
          if (pending) {
            // Skip creating command block for fullterm mode commands
            // Fullterm mode is for interactive apps (vim, ssh, etc.) that use
            // the full terminal - their output shouldn't appear in the timeline
            const session = state.sessions[sessionId];
            const isFullterm = session?.renderMode === "fullterm";

            // Only create a command block if:
            // 1. There was an actual command (not empty)
            // 2. NOT in fullterm mode (those sessions are handled by xterm directly)
            if (pending.command && !isFullterm) {
              const blockId = crypto.randomUUID();
              const block: CommandBlock = {
                id: blockId,
                sessionId,
                command: pending.command,
                output: pending.output,
                exitCode,
                startTime: pending.startTime,
                durationMs: Date.now() - new Date(pending.startTime).getTime(),
                workingDirectory: pending.workingDirectory,
                isCollapsed: false,
              };
              if (!state.commandBlocks[sessionId]) {
                state.commandBlocks[sessionId] = [];
              }
              state.commandBlocks[sessionId].push(block);

              // Also push to unified timeline
              if (!state.timelines[sessionId]) {
                state.timelines[sessionId] = [];
              }
              state.timelines[sessionId].push({
                id: blockId,
                type: "command",
                timestamp: pending.startTime,
                data: block,
              });
            }

            state.pendingCommand[sessionId] = null;
          }
        }),

      appendOutput: (sessionId, data) =>
        set((state) => {
          const pending = state.pendingCommand[sessionId];
          // Only append output if we have an active command (command_start was received)
          // This prevents capturing prompt text as command output
          if (pending) {
            pending.output += data;
          }
        }),

      toggleBlockCollapse: (blockId) =>
        set((state) => {
          // Update in legacy commandBlocks
          for (const blocks of Object.values(state.commandBlocks)) {
            const block = blocks.find((b) => b.id === blockId);
            if (block) {
              block.isCollapsed = !block.isCollapsed;
              break;
            }
          }
          // Also update in unified timeline
          for (const timeline of Object.values(state.timelines)) {
            const unifiedBlock = timeline.find((b) => b.type === "command" && b.id === blockId);
            if (unifiedBlock && unifiedBlock.type === "command") {
              unifiedBlock.data.isCollapsed = !unifiedBlock.data.isCollapsed;
              break;
            }
          }
        }),

      clearBlocks: (sessionId) =>
        set((state) => {
          state.commandBlocks[sessionId] = [];
          state.pendingCommand[sessionId] = null;
        }),

      requestTerminalClear: (sessionId) =>
        set((state) => {
          state.terminalClearRequest[sessionId] = (state.terminalClearRequest[sessionId] ?? 0) + 1;
        }),

      // Agent actions
      addAgentMessage: (sessionId, message) =>
        set((state) => {
          if (!state.agentMessages[sessionId]) {
            state.agentMessages[sessionId] = [];
          }
          state.agentMessages[sessionId].push(message);

          // Also push to unified timeline
          if (!state.timelines[sessionId]) {
            state.timelines[sessionId] = [];
          }
          state.timelines[sessionId].push({
            id: message.id,
            type: "agent_message",
            timestamp: message.timestamp,
            data: message,
          });

          // Accumulate token usage for the session if available
          if (message.tokensUsed) {
            state.sessionTokenUsage[sessionId] =
              (state.sessionTokenUsage[sessionId] ?? 0) + message.tokensUsed;
          }
        }),

      updateAgentStreaming: (sessionId, delta) =>
        set((state) => {
          // Append delta to accumulated text
          state.agentStreaming[sessionId] = (state.agentStreaming[sessionId] || "") + delta;

          // Update streaming blocks - just append the new delta text
          if (!state.streamingBlocks[sessionId]) {
            state.streamingBlocks[sessionId] = [];
          }
          const blocks = state.streamingBlocks[sessionId];

          // Just append or update the current text block with the new delta
          const lastBlock = blocks[blocks.length - 1];
          if (lastBlock && lastBlock.type === "text") {
            // Append delta to the last text block
            lastBlock.content += delta;
          } else if (delta) {
            // Add new text block (after a tool block or as first block)
            blocks.push({ type: "text", content: delta });
          }
        }),

      clearAgentStreaming: (sessionId) =>
        set((state) => {
          state.agentStreaming[sessionId] = "";
          state.streamingBlocks[sessionId] = [];
          state.streamingTextOffset[sessionId] = 0;
        }),

      setAgentInitialized: (sessionId, initialized) =>
        set((state) => {
          state.agentInitialized[sessionId] = initialized;
        }),

      setAgentThinking: (sessionId, thinking) =>
        set((state) => {
          state.isAgentThinking[sessionId] = thinking;
        }),

      setPendingToolApproval: (sessionId, tool) =>
        set((state) => {
          state.pendingToolApproval[sessionId] = tool;
        }),

      markToolRequestProcessed: (requestId) =>
        set((state) => {
          state.processedToolRequests.add(requestId);
        }),

      isToolRequestProcessed: (requestId) => {
        return _get().processedToolRequests.has(requestId);
      },

      updateToolCallStatus: (sessionId, toolId, status, result) =>
        set((state) => {
          const messages = state.agentMessages[sessionId];
          if (messages) {
            for (const msg of messages) {
              const tool = msg.toolCalls?.find((t) => t.id === toolId);
              if (tool) {
                tool.status = status;
                if (result !== undefined) tool.result = result;
                break;
              }
            }
          }
        }),

      clearAgentMessages: (sessionId) =>
        set((state) => {
          state.agentMessages[sessionId] = [];
          state.agentStreaming[sessionId] = "";
        }),

      restoreAgentMessages: (sessionId, messages) =>
        set((state) => {
          state.agentMessages[sessionId] = messages;
          state.agentStreaming[sessionId] = "";
          // Replace the timeline with restored messages (clear first, then add)
          state.timelines[sessionId] = [];
          for (const message of messages) {
            state.timelines[sessionId].push({
              id: message.id,
              type: "agent_message",
              timestamp: message.timestamp,
              data: message,
            });
          }
        }),

      addActiveToolCall: (sessionId, toolCall) =>
        set((state) => {
          if (!state.activeToolCalls[sessionId]) {
            state.activeToolCalls[sessionId] = [];
          }
          state.activeToolCalls[sessionId].push({
            ...toolCall,
            status: "running",
            startedAt: new Date().toISOString(),
          });
        }),

      completeActiveToolCall: (sessionId, toolId, success, result) =>
        set((state) => {
          const tools = state.activeToolCalls[sessionId];
          if (tools) {
            const tool = tools.find((t) => t.id === toolId);
            if (tool) {
              tool.status = success ? "completed" : "error";
              tool.result = result;
              tool.completedAt = new Date().toISOString();
            }
          }
        }),

      clearActiveToolCalls: (sessionId) =>
        set((state) => {
          state.activeToolCalls[sessionId] = [];
        }),

      // Streaming blocks actions
      addStreamingToolBlock: (sessionId, toolCall) =>
        set((state) => {
          if (!state.streamingBlocks[sessionId]) {
            state.streamingBlocks[sessionId] = [];
          }

          const blocks = state.streamingBlocks[sessionId];

          // Append the tool block (text is already added to last text block by updateAgentStreaming)
          blocks.push({
            type: "tool",
            toolCall: {
              ...toolCall,
              status: "running",
              startedAt: new Date().toISOString(),
            },
          });
        }),

      updateStreamingToolBlock: (sessionId, toolId, success, result) =>
        set((state) => {
          const blocks = state.streamingBlocks[sessionId];
          if (blocks) {
            for (const block of blocks) {
              if (block.type === "tool" && block.toolCall.id === toolId) {
                block.toolCall.status = success ? "completed" : "error";
                block.toolCall.result = result;
                block.toolCall.completedAt = new Date().toISOString();
                break;
              }
            }
          }
        }),

      clearStreamingBlocks: (sessionId) =>
        set((state) => {
          state.streamingBlocks[sessionId] = [];
        }),

      addUdiffResultBlock: (sessionId, response, durationMs) =>
        set((state) => {
          if (!state.streamingBlocks[sessionId]) {
            state.streamingBlocks[sessionId] = [];
          }
          state.streamingBlocks[sessionId].push({
            type: "udiff_result",
            response,
            durationMs,
          });
        }),

      // Thinking content actions
      appendThinkingContent: (sessionId, content) =>
        set((state) => {
          const previousLength = state.thinkingContent[sessionId]?.length ?? 0;
          if (!state.thinkingContent[sessionId]) {
            state.thinkingContent[sessionId] = "";
          }
          state.thinkingContent[sessionId] += content;
          console.log("[Thinking] Store appendThinkingContent:", {
            sessionId,
            appendedLength: content.length,
            previousTotalLength: previousLength,
            newTotalLength: state.thinkingContent[sessionId].length,
          });
        }),

      clearThinkingContent: (sessionId) =>
        set((state) => {
          const previousLength = state.thinkingContent[sessionId]?.length ?? 0;
          console.log("[Thinking] Store clearThinkingContent:", {
            sessionId,
            previousLength,
          });
          state.thinkingContent[sessionId] = "";
        }),

      setThinkingExpanded: (sessionId, expanded) =>
        set((state) => {
          state.isThinkingExpanded[sessionId] = expanded;
        }),

      // Timeline actions
      clearTimeline: (sessionId) =>
        set((state) => {
          state.timelines[sessionId] = [];
          // Also clear the legacy stores for consistency
          state.commandBlocks[sessionId] = [];
          state.pendingCommand[sessionId] = null;
          state.agentMessages[sessionId] = [];
          state.agentStreaming[sessionId] = "";
          state.streamingBlocks[sessionId] = [];
        }),

      // Workflow actions
      startWorkflow: (sessionId, workflow) =>
        set((state) => {
          state.activeWorkflows[sessionId] = {
            workflowId: workflow.workflowId,
            workflowName: workflow.workflowName,
            sessionId: workflow.workflowSessionId,
            status: "running",
            steps: [],
            currentStepIndex: -1,
            totalSteps: 0,
            startedAt: new Date().toISOString(),
          };
        }),

      workflowStepStarted: (sessionId, step) =>
        set((state) => {
          const workflow = state.activeWorkflows[sessionId];
          if (!workflow) return;

          workflow.currentStepIndex = step.stepIndex;
          workflow.totalSteps = step.totalSteps;

          // Initialize step if not already present
          if (!workflow.steps[step.stepIndex]) {
            workflow.steps[step.stepIndex] = {
              name: step.stepName,
              index: step.stepIndex,
              status: "running",
              startedAt: new Date().toISOString(),
            };
          } else {
            workflow.steps[step.stepIndex].status = "running";
            workflow.steps[step.stepIndex].startedAt = new Date().toISOString();
          }
        }),

      workflowStepCompleted: (sessionId, step) =>
        set((state) => {
          const workflow = state.activeWorkflows[sessionId];
          if (!workflow) return;

          // Find the step by name (since index might not be exact)
          const stepData = workflow.steps.find((s) => s.name === step.stepName);
          if (stepData) {
            stepData.status = "completed";
            stepData.output = step.output;
            stepData.durationMs = step.durationMs;
            stepData.completedAt = new Date().toISOString();
          }
        }),

      completeWorkflow: (sessionId, result) =>
        set((state) => {
          const workflow = state.activeWorkflows[sessionId];
          if (!workflow) return;

          workflow.status = "completed";
          workflow.finalOutput = result.finalOutput;
          workflow.totalDurationMs = result.totalDurationMs;
          workflow.completedAt = new Date().toISOString();

          // Move to history (but keep visible in activeWorkflows for current message)
          if (!state.workflowHistory[sessionId]) {
            state.workflowHistory[sessionId] = [];
          }
          state.workflowHistory[sessionId].push({ ...workflow });
          // Note: We intentionally don't clear activeWorkflows here
          // The workflow tree stays visible until the AI response is finalized
        }),

      failWorkflow: (sessionId, error) =>
        set((state) => {
          const workflow = state.activeWorkflows[sessionId];
          if (!workflow) return;

          workflow.status = "error";
          workflow.error = error.error;
          workflow.completedAt = new Date().toISOString();

          // Mark current step as error if specified
          if (error.stepName) {
            const stepData = workflow.steps.find((s) => s.name === error.stepName);
            if (stepData) {
              stepData.status = "error";
            }
          }

          // Move to history (but keep visible in activeWorkflows for current message)
          if (!state.workflowHistory[sessionId]) {
            state.workflowHistory[sessionId] = [];
          }
          state.workflowHistory[sessionId].push({ ...workflow });
          // Note: We intentionally don't clear activeWorkflows here
          // The workflow tree stays visible until the AI response is finalized
        }),

      clearActiveWorkflow: (sessionId) =>
        set((state) => {
          state.activeWorkflows[sessionId] = null;
        }),

      preserveWorkflowToolCalls: (sessionId) =>
        set((state) => {
          const workflow = state.activeWorkflows[sessionId];
          const toolCalls = state.activeToolCalls[sessionId];

          if (!workflow || !toolCalls) return;

          // Filter tool calls that belong to this workflow
          const workflowToolCalls = toolCalls.filter((tool) => {
            const source = tool.source;
            return source?.type === "workflow" && source.workflowId === workflow.workflowId;
          });

          // Store them in the workflow
          workflow.toolCalls = workflowToolCalls;
        }),

      // Sub-agent actions
      startSubAgent: (sessionId, agent) =>
        set((state) => {
          if (!state.activeSubAgents[sessionId]) {
            state.activeSubAgents[sessionId] = [];
          }
          state.activeSubAgents[sessionId].push({
            agentId: agent.agentId,
            agentName: agent.agentName,
            task: agent.task,
            depth: agent.depth,
            status: "running",
            toolCalls: [],
            startedAt: new Date().toISOString(),
          });
        }),

      addSubAgentToolCall: (sessionId, agentId, toolCall) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.agentId === agentId);
          if (agent) {
            agent.toolCalls.push({
              ...toolCall,
              status: "running",
              startedAt: new Date().toISOString(),
            });
          }
        }),

      completeSubAgentToolCall: (sessionId, agentId, toolId, success, result) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.agentId === agentId);
          if (agent) {
            const tool = agent.toolCalls.find((t) => t.id === toolId);
            if (tool) {
              tool.status = success ? "completed" : "error";
              tool.result = result;
              tool.completedAt = new Date().toISOString();
            }
          }
        }),

      completeSubAgent: (sessionId, agentId, result) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.agentId === agentId);
          if (agent) {
            agent.status = "completed";
            agent.response = result.response;
            agent.durationMs = result.durationMs;
            agent.completedAt = new Date().toISOString();
          }
        }),

      failSubAgent: (sessionId, agentId, error) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.agentId === agentId);
          if (agent) {
            agent.status = "error";
            agent.error = error;
            agent.completedAt = new Date().toISOString();
          }
        }),

      clearActiveSubAgents: (sessionId) =>
        set((state) => {
          state.activeSubAgents[sessionId] = [];
        }),

      // AI config actions
      setAiConfig: (config) =>
        set((state) => {
          state.aiConfig = { ...state.aiConfig, ...config };
        }),

      // Per-session AI config actions
      setSessionAiConfig: (sessionId, config) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            const currentConfig = state.sessions[sessionId].aiConfig || {
              provider: "",
              model: "",
              status: "disconnected" as AiStatus,
            };
            state.sessions[sessionId].aiConfig = { ...currentConfig, ...config };
          }
        }),

      getSessionAiConfig: (sessionId) => {
        const session = _get().sessions[sessionId];
        return session?.aiConfig;
      },

      // Plan actions
      setPlan: (sessionId, plan) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].plan = plan;
          }
        }),

      // Notification actions
      addNotification: (notification) =>
        set((state) => {
          state.notifications.unshift({
            ...notification,
            id: crypto.randomUUID(),
            timestamp: new Date().toISOString(),
            read: false,
          });
        }),

      markNotificationRead: (notificationId) =>
        set((state) => {
          const notification = state.notifications.find((n) => n.id === notificationId);
          if (notification) {
            notification.read = true;
          }
        }),

      markAllNotificationsRead: () =>
        set((state) => {
          for (const notification of state.notifications) {
            notification.read = true;
          }
        }),

      removeNotification: (notificationId) =>
        set((state) => {
          state.notifications = state.notifications.filter((n) => n.id !== notificationId);
        }),

      clearNotifications: () =>
        set((state) => {
          state.notifications = [];
        }),

      setNotificationsExpanded: (expanded) =>
        set((state) => {
          state.notificationsExpanded = expanded;
        }),
    })),
    { name: "qbit" }
  )
);

// Stable empty arrays to avoid re-render loops
const EMPTY_BLOCKS: CommandBlock[] = [];
const EMPTY_MESSAGES: AgentMessage[] = [];

// Selectors
export const useActiveSession = () =>
  useStore((state) => {
    const id = state.activeSessionId;
    return id ? state.sessions[id] : null;
  });

export const useSessionBlocks = (sessionId: string) =>
  useStore((state) => state.commandBlocks[sessionId] ?? EMPTY_BLOCKS);

export const useTerminalClearRequest = (sessionId: string) =>
  useStore((state) => state.terminalClearRequest[sessionId] ?? 0);

export const usePendingCommand = (sessionId: string) =>
  useStore((state) => state.pendingCommand[sessionId]);

export const useSessionMode = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.mode ?? "terminal");

export const useAgentMessages = (sessionId: string) =>
  useStore((state) => state.agentMessages[sessionId] ?? EMPTY_MESSAGES);

export const useAgentStreaming = (sessionId: string) =>
  useStore((state) => state.agentStreaming[sessionId] ?? "");

export const useAgentInitialized = (sessionId: string) =>
  useStore((state) => state.agentInitialized[sessionId] ?? false);

export const usePendingToolApproval = (sessionId: string) =>
  useStore((state) => state.pendingToolApproval[sessionId] ?? null);

// Timeline selectors
const EMPTY_TIMELINE: UnifiedBlock[] = [];

export const useSessionTimeline = (sessionId: string) =>
  useStore((state) => state.timelines[sessionId] ?? EMPTY_TIMELINE);

export const useInputMode = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.inputMode ?? "terminal");

export const useAgentMode = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.agentMode ?? "default");

export const useRenderMode = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.renderMode ?? "timeline");

// Active tool calls selector
const EMPTY_TOOL_CALLS: ActiveToolCall[] = [];

export const useActiveToolCalls = (sessionId: string) =>
  useStore((state) => state.activeToolCalls[sessionId] ?? EMPTY_TOOL_CALLS);

// Streaming blocks selector
const EMPTY_STREAMING_BLOCKS: StreamingBlock[] = [];

export const useStreamingBlocks = (sessionId: string) =>
  useStore((state) => state.streamingBlocks[sessionId] ?? EMPTY_STREAMING_BLOCKS);

// AI config selector (global - for backwards compatibility)
export const useAiConfig = () => useStore((state) => state.aiConfig);

// Per-session AI config selector
export const useSessionAiConfig = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.aiConfig);

// Agent thinking selector
export const useIsAgentThinking = (sessionId: string) =>
  useStore((state) => state.isAgentThinking[sessionId] ?? false);

// Extended thinking content selectors
export const useThinkingContent = (sessionId: string) =>
  useStore((state) => state.thinkingContent[sessionId] ?? "");

export const useIsThinkingExpanded = (sessionId: string) =>
  useStore((state) => state.isThinkingExpanded[sessionId] ?? false);

// Notification selectors
const EMPTY_NOTIFICATIONS: Notification[] = [];

export const useNotifications = () =>
  useStore((state) => state.notifications ?? EMPTY_NOTIFICATIONS);

export const useUnreadNotificationCount = () =>
  useStore((state) => state.notifications.filter((n) => !n.read).length);

export const useNotificationsExpanded = () => useStore((state) => state.notificationsExpanded);

// Helper function to clear conversation (both frontend and backend)
// This should be called instead of clearTimeline when you want to reset AI context
export async function clearConversation(sessionId: string): Promise<void> {
  // Clear frontend state
  useStore.getState().clearTimeline(sessionId);

  // Clear backend conversation history (try session-specific first, fall back to global)
  try {
    const { clearAiConversationSession, clearAiConversation } = await import("@/lib/ai");
    // Try session-specific clear first
    try {
      await clearAiConversationSession(sessionId);
    } catch {
      // Fall back to global clear (legacy)
      await clearAiConversation();
    }
  } catch (error) {
    console.warn("Failed to clear backend conversation history:", error);
  }
}

// Helper function to restore a previous session (both frontend and backend)
export async function restoreSession(sessionId: string, identifier: string): Promise<void> {
  const aiModule = await import("@/lib/ai");
  const { restoreAiSession, initAiSession } = aiModule;
  type ProviderConfig = Parameters<typeof initAiSession>[1];
  const { getSettings } = await import("@/lib/settings");

  // Restore backend conversation history and get the session data
  const session = await restoreAiSession(identifier);

  // Convert session messages to AgentMessages for the UI
  const agentMessages: AgentMessage[] = session.messages
    .filter((msg) => msg.role === "user" || msg.role === "assistant")
    .map((msg, index) => ({
      id: `restored-${identifier}-${index}`,
      sessionId,
      role: msg.role as "user" | "assistant",
      content: msg.content,
      timestamp: index === 0 ? session.started_at : session.ended_at,
      isStreaming: false,
    }));

  // Clear existing state first
  useStore.getState().clearTimeline(sessionId);

  // Restore the messages to the store (this also populates the timeline)
  useStore.getState().restoreAgentMessages(sessionId, agentMessages);

  // Switch to agent mode since we're restoring an AI conversation
  useStore.getState().setInputMode(sessionId, "agent");

  // Restore the AI provider/model for this session
  try {
    const settings = await getSettings();
    const workspace = session.workspace_path;
    const provider = session.provider;
    const model = session.model;

    // Build a ProviderConfig based on the restored provider/model
    let config: ProviderConfig | null = null;

    if (
      provider === "anthropic_vertex" &&
      settings.ai.vertex_ai.credentials_path &&
      settings.ai.vertex_ai.project_id &&
      settings.ai.vertex_ai.location
    ) {
      config = {
        provider: "vertex_ai",
        workspace,
        model,
        credentials_path: settings.ai.vertex_ai.credentials_path,
        project_id: settings.ai.vertex_ai.project_id,
        location: settings.ai.vertex_ai.location,
      };
    } else if (provider === "openrouter" && settings.ai.openrouter.api_key) {
      config = {
        provider: "openrouter",
        workspace,
        model,
        api_key: settings.ai.openrouter.api_key,
      };
    } else if (provider === "openai" && settings.ai.openai.api_key) {
      config = {
        provider: "openai",
        workspace,
        model,
        api_key: settings.ai.openai.api_key,
      };
    } else if (provider === "anthropic" && settings.ai.anthropic.api_key) {
      config = {
        provider: "anthropic",
        workspace,
        model,
        api_key: settings.ai.anthropic.api_key,
      };
    } else if (provider === "ollama") {
      config = {
        provider: "ollama",
        workspace,
        model,
      };
    } else if (provider === "gemini" && settings.ai.gemini.api_key) {
      config = {
        provider: "gemini",
        workspace,
        model,
        api_key: settings.ai.gemini.api_key,
      };
    } else if (provider === "groq" && settings.ai.groq.api_key) {
      config = {
        provider: "groq",
        workspace,
        model,
        api_key: settings.ai.groq.api_key,
      };
    } else if (provider === "xai" && settings.ai.xai.api_key) {
      config = {
        provider: "xai",
        workspace,
        model,
        api_key: settings.ai.xai.api_key,
      };
    }

    if (config) {
      // Initialize the AI session with the restored provider/model
      await initAiSession(sessionId, config);

      // Update the store's AI config for this session
      useStore.getState().setSessionAiConfig(sessionId, {
        provider,
        model,
        status: "ready",
      });
    } else {
      console.warn(`Could not restore AI for provider "${provider}" - API key may be missing`);
    }
  } catch (error) {
    console.warn("Failed to restore AI provider/model:", error);
  }
}

// Expose store for testing in development
if (import.meta.env.DEV) {
  (window as unknown as { __QBIT_STORE__: typeof useStore }).__QBIT_STORE__ = useStore;
}
