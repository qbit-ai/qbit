import { enableMapSet } from "immer";
import { create } from "zustand";
import { devtools } from "zustand/middleware";
import { immer } from "zustand/middleware/immer";
import type { ApprovalPattern, ReasoningEffort } from "@/lib/ai";
import { logger } from "@/lib/logger";
import {
  countLeafPanes,
  findPaneById,
  getAllLeafPanes,
  getFirstLeafPane,
  getPaneNeighbor,
  type PaneId,
  type PaneNode,
  removePaneNode,
  type SplitDirection,
  splitPaneNode,
  type TabLayout,
  updatePaneRatio,
} from "@/lib/pane-utils";
import { TerminalInstanceManager } from "@/lib/terminal/TerminalInstanceManager";
import type { RiskLevel } from "@/lib/tools";
import {
  type ContextMetrics,
  type ContextSlice,
  createContextSlice,
  createGitSlice,
  createNotificationSlice,
  type GitSlice,
  type Notification,
  type NotificationSlice,
  type NotificationType,
  selectContextMetrics,
  selectNotifications,
  selectNotificationsExpanded,
  selectUnreadNotificationCount,
} from "./slices";

export type { ApprovalPattern, ReasoningEffort, RiskLevel };
// Re-export pane types from the single source of truth
export type { PaneId, PaneNode, SplitDirection, TabLayout };

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
 * Tab type determines what kind of content is displayed in a tab.
 * - terminal: Standard terminal/agent session with PTY
 * - settings: Settings panel (no PTY, gear icon)
 */
export type TabType = "terminal" | "settings";

/**
 * Agent mode determines how tool approvals are handled:
 * - default: Tool approval required based on policy (normal HITL)
 * - auto-approve: All tool calls are automatically approved
 * - planning: Only read-only tools allowed (no modifications)
 */
export type AgentMode = "default" | "auto-approve" | "planning";

// Re-export types from slices
export type { ContextMetrics, Notification, NotificationType };

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
  /** Type of tab - determines icon and content rendering. Defaults to "terminal" */
  tabType?: TabType;
  name: string;
  workingDirectory: string;
  createdAt: string;
  mode: SessionMode;
  inputMode?: InputMode; // Toggle button state for unified input (defaults to "agent")
  agentMode?: AgentMode; // Agent behavior mode (defaults to "default")
  renderMode?: RenderMode; // How to render terminal content (defaults to "timeline")
  customName?: string; // User-defined custom name (set via double-click)
  processName?: string; // Detected running process name
  virtualEnv?: string | null; // Active Python virtual environment name
  gitBranch?: string | null; // Current git branch (null if not a git repo)
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
      type: "system_hook";
      timestamp: string;
      data: { hooks: string[] };
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
  | { type: "udiff_result"; response: string; durationMs: number }
  | { type: "system_hooks"; hooks: string[] };

/** Result of context compaction operation */
export type CompactionResult =
  | {
      status: "success";
      tokensBefore: number;
      messagesBefore: number;
      messagesAfter: number;
      summaryLength: number;
    }
  | {
      status: "failed";
      tokensBefore: number;
      messagesBefore: number;
      error: string;
    };

export interface AgentMessage {
  id: string;
  sessionId: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: string;
  isStreaming?: boolean;
  /** Image attachments for user messages */
  attachments?: { type: "image"; data: string; media_type?: string; filename?: string }[];
  toolCalls?: ToolCall[];
  /** Interleaved text and tool call blocks from streaming (preserves order) */
  streamingHistory?: FinalizedStreamingBlock[];
  /** Extended thinking content from the model's reasoning process */
  thinkingContent?: string;
  /** Workflow that was executed during this message (if any) */
  workflow?: ActiveWorkflow;
  /** Sub-agents that were spawned during this message */
  subAgents?: ActiveSubAgent[];
  /** System hooks that were injected during this turn */
  systemHooks?: string[];
  /** Input tokens used for this message (if available) */
  inputTokens?: number;
  /** Output tokens used for this message (if available) */
  outputTokens?: number;
  /** Context compaction result (if this message represents a compaction event) */
  compaction?: CompactionResult;
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
  /** Accumulated streaming output for real-time display (for run_command) */
  streamingOutput?: string;
}

/** Streaming block types for interleaved text and tool calls */
export type StreamingBlock =
  | { type: "text"; content: string }
  | { type: "tool"; toolCall: ActiveToolCall }
  | { type: "udiff_result"; response: string; durationMs: number }
  | { type: "system_hooks"; hooks: string[] };

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
  parentRequestId: string;
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

interface QbitState extends ContextSlice, GitSlice, NotificationSlice {
  // Sessions
  sessions: Record<string, Session>;
  activeSessionId: string | null;

  // AI configuration
  aiConfig: AiConfig;

  // Unified timeline - single source of truth for all blocks
  timelines: Record<string, UnifiedBlock[]>;

  // Terminal state
  pendingCommand: Record<string, PendingCommand | null>;
  lastSentCommand: Record<string, string | null>;

  // Agent state
  agentStreaming: Record<string, string>;
  streamingBlocks: Record<string, StreamingBlock[]>; // Interleaved text and tool blocks
  streamingTextOffset: Record<string, number>; // Tracks how much text has been assigned to blocks
  agentInitialized: Record<string, boolean>;
  isAgentThinking: Record<string, boolean>; // True when waiting for first content from agent
  isAgentResponding: Record<string, boolean>; // True when agent is actively responding (from started to completed)
  pendingToolApproval: Record<string, ToolCall | null>;
  processedToolRequests: Set<string>; // Track processed request IDs to prevent duplicates
  activeToolCalls: Record<string, ActiveToolCall[]>; // Tool calls currently in progress per session

  // Extended thinking state (for models like Opus 4.5)
  thinkingContent: Record<string, string>; // Accumulated thinking content per session
  isThinkingExpanded: Record<string, boolean>; // Whether thinking section is expanded

  // Workflow state
  activeWorkflows: Record<string, ActiveWorkflow | null>; // Active workflow per session
  workflowHistory: Record<string, ActiveWorkflow[]>; // Completed workflows per session

  // Sub-agent state
  activeSubAgents: Record<string, ActiveSubAgent[]>; // Active sub-agents per session

  // Terminal clear request (incremented to trigger clear)
  terminalClearRequest: Record<string, number>;

  // Pane layouts for multi-pane support (keyed by tab's root session ID)
  tabLayouts: Record<string, TabLayout>;

  // Session actions
  addSession: (session: Session, options?: { isPaneSession?: boolean }) => void;
  removeSession: (sessionId: string) => void;
  setActiveSession: (sessionId: string) => void;
  updateWorkingDirectory: (sessionId: string, path: string) => void;
  updateVirtualEnv: (sessionId: string, name: string | null) => void;
  updateGitBranch: (sessionId: string, branch: string | null) => void;
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
  setPendingOutput: (sessionId: string, output: string) => void;
  toggleBlockCollapse: (blockId: string) => void;
  setLastSentCommand: (sessionId: string, command: string | null) => void;
  clearBlocks: (sessionId: string) => void;
  requestTerminalClear: (sessionId: string) => void;

  // Agent actions
  addAgentMessage: (sessionId: string, message: AgentMessage) => void;
  updateAgentStreaming: (sessionId: string, content: string) => void;
  clearAgentStreaming: (sessionId: string) => void;
  setAgentInitialized: (sessionId: string, initialized: boolean) => void;
  setAgentThinking: (sessionId: string, thinking: boolean) => void;
  setAgentResponding: (sessionId: string, responding: boolean) => void;
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
  addStreamingSystemHooksBlock: (sessionId: string, hooks: string[]) => void;
  /** Append streaming output chunk to a running tool call (for run_command) */
  appendToolStreamingOutput: (sessionId: string, toolId: string, chunk: string) => void;

  // Thinking content actions
  appendThinkingContent: (sessionId: string, content: string) => void;
  clearThinkingContent: (sessionId: string) => void;
  setThinkingExpanded: (sessionId: string, expanded: boolean) => void;

  // Timeline actions
  addSystemHookBlock: (sessionId: string, hooks: string[]) => void;
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
    agent: {
      agentId: string;
      agentName: string;
      parentRequestId: string;
      task: string;
      depth: number;
    }
  ) => void;
  addSubAgentToolCall: (
    sessionId: string,
    parentRequestId: string,
    toolCall: { id: string; name: string; args: Record<string, unknown> }
  ) => void;
  completeSubAgentToolCall: (
    sessionId: string,
    parentRequestId: string,
    toolId: string,
    success: boolean,
    result?: unknown
  ) => void;
  completeSubAgent: (
    sessionId: string,
    parentRequestId: string,
    result: { response: string; durationMs: number }
  ) => void;
  failSubAgent: (sessionId: string, parentRequestId: string, error: string) => void;
  clearActiveSubAgents: (sessionId: string) => void;

  // AI config actions
  setAiConfig: (config: Partial<AiConfig>) => void;
  // Per-session AI config actions
  setSessionAiConfig: (sessionId: string, config: Partial<AiConfig>) => void;
  getSessionAiConfig: (sessionId: string) => AiConfig | undefined;

  // Plan actions
  setPlan: (sessionId: string, plan: TaskPlan) => void;

  // Pane actions for multi-pane support
  splitPane: (
    tabId: string,
    paneId: PaneId,
    direction: SplitDirection,
    newPaneId: PaneId,
    newSessionId: string
  ) => void;
  closePane: (tabId: string, paneId: PaneId) => void;
  focusPane: (tabId: string, paneId: PaneId) => void;
  resizePane: (tabId: string, splitPaneId: PaneId, ratio: number) => void;
  navigatePane: (tabId: string, direction: "up" | "down" | "left" | "right") => void;
  /**
   * Open settings in a tab. If a settings tab already exists, focus it.
   * Otherwise, create a new settings tab.
   */
  openSettingsTab: () => void;
  /**
   * Get all session IDs belonging to a tab (root + all pane sessions).
   * Used by TabBar to perform backend cleanup before removing state.
   */
  getTabSessionIds: (tabId: string) => string[];
  /**
   * Remove all state for a tab and its panes (frontend only).
   * Caller is responsible for backend cleanup (PTY/AI) before calling this.
   */
  closeTab: (tabId: string) => void;
}

export const useStore = create<QbitState>()(
  devtools(
    immer((set, get, _store) => ({
      // Slices
      ...createContextSlice(set, get),
      ...createGitSlice(set, get),
      ...createNotificationSlice(set, get),

      // Core state
      sessions: {},
      activeSessionId: null,
      aiConfig: {
        provider: "",
        model: "",
        status: "disconnected" as AiStatus,
      },
      timelines: {},
      pendingCommand: {},
      lastSentCommand: {},
      agentStreaming: {},
      streamingBlocks: {},
      streamingTextOffset: {},
      agentInitialized: {},
      isAgentThinking: {},
      isAgentResponding: {},
      pendingToolApproval: {},
      processedToolRequests: new Set<string>(),
      activeToolCalls: {},
      thinkingContent: {},
      isThinkingExpanded: {},
      activeWorkflows: {},
      tabLayouts: {},
      workflowHistory: {},
      activeSubAgents: {},
      terminalClearRequest: {},

      addSession: (session, options) =>
        set((state) => {
          const isPaneSession = options?.isPaneSession ?? false;

          state.sessions[session.id] = {
            ...session,
            inputMode: session.inputMode ?? "terminal", // Default to terminal mode
          };

          // Only set as active and create tab layout for new tabs, not pane sessions
          if (!isPaneSession) {
            state.activeSessionId = session.id;
          }

          state.timelines[session.id] = [];
          state.pendingCommand[session.id] = null;
          state.lastSentCommand[session.id] = null;
          state.agentStreaming[session.id] = "";
          state.streamingBlocks[session.id] = [];
          state.streamingTextOffset[session.id] = 0;
          state.agentInitialized[session.id] = false;
          state.isAgentThinking[session.id] = false;
          state.isAgentResponding[session.id] = false;
          state.pendingToolApproval[session.id] = null;
          state.activeToolCalls[session.id] = [];
          state.thinkingContent[session.id] = "";
          state.isThinkingExpanded[session.id] = false;
          state.activeWorkflows[session.id] = null;
          state.workflowHistory[session.id] = [];
          state.activeSubAgents[session.id] = [];
          // Initialize context metrics with default values
          state.contextMetrics[session.id] = {
            utilization: 0,
            usedTokens: 0,
            maxTokens: 0,
            isWarning: false,
          };
          // Initialize compaction state
          state.compactionCount[session.id] = 0;
          state.isCompacting[session.id] = false;
          state.isSessionDead[session.id] = false;
          state.compactionError[session.id] = null;
          state.gitStatus[session.id] = null;
          // Start with loading true so git badge shows loading spinner immediately
          state.gitStatusLoading[session.id] = true;
          state.gitCommitMessage[session.id] = "";

          // Only initialize pane layout for new tabs, not pane sessions
          // Pane sessions are added to an existing tab's layout via splitPane
          if (!isPaneSession) {
            state.tabLayouts[session.id] = {
              root: { type: "leaf", id: session.id, sessionId: session.id },
              focusedPaneId: session.id,
            };
          }
        }),

      removeSession: (sessionId) => {
        // Dispose terminal instance (outside state update to avoid side effects in Immer)
        TerminalInstanceManager.dispose(sessionId);

        // Clean up AI event sequence tracking to prevent memory leak
        import("@/hooks/useAiEvents").then(({ resetSessionSequence }) => {
          resetSessionSequence(sessionId);
        });

        set((state) => {
          delete state.sessions[sessionId];
          delete state.timelines[sessionId];
          delete state.pendingCommand[sessionId];
          delete state.lastSentCommand[sessionId];
          delete state.agentStreaming[sessionId];
          delete state.streamingBlocks[sessionId];
          delete state.streamingTextOffset[sessionId];
          delete state.agentInitialized[sessionId];
          delete state.isAgentThinking[sessionId];
          delete state.isAgentResponding[sessionId];
          delete state.pendingToolApproval[sessionId];
          delete state.activeToolCalls[sessionId];
          delete state.thinkingContent[sessionId];
          delete state.isThinkingExpanded[sessionId];
          delete state.gitStatus[sessionId];
          delete state.gitStatusLoading[sessionId];
          delete state.gitCommitMessage[sessionId];
          delete state.contextMetrics[sessionId];
          delete state.compactionCount[sessionId];
          delete state.isCompacting[sessionId];
          delete state.isSessionDead[sessionId];
          delete state.compactionError[sessionId];
          // Clean up tab layout if this is a tab's root session
          delete state.tabLayouts[sessionId];

          if (state.activeSessionId === sessionId) {
            const remaining = Object.keys(state.sessions);
            state.activeSessionId = remaining[0] ?? null;
          }
        });
      },

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

      updateVirtualEnv: (sessionId, name) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].virtualEnv = name;
          }
        }),

      updateGitBranch: (sessionId, branch) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].gitBranch = branch;
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
            logger.info("[store] setRenderMode:", {
              sessionId,
              from: state.sessions[sessionId].renderMode,
              to: mode,
            });
            state.sessions[sessionId].renderMode = mode;
          }
        }),

      handlePromptStart: (sessionId) =>
        set((state) => {
          // Finalize any pending command without exit code
          const pending = state.pendingCommand[sessionId];
          if (pending?.command) {
            const session = state.sessions[sessionId];
            // Use the session's CURRENT working directory (at command end), not the one
            // captured at command start. This ensures that for commands like "cd foo && ls",
            // file paths in the output are resolved relative to the new directory.
            const currentWorkingDir = session?.workingDirectory || pending.workingDirectory;
            const blockId = crypto.randomUUID();
            const block: CommandBlock = {
              id: blockId,
              sessionId,
              command: pending.command,
              output: pending.output,
              exitCode: null,
              startTime: pending.startTime,
              durationMs: null,
              workingDirectory: currentWorkingDir,
              isCollapsed: false,
            };

            // Push to unified timeline (Single Source of Truth)
            if (!state.timelines[sessionId]) {
              state.timelines[sessionId] = [];
            }
            state.timelines[sessionId].push({
              id: blockId,
              type: "command",
              // Use completion time for ordering in the unified timeline.
              timestamp: new Date().toISOString(),
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
          const effectiveCommand = command || state.lastSentCommand[sessionId] || null;
          state.pendingCommand[sessionId] = {
            command: effectiveCommand,
            output: "",
            startTime: new Date().toISOString(),
            workingDirectory: session?.workingDirectory || "",
          };
          state.lastSentCommand[sessionId] = null;
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
              // Use the session's CURRENT working directory (at command end), not the one
              // captured at command start. This ensures that for commands like "cd foo && ls",
              // file paths in the output are resolved relative to the new directory.
              const currentWorkingDir = session?.workingDirectory || pending.workingDirectory;
              const block: CommandBlock = {
                id: blockId,
                sessionId,
                command: pending.command,
                output: pending.output,
                exitCode,
                startTime: pending.startTime,
                durationMs: Date.now() - new Date(pending.startTime).getTime(),
                workingDirectory: currentWorkingDir,
                isCollapsed: false,
              };

              // Push to unified timeline (Single Source of Truth)
              if (!state.timelines[sessionId]) {
                state.timelines[sessionId] = [];
              }
              state.timelines[sessionId].push({
                id: blockId,
                type: "command",
                // Use completion time for ordering in the unified timeline.
                timestamp: new Date().toISOString(),
                data: block,
              });
            }

            state.pendingCommand[sessionId] = null;
          }
        }),

      appendOutput: (sessionId, data) =>
        set((state) => {
          let pending = state.pendingCommand[sessionId];
          // Auto-create pendingCommand if it doesn't exist (fallback for missing command_start)
          // This allows showing output even when OSC 133 shell integration isn't working
          if (!pending) {
            const session = state.sessions[sessionId];
            pending = {
              command: null, // Will show as "Running..." in the UI
              output: "",
              startTime: new Date().toISOString(),
              workingDirectory: session?.workingDirectory || "",
            };
            state.pendingCommand[sessionId] = pending;
          }
          pending.output += data;
        }),

      setPendingOutput: (sessionId, output) =>
        set((state) => {
          const pending = state.pendingCommand[sessionId];
          if (pending) {
            pending.output = output;
          }
        }),

      toggleBlockCollapse: (blockId) =>
        set((state) => {
          // Update in unified timeline
          for (const timeline of Object.values(state.timelines)) {
            const unifiedBlock = timeline.find((b) => b.type === "command" && b.id === blockId);
            if (unifiedBlock && unifiedBlock.type === "command") {
              unifiedBlock.data.isCollapsed = !unifiedBlock.data.isCollapsed;
              break;
            }
          }
        }),

      setLastSentCommand: (sessionId, command) =>
        set((state) => {
          state.lastSentCommand[sessionId] = command;
        }),

      clearBlocks: (sessionId) =>
        set((state) => {
          // Clear command blocks from timeline
          const timeline = state.timelines[sessionId];
          if (timeline) {
            state.timelines[sessionId] = timeline.filter((block) => block.type !== "command");
          }
          state.pendingCommand[sessionId] = null;
        }),

      requestTerminalClear: (sessionId) =>
        set((state) => {
          state.terminalClearRequest[sessionId] = (state.terminalClearRequest[sessionId] ?? 0) + 1;
        }),

      // Agent actions
      addAgentMessage: (sessionId, message) =>
        set((state) => {
          // Push to unified timeline (Single Source of Truth)
          if (!state.timelines[sessionId]) {
            state.timelines[sessionId] = [];
          }
          state.timelines[sessionId].push({
            id: message.id,
            type: "agent_message",
            timestamp: message.timestamp,
            data: message,
          });

          // Accumulate token usage for the session if available (input/output separately)
          if (message.inputTokens || message.outputTokens) {
            const current = state.sessionTokenUsage[sessionId] ?? { input: 0, output: 0 };
            state.sessionTokenUsage[sessionId] = {
              input: current.input + (message.inputTokens ?? 0),
              output: current.output + (message.outputTokens ?? 0),
            };
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

      setAgentResponding: (sessionId, responding) =>
        set((state) => {
          state.isAgentResponding[sessionId] = responding;
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
        return get().processedToolRequests.has(requestId);
      },

      updateToolCallStatus: (sessionId, toolId, status, result) =>
        set((state) => {
          // Update tool call status in timeline (Single Source of Truth)
          const timeline = state.timelines[sessionId];
          if (timeline) {
            for (const block of timeline) {
              if (block.type === "agent_message") {
                const tool = block.data.toolCalls?.find((t) => t.id === toolId);
                if (tool) {
                  tool.status = status;
                  if (result !== undefined) tool.result = result;
                  return;
                }
              }
            }
          }
        }),

      clearAgentMessages: (sessionId) =>
        set((state) => {
          // Clear agent messages from timeline
          const timeline = state.timelines[sessionId];
          if (timeline) {
            state.timelines[sessionId] = timeline.filter((block) => block.type !== "agent_message");
          }
          state.agentStreaming[sessionId] = "";
        }),

      restoreAgentMessages: (sessionId, messages) =>
        set((state) => {
          state.agentStreaming[sessionId] = "";
          // Replace the timeline with restored messages (Single Source of Truth)
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

      addStreamingSystemHooksBlock: (sessionId, hooks) =>
        set((state) => {
          if (!state.streamingBlocks[sessionId]) {
            state.streamingBlocks[sessionId] = [];
          }
          state.streamingBlocks[sessionId].push({
            type: "system_hooks",
            hooks,
          });
        }),

      appendToolStreamingOutput: (sessionId, toolId, chunk) =>
        set((state) => {
          // Update in activeToolCalls
          const tools = state.activeToolCalls[sessionId];
          if (tools) {
            const toolIndex = tools.findIndex((t) => t.id === toolId);
            if (toolIndex !== -1) {
              // Create new object to ensure reference change for React re-render
              tools[toolIndex] = {
                ...tools[toolIndex],
                streamingOutput: (tools[toolIndex].streamingOutput ?? "") + chunk,
              };
            }
          }
          // Also update in streamingBlocks
          const blocks = state.streamingBlocks[sessionId];
          if (blocks) {
            for (let i = 0; i < blocks.length; i++) {
              const block = blocks[i];
              if (block.type === "tool" && block.toolCall.id === toolId) {
                // Create new block and toolCall objects to ensure reference change
                blocks[i] = {
                  ...block,
                  toolCall: {
                    ...block.toolCall,
                    streamingOutput: (block.toolCall.streamingOutput ?? "") + chunk,
                  },
                };
                break;
              }
            }
          }
        }),

      // Thinking content actions
      appendThinkingContent: (sessionId, content) =>
        set((state) => {
          if (!state.thinkingContent[sessionId]) {
            state.thinkingContent[sessionId] = "";
          }
          state.thinkingContent[sessionId] += content;
        }),

      clearThinkingContent: (sessionId) =>
        set((state) => {
          state.thinkingContent[sessionId] = "";
        }),

      setThinkingExpanded: (sessionId, expanded) =>
        set((state) => {
          state.isThinkingExpanded[sessionId] = expanded;
        }),

      // Timeline actions
      addSystemHookBlock: (sessionId, hooks) =>
        set((state) => {
          if (!state.timelines[sessionId]) {
            state.timelines[sessionId] = [];
          }
          state.timelines[sessionId].push({
            id: crypto.randomUUID(),
            type: "system_hook",
            timestamp: new Date().toISOString(),
            data: { hooks },
          });
        }),

      clearTimeline: (sessionId) =>
        set((state) => {
          state.timelines[sessionId] = [];
          state.pendingCommand[sessionId] = null;
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
            parentRequestId: agent.parentRequestId,
            task: agent.task,
            depth: agent.depth,
            status: "running",
            toolCalls: [],
            startedAt: new Date().toISOString(),
          });
        }),

      addSubAgentToolCall: (sessionId, parentRequestId, toolCall) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.parentRequestId === parentRequestId);
          if (agent) {
            agent.toolCalls.push({
              ...toolCall,
              status: "running",
              startedAt: new Date().toISOString(),
            });
          }
        }),

      completeSubAgentToolCall: (sessionId, parentRequestId, toolId, success, result) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.parentRequestId === parentRequestId);
          if (agent) {
            const tool = agent.toolCalls.find((t) => t.id === toolId);
            if (tool) {
              tool.status = success ? "completed" : "error";
              tool.result = result;
              tool.completedAt = new Date().toISOString();
            }
          }
        }),

      completeSubAgent: (sessionId, parentRequestId, result) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.parentRequestId === parentRequestId);
          if (agent) {
            agent.status = "completed";
            agent.response = result.response;
            agent.durationMs = result.durationMs;
            agent.completedAt = new Date().toISOString();
          }
        }),

      failSubAgent: (sessionId, parentRequestId, error) =>
        set((state) => {
          const agents = state.activeSubAgents[sessionId];
          if (!agents) return;

          const agent = agents.find((a) => a.parentRequestId === parentRequestId);
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
        const session = get().sessions[sessionId];
        return session?.aiConfig;
      },

      // Plan actions
      setPlan: (sessionId, plan) =>
        set((state) => {
          if (state.sessions[sessionId]) {
            state.sessions[sessionId].plan = plan;
          }
        }),

      // Pane actions for multi-pane support
      splitPane: (tabId, paneId, direction, newPaneId, newSessionId) =>
        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) return;

          // Prevent splitting for non-terminal tabs (e.g., settings)
          const rootSession = state.sessions[tabId];
          const tabType = rootSession?.tabType ?? "terminal";
          if (tabType !== "terminal") {
            logger.warn(`[store] splitPane: Cannot split ${tabType} tabs`);
            return;
          }

          // Check pane limit (max 4 panes per tab)
          const currentCount = countLeafPanes(layout.root);
          if (currentCount >= 4) {
            logger.warn("[store] splitPane: Maximum pane limit (4) reached");
            return;
          }

          // Split the pane
          state.tabLayouts[tabId].root = splitPaneNode(
            layout.root,
            paneId,
            direction,
            newPaneId,
            newSessionId
          );

          // Focus the new pane (but keep activeSessionId pointing to the tab's root)
          // Note: activeSessionId identifies the TAB, not the focused pane within it.
          // The focused pane's session can be retrieved via useFocusedSessionId().
          state.tabLayouts[tabId].focusedPaneId = newPaneId;
        }),

      closePane: (tabId, paneId) => {
        // Get the session ID before state update (to dispose terminal outside Immer)
        const currentState = get();
        const layout = currentState.tabLayouts[tabId];
        if (!layout) return;

        const paneNode = findPaneById(layout.root, paneId);
        if (!paneNode || paneNode.type !== "leaf") return;

        const sessionIdToRemove = paneNode.sessionId;

        // Dispose terminal instance (outside state update to avoid side effects in Immer)
        TerminalInstanceManager.dispose(sessionIdToRemove);

        // Clean up AI event sequence tracking to prevent memory leak
        import("@/hooks/useAiEvents").then(({ resetSessionSequence }) => {
          resetSessionSequence(sessionIdToRemove);
        });

        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) return;

          // Remove pane from tree
          const newRoot = removePaneNode(layout.root, paneId);

          if (newRoot === null) {
            // Last pane in tab - remove the entire tab
            // Note: Session cleanup should be handled by the caller
            delete state.tabLayouts[tabId];
            return;
          }

          // Update tree
          state.tabLayouts[tabId].root = newRoot;

          // Update focus to sibling or first available pane
          // Note: activeSessionId stays as the tab's root session ID
          if (layout.focusedPaneId === paneId) {
            const newFocusId = getFirstLeafPane(newRoot);
            state.tabLayouts[tabId].focusedPaneId = newFocusId;
          }

          // Clean up the closed session's state
          delete state.sessions[sessionIdToRemove];
          delete state.timelines[sessionIdToRemove];
          delete state.pendingCommand[sessionIdToRemove];
          delete state.lastSentCommand[sessionIdToRemove];
          delete state.agentStreaming[sessionIdToRemove];
          delete state.streamingBlocks[sessionIdToRemove];
          delete state.streamingTextOffset[sessionIdToRemove];
          delete state.agentInitialized[sessionIdToRemove];
          delete state.isAgentThinking[sessionIdToRemove];
          delete state.isAgentResponding[sessionIdToRemove];
          delete state.pendingToolApproval[sessionIdToRemove];
          delete state.activeToolCalls[sessionIdToRemove];
          delete state.thinkingContent[sessionIdToRemove];
          delete state.isThinkingExpanded[sessionIdToRemove];
          delete state.gitStatus[sessionIdToRemove];
          delete state.gitStatusLoading[sessionIdToRemove];
          delete state.gitCommitMessage[sessionIdToRemove];
          delete state.contextMetrics[sessionIdToRemove];
        });
      },

      focusPane: (tabId, paneId) =>
        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) return;

          const paneNode = findPaneById(layout.root, paneId);
          if (!paneNode || paneNode.type !== "leaf") return;

          // Only update focusedPaneId - activeSessionId stays as the tab's root session ID
          // The focused pane's session can be retrieved via useFocusedSessionId()
          state.tabLayouts[tabId].focusedPaneId = paneId;
        }),

      resizePane: (tabId, splitPaneId, ratio) =>
        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) return;

          state.tabLayouts[tabId].root = updatePaneRatio(layout.root, splitPaneId, ratio);
        }),

      navigatePane: (tabId, direction) =>
        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) return;

          const neighborId = getPaneNeighbor(layout.root, layout.focusedPaneId, direction);
          if (!neighborId) return;

          // Only update focusedPaneId - activeSessionId stays as the tab's root session ID
          state.tabLayouts[tabId].focusedPaneId = neighborId;
        }),

      openSettingsTab: () =>
        set((state) => {
          // Check if a settings tab already exists
          const existingSettingsTab = Object.values(state.sessions).find(
            (session) => session.tabType === "settings"
          );

          if (existingSettingsTab) {
            // Focus the existing settings tab
            state.activeSessionId = existingSettingsTab.id;
            return;
          }

          // Create a new settings tab with a unique ID
          const settingsId = `settings-${Date.now()}`;

          // Create minimal session for settings tab
          state.sessions[settingsId] = {
            id: settingsId,
            tabType: "settings",
            name: "Settings",
            workingDirectory: "",
            createdAt: new Date().toISOString(),
            mode: "terminal", // Not used for settings, but required by Session interface
          };

          // Set as active tab
          state.activeSessionId = settingsId;

          // Create tab layout (single pane, no splitting for settings)
          state.tabLayouts[settingsId] = {
            root: { type: "leaf", id: settingsId, sessionId: settingsId },
            focusedPaneId: settingsId,
          };
        }),

      getTabSessionIds: (tabId) => {
        const layout = get().tabLayouts[tabId];
        if (!layout) return [];
        return getAllLeafPanes(layout.root).map((pane) => pane.sessionId);
      },

      closeTab: (tabId) => {
        // Get session IDs before state update (for cleanup outside Immer)
        const currentState = get();
        const layout = currentState.tabLayouts[tabId];
        const sessionIdsToClean: string[] = [];

        if (!layout) {
          // No layout - backward compatibility
          sessionIdsToClean.push(tabId);
        } else {
          // Get all pane sessions in this tab
          const panes = getAllLeafPanes(layout.root);
          for (const pane of panes) {
            sessionIdsToClean.push(pane.sessionId);
          }
        }

        // Clean up outside Immer (terminal instances and AI event sequence tracking)
        for (const sessionId of sessionIdsToClean) {
          TerminalInstanceManager.dispose(sessionId);
        }

        // Clean up AI event sequence tracking to prevent memory leak
        import("@/hooks/useAiEvents").then(({ resetSessionSequence }) => {
          for (const sessionId of sessionIdsToClean) {
            resetSessionSequence(sessionId);
          }
        });

        set((state) => {
          const layout = state.tabLayouts[tabId];
          if (!layout) {
            // No layout - just remove the session directly (backward compatibility)
            delete state.sessions[tabId];
            delete state.timelines[tabId];
            delete state.pendingCommand[tabId];
            delete state.lastSentCommand[tabId];
            delete state.agentStreaming[tabId];
            delete state.streamingBlocks[tabId];
            delete state.streamingTextOffset[tabId];
            delete state.agentInitialized[tabId];
            delete state.isAgentThinking[tabId];
            delete state.isAgentResponding[tabId];
            delete state.pendingToolApproval[tabId];
            delete state.activeToolCalls[tabId];
            delete state.thinkingContent[tabId];
            delete state.isThinkingExpanded[tabId];
            delete state.contextMetrics[tabId];

            if (state.activeSessionId === tabId) {
              const remaining = Object.keys(state.sessions);
              state.activeSessionId = remaining[0] ?? null;
            }
            return;
          }

          // Get all pane sessions in this tab
          const panes = getAllLeafPanes(layout.root);

          // Remove state for each pane session
          for (const pane of panes) {
            const sessionId = pane.sessionId;
            delete state.sessions[sessionId];
            delete state.timelines[sessionId];
            delete state.pendingCommand[sessionId];
            delete state.lastSentCommand[sessionId];
            delete state.agentStreaming[sessionId];
            delete state.streamingBlocks[sessionId];
            delete state.streamingTextOffset[sessionId];
            delete state.agentInitialized[sessionId];
            delete state.isAgentThinking[sessionId];
            delete state.isAgentResponding[sessionId];
            delete state.pendingToolApproval[sessionId];
            delete state.activeToolCalls[sessionId];
            delete state.thinkingContent[sessionId];
            delete state.gitStatus[sessionId];
            delete state.gitStatusLoading[sessionId];
            delete state.gitCommitMessage[sessionId];
            delete state.isThinkingExpanded[sessionId];
            delete state.contextMetrics[sessionId];
          }

          // Remove the tab layout
          delete state.tabLayouts[tabId];

          // Update active session if needed
          if (state.activeSessionId === tabId) {
            // Find another tab's root session
            const remaining = Object.keys(state.tabLayouts);
            state.activeSessionId = remaining[0] ?? null;
          }
        });
      },
    })),
    { name: "qbit" }
  )
);

// Stable empty arrays to avoid re-render loops
// Import derived selectors for Single Source of Truth pattern
import { memoizedSelectAgentMessages, memoizedSelectCommandBlocks } from "@/lib/timeline/selectors";

// Selectors
export const useActiveSession = () =>
  useStore((state) => {
    const id = state.activeSessionId;
    return id ? state.sessions[id] : null;
  });

/**
 * Get command blocks for a session.
 * Derives from the unified timeline (Single Source of Truth).
 */
export const useSessionBlocks = (sessionId: string) =>
  useStore((state) => memoizedSelectCommandBlocks(sessionId, state.timelines[sessionId]));

export const useTerminalClearRequest = (sessionId: string) =>
  useStore((state) => state.terminalClearRequest[sessionId] ?? 0);

export const usePendingCommand = (sessionId: string) =>
  useStore((state) => state.pendingCommand[sessionId]);

export const useSessionMode = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.mode ?? "terminal");

/**
 * Get agent messages for a session.
 * Derives from the unified timeline (Single Source of Truth).
 */
export const useAgentMessages = (sessionId: string) =>
  useStore((state) => memoizedSelectAgentMessages(sessionId, state.timelines[sessionId]));

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

export const useGitBranch = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.gitBranch ?? null);

export const useGitStatus = (sessionId: string) =>
  useStore((state) => state.gitStatus[sessionId] ?? null);
export const useGitStatusLoading = (sessionId: string) =>
  useStore((state) => state.gitStatusLoading[sessionId] ?? false);
export const useGitCommitMessage = (sessionId: string) =>
  useStore((state) => state.gitCommitMessage[sessionId] ?? "");

// Active tool calls selector
const EMPTY_TOOL_CALLS: ActiveToolCall[] = [];

export const useActiveToolCalls = (sessionId: string) =>
  useStore((state) => state.activeToolCalls[sessionId] ?? EMPTY_TOOL_CALLS);

// Streaming blocks selector
const EMPTY_STREAMING_BLOCKS: StreamingBlock[] = [];

export const useStreamingBlocks = (sessionId: string) =>
  useStore((state) => state.streamingBlocks[sessionId] ?? EMPTY_STREAMING_BLOCKS);

// Streaming text length selector (for auto-scroll triggers)
export const useStreamingTextLength = (sessionId: string) =>
  useStore((state) => state.agentStreaming[sessionId]?.length ?? 0);

// AI config selector (global - for backwards compatibility)
export const useAiConfig = () => useStore((state) => state.aiConfig);

// Per-session AI config selector
export const useSessionAiConfig = (sessionId: string) =>
  useStore((state) => state.sessions[sessionId]?.aiConfig);

// Agent thinking selector
export const useIsAgentThinking = (sessionId: string) =>
  useStore((state) => state.isAgentThinking[sessionId] ?? false);

// Agent responding selector (true when agent is actively responding, from started to completed)
export const useIsAgentResponding = (sessionId: string) =>
  useStore((state) => state.isAgentResponding[sessionId] ?? false);

// Extended thinking content selectors
export const useThinkingContent = (sessionId: string) =>
  useStore((state) => state.thinkingContent[sessionId] ?? "");

export const useIsThinkingExpanded = (sessionId: string) =>
  useStore((state) => state.isThinkingExpanded[sessionId] ?? false);

// Notification selectors (using slice selectors)
export const useNotifications = () => useStore(selectNotifications);

export const useUnreadNotificationCount = () => useStore(selectUnreadNotificationCount);

export const useNotificationsExpanded = () => useStore(selectNotificationsExpanded);

// Context metrics selector (uses slice selector)
export const useContextMetrics = (sessionId: string) =>
  useStore((state) => selectContextMetrics(state, sessionId));

// Pane layout selectors
export const useTabLayout = (tabId: string | null) =>
  useStore((state) => (tabId ? state.tabLayouts[tabId] : null));

export const useFocusedPaneId = (tabId: string | null) =>
  useStore((state) => (tabId ? state.tabLayouts[tabId]?.focusedPaneId : null));

/**
 * Get the session ID of the currently focused pane.
 * Falls back to tabId if no layout exists (backward compatibility).
 */
export const useFocusedSessionId = (tabId: string | null) =>
  useStore((state) => {
    if (!tabId) return null;
    const layout = state.tabLayouts[tabId];
    if (!layout) return tabId; // Fallback to tab session for backward compatibility
    const pane = findPaneById(layout.root, layout.focusedPaneId);
    return pane?.type === "leaf" ? pane.sessionId : tabId;
  });

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
    logger.warn("Failed to clear backend conversation history:", error);
  }
}

// Helper function to restore a previous session (both frontend and backend)
export async function restoreSession(sessionId: string, identifier: string): Promise<void> {
  const aiModule = await import("@/lib/ai");
  const { loadAiSession, restoreAiSession, initAiSession, buildProviderConfig } = aiModule;
  const { getSettings } = await import("@/lib/settings");

  // First, load the session data from disk (doesn't require AI bridge)
  const session = await loadAiSession(identifier);
  if (!session) {
    throw new Error(`Session '${identifier}' not found`);
  }

  // Get current settings and use the user's current default provider
  // (not the session's original provider - conversation history is provider-agnostic)
  const settings = await getSettings();
  const workspace = session.workspace_path;

  logger.info(
    `Restoring session (original: ${session.provider}/${session.model}, ` +
      `using current: ${settings.ai.default_provider}/${settings.ai.default_model})`
  );

  // Build config using the user's current default provider
  const config = await buildProviderConfig(settings, workspace);

  // Initialize the AI bridge BEFORE restoring messages
  await initAiSession(sessionId, config);

  // Update the store's AI config for this session with the current provider
  useStore.getState().setSessionAiConfig(sessionId, {
    provider: settings.ai.default_provider,
    model: settings.ai.default_model,
    status: "ready",
  });

  // Now restore the backend conversation history (bridge is initialized)
  await restoreAiSession(sessionId, identifier);

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
}

// Expose store for testing in development
if (import.meta.env.DEV) {
  (window as unknown as { __QBIT_STORE__: typeof useStore }).__QBIT_STORE__ = useStore;
}
