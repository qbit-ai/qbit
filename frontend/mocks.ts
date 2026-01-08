/**
 * Tauri IPC Mock Adapter
 *
 * This module provides mock implementations for all Tauri IPC commands and events,
 * enabling browser-only development without the Rust backend.
 *
 * Usage: This file is automatically loaded in browser environments
 * (when window.__TAURI_INTERNALS__ is undefined).
 *
 * Events can be emitted using the exported helper functions:
 * - emitTerminalOutput(sessionId, data)
 * - emitCommandBlock(block)
 * - emitDirectoryChanged(sessionId, directory)
 * - emitSessionEnded(sessionId)
 * - emitAiEvent(event)
 */

import * as tauriEvent from "@tauri-apps/api/event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";

// =============================================================================
// Browser Mode Flag
// =============================================================================

// This flag is set to true when mocks are initialized.
// It's exposed globally so App.tsx can check it even after mockWindows()
// creates __TAURI_INTERNALS__.
declare global {
  interface Window {
    __MOCK_BROWSER_MODE__?: boolean;
  }
}

/**
 * Check if we're running in mock browser mode.
 * Use this instead of checking __TAURI_INTERNALS__ in components.
 */
export function isMockBrowserMode(): boolean {
  return window.__MOCK_BROWSER_MODE__ === true;
}

// =============================================================================
// Event System (custom implementation for browser mode)
// =============================================================================

// Auto-incrementing handler ID
let nextHandlerId = 1;

// Map of event name -> array of { handlerId, callback }
const mockEventListeners: Map<
  string,
  Array<{ handlerId: number; callback: (event: { event: string; payload: unknown }) => void }>
> = new Map();

// Map of handler ID -> { event, callback } (for unlisten)
const handlerToEvent: Map<number, string> = new Map();

/**
 * Register an event listener with its callback
 */
export function mockRegisterListener(
  event: string,
  callback: (event: { event: string; payload: unknown }) => void
): number {
  const handlerId = nextHandlerId++;
  if (!mockEventListeners.has(event)) {
    mockEventListeners.set(event, []);
  }
  mockEventListeners.get(event)?.push({ handlerId, callback });
  handlerToEvent.set(handlerId, event);
  console.log(`[Mock Events] Registered listener for "${event}" (handler: ${handlerId})`);
  return handlerId;
}

/**
 * Unregister an event listener by handler ID
 */
export function mockUnregisterListener(handlerId: number): void {
  const eventName = handlerToEvent.get(handlerId);
  if (!eventName) return;

  handlerToEvent.delete(handlerId);
  const listeners = mockEventListeners.get(eventName);
  if (listeners) {
    const filtered = listeners.filter((l) => l.handlerId !== handlerId);
    mockEventListeners.set(eventName, filtered);
    console.log(`[Mock Events] Unregistered listener for "${eventName}" (handler: ${handlerId})`);
  }
}

/**
 * Dispatch an event to all registered listeners
 */
function dispatchMockEvent(eventName: string, payload: unknown): void {
  const listeners = mockEventListeners.get(eventName);
  if (listeners && listeners.length > 0) {
    console.log(
      `[Mock Events] Dispatching "${eventName}" to ${listeners.length} listener(s)`,
      payload
    );
    for (const { callback } of listeners) {
      try {
        callback({ event: eventName, payload });
      } catch (e) {
        console.error(`[Mock Events] Error in listener for "${eventName}":`, e);
      }
    }
  } else {
    console.log(`[Mock Events] No listeners for "${eventName}"`, payload);
  }
}

// =============================================================================
// Mock Data
// =============================================================================

// Mock PTY sessions
// Keep the first session id stable for MockDevTools presets.
let mockPtySessionCounter = 1;
const mockPtySessions: Record<
  string,
  { id: string; working_directory: string; rows: number; cols: number }
> = {
  "mock-session-001": {
    id: "mock-session-001",
    working_directory: "/home/user",
    rows: 24,
    cols: 80,
  },
};

// Mock AI state
let mockAiInitialized = false;
let mockConversationLength = 0;
let mockSessionPersistenceEnabled = true;

// Mock Git state (used by get_git_branch and git_status)
interface MockGitStatusSummary {
  branch: string | null;
  ahead: number;
  behind: number;
  entries: Array<unknown>;
  insertions: number;
  deletions: number;
}

let mockGitBranch: string | null = "main";
let mockGitStatus: MockGitStatusSummary = {
  branch: mockGitBranch,
  ahead: 0,
  behind: 0,
  entries: [],
  insertions: 0,
  deletions: 0,
};

export function setMockGitState(next: Partial<MockGitStatusSummary>): void {
  if ("branch" in next) {
    mockGitBranch = next.branch ?? null;
  }

  mockGitStatus = {
    ...mockGitStatus,
    ...next,
    branch: mockGitBranch,
  };
}

// Session-specific AI state (for per-tab isolation)
const mockSessionAiState: Map<
  string,
  { initialized: boolean; conversationLength: number; config?: unknown }
> = new Map();

// =============================================================================
// Parameter Validation Helper
// =============================================================================

/**
 * Validates that required parameters are present in the args object.
 * Throws an error (like Tauri would) if a required parameter is missing.
 *
 * @param cmd - The command name (for error messages)
 * @param args - The arguments object passed to the command
 * @param requiredParams - List of required parameter names (in camelCase, as sent from JS)
 */
function validateRequiredParams(cmd: string, args: unknown, requiredParams: string[]): void {
  const argsObj = args as Record<string, unknown> | undefined;

  for (const param of requiredParams) {
    if (!argsObj || !(param in argsObj) || argsObj[param] === undefined) {
      const error = `invalid args \`${param}\` for command \`${cmd}\`: command ${cmd} missing required key ${param}`;
      console.error(`[Mock IPC] ${error}`);
      throw new Error(error);
    }
  }
}

// Mock tool definitions
const mockTools = [
  {
    name: "read_file",
    description: "Read the contents of a file",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "Path to the file" },
      },
      required: ["path"],
    },
  },
  {
    name: "write_file",
    description: "Write content to a file",
    parameters: {
      type: "object",
      properties: {
        path: { type: "string", description: "Path to the file" },
        content: { type: "string", description: "Content to write" },
      },
      required: ["path", "content"],
    },
  },
  {
    name: "run_command",
    description: "Execute a shell command",
    parameters: {
      type: "object",
      properties: {
        command: { type: "string", description: "Command to execute" },
      },
      required: ["command"],
    },
  },
];

// Mock workflows
const mockWorkflows = [
  { name: "code-review", description: "Review code changes and provide feedback" },
  { name: "test-generation", description: "Generate unit tests for code" },
  { name: "refactor", description: "Suggest code refactoring improvements" },
];

// Mock sub-agents
const mockSubAgents = [
  { id: "explorer", name: "Code Explorer", description: "Explores and understands codebases" },
  { id: "debugger", name: "Debug Assistant", description: "Helps debug issues" },
  { id: "documenter", name: "Documentation Writer", description: "Generates documentation" },
];

// Mock sessions
const mockSessions = [
  {
    identifier: "session-2024-01-15-001",
    path: "/home/user/.qbit/sessions/session-2024-01-15-001.json",
    workspace_label: "qbit",
    workspace_path: "/home/user/qbit",
    model: "claude-opus-4.5",
    provider: "anthropic_vertex",
    started_at: "2024-01-15T10:00:00Z",
    ended_at: "2024-01-15T11:30:00Z",
    total_messages: 24,
    distinct_tools: ["read_file", "write_file", "run_command"],
    first_prompt_preview: "Can you help me refactor the authentication module?",
    first_reply_preview: "I'll help you refactor the authentication module...",
  },
  {
    identifier: "session-2024-01-14-002",
    path: "/home/user/.qbit/sessions/session-2024-01-14-002.json",
    workspace_label: "qbit",
    workspace_path: "/home/user/qbit",
    model: "claude-opus-4.5",
    provider: "anthropic_vertex",
    started_at: "2024-01-14T14:00:00Z",
    ended_at: "2024-01-14T16:45:00Z",
    total_messages: 42,
    distinct_tools: ["read_file", "run_command"],
    first_prompt_preview: "Help me add unit tests for the PTY manager",
    first_reply_preview: "I'll help you add unit tests for the PTY manager...",
  },
];

// Mock approval patterns
const mockApprovalPatterns = [
  {
    tool_name: "read_file",
    total_requests: 50,
    approvals: 50,
    denials: 0,
    always_allow: true,
    last_updated: "2024-01-15T10:00:00Z",
    justifications: [],
  },
  {
    tool_name: "write_file",
    total_requests: 20,
    approvals: 18,
    denials: 2,
    always_allow: false,
    last_updated: "2024-01-15T09:30:00Z",
    justifications: ["Writing config file", "Updating source code"],
  },
  {
    tool_name: "run_command",
    total_requests: 30,
    approvals: 25,
    denials: 5,
    always_allow: false,
    last_updated: "2024-01-15T11:00:00Z",
    justifications: ["Running tests", "Building project"],
  },
];

// Mock HITL config
let mockHitlConfig = {
  always_allow: ["read_file"],
  always_require_approval: ["run_command"],
  pattern_learning_enabled: true,
  min_approvals: 3,
  approval_threshold: 0.8,
};

// Mock prompts
const mockPrompts = [
  { name: "review", path: "/home/user/.qbit/prompts/review.md", source: "global" as const },
  { name: "explain", path: "/home/user/.qbit/prompts/explain.md", source: "global" as const },
  { name: "project-context", path: ".qbit/prompts/project-context.md", source: "local" as const },
];

// Mock indexer state
let mockIndexerInitialized = false;
let mockIndexerWorkspace: string | null = null;
let mockIndexedFileCount = 0;

// Mock codebases state
interface MockCodebase {
  path: string;
  file_count: number;
  status: "synced" | "indexing" | "not_indexed" | "error";
  error?: string;
  memory_file?: string;
}

let mockCodebases: MockCodebase[] = [
  {
    path: "/home/user/projects/my-app",
    file_count: 150,
    status: "synced",
    memory_file: "CLAUDE.md",
  },
  {
    path: "/home/user/projects/backend-api",
    file_count: 89,
    status: "synced",
    memory_file: "AGENTS.md",
  },
];

// Mock settings state
// Mock per-project settings (stored in .qbit/project.toml)
const mockProjectSettings: {
  provider: string | null;
  model: string | null;
  agent_mode: string | null;
} = {
  provider: null,
  model: null,
  agent_mode: null,
};

let mockSettings = {
  version: 1,
  ai: {
    default_provider: "vertex_ai",
    default_model: "claude-opus-4-5@20251101",
    vertex_ai: {
      credentials_path: "/mock/path/to/credentials.json",
      project_id: "mock-project-id",
      location: "us-east5",
      show_in_selector: true,
    },
    openrouter: {
      api_key: "mock-openrouter-key",
      show_in_selector: true,
    },
    anthropic: {
      api_key: null,
      show_in_selector: true,
    },
    openai: {
      api_key: "mock-openai-key",
      base_url: null,
      show_in_selector: true,
    },
    ollama: {
      base_url: "http://localhost:11434",
      show_in_selector: true,
    },
    gemini: {
      api_key: null,
      show_in_selector: true,
    },
    groq: {
      api_key: null,
      show_in_selector: true,
    },
    xai: {
      api_key: null,
      show_in_selector: true,
    },
    zai: {
      api_key: null,
      use_coding_endpoint: true,
      show_in_selector: true,
    },
  },
  api_keys: {
    tavily: null,
    github: null,
  },
  ui: {
    theme: "dark",
    show_tips: true,
    hide_banner: false,
  },
  terminal: {
    shell: null,
    font_family: "JetBrains Mono",
    font_size: 14,
    scrollback: 10000,
  },
  agent: {
    session_persistence: true,
    session_retention_days: 30,
    pattern_learning: true,
    min_approvals_for_auto: 3,
    approval_threshold: 0.8,
  },
  mcp_servers: {},
  trust: {
    full_trust: [],
    read_only_trust: [],
    never_trust: [],
  },
  privacy: {
    usage_statistics: false,
    log_prompts: false,
  },
  advanced: {
    enable_experimental: false,
    log_level: "info",
  },
  sidecar: {
    enabled: true,
    synthesis_enabled: true,
    synthesis_backend: "template",
    synthesis_vertex: {
      project_id: null,
      location: null,
      model: "claude-sonnet-4-5-20250514",
      credentials_path: null,
    },
    synthesis_openai: {
      api_key: null,
      model: "gpt-4o-mini",
      base_url: null,
    },
    synthesis_grok: {
      api_key: null,
      model: "grok-2",
    },
    retention_days: 30,
    capture_tool_calls: true,
    capture_reasoning: true,
  },
};

// =============================================================================
// Event Types (matching backend events)
// =============================================================================

export interface TerminalOutputEvent {
  session_id: string;
  data: string;
}

// Command block events are lifecycle events, not full blocks
export interface CommandBlockEvent {
  session_id: string;
  command: string | null;
  exit_code: number | null;
  event_type: "prompt_start" | "prompt_end" | "command_start" | "command_end";
}

export interface DirectoryChangedEvent {
  session_id: string;
  path: string;
}

export interface SessionEndedEvent {
  session_id: string;
}

export type AiEventType =
  | { type: "started"; turn_id: string }
  | { type: "text_delta"; delta: string; accumulated: string }
  | { type: "tool_request"; tool_name: string; args: unknown; request_id: string }
  | {
      type: "tool_result";
      tool_name: string;
      result: unknown;
      success: boolean;
      request_id: string;
    }
  | {
      type: "completed";
      response: string;
      tokens_used?: number;
      duration_ms?: number;
      input_tokens?: number;
      output_tokens?: number;
    }
  | { type: "error"; message: string; error_type: string }
  | { type: "sub_agent_started"; agent_id: string; agent_name: string; task: string; depth: number }
  | {
      type: "sub_agent_tool_request";
      agent_id: string;
      tool_name: string;
      args: unknown;
      request_id: string;
    }
  | {
      type: "sub_agent_tool_result";
      agent_id: string;
      tool_name: string;
      result: unknown;
      success: boolean;
      request_id: string;
    }
  | { type: "sub_agent_completed"; agent_id: string; response: string; duration_ms: number }
  | { type: "sub_agent_error"; agent_id: string; error: string };

// =============================================================================
// Event Emitter Helpers
// =============================================================================

/**
 * Emit a terminal output event.
 * Use this to simulate terminal output in browser mode.
 */
export async function emitTerminalOutput(sessionId: string, data: string): Promise<void> {
  dispatchMockEvent("terminal_output", { session_id: sessionId, data });
}

/**
 * Emit a command block lifecycle event.
 * Use this to simulate command lifecycle events in browser mode.
 *
 * To simulate a full command execution, call in sequence:
 * 1. emitCommandBlockEvent(sessionId, "prompt_start")
 * 2. emitCommandBlockEvent(sessionId, "command_start", command)
 * 3. emitTerminalOutput(sessionId, output)  // The actual command output
 * 4. emitCommandBlockEvent(sessionId, "command_end", command, exitCode)
 * 5. emitCommandBlockEvent(sessionId, "prompt_end")
 */
export async function emitCommandBlockEvent(
  sessionId: string,
  eventType: CommandBlockEvent["event_type"],
  command: string | null = null,
  exitCode: number | null = null
): Promise<void> {
  dispatchMockEvent("command_block", {
    session_id: sessionId,
    command,
    exit_code: exitCode,
    event_type: eventType,
  });
}

/**
 * Helper to simulate a complete command execution with output.
 * This emits the proper sequence of events that the app expects.
 */
export async function simulateCommand(
  sessionId: string,
  command: string,
  output: string,
  exitCode: number = 0
): Promise<void> {
  // Start command
  await emitCommandBlockEvent(sessionId, "command_start", command);

  // Send output
  await emitTerminalOutput(sessionId, `$ ${command}\r\n`);
  await emitTerminalOutput(sessionId, output);
  if (!output.endsWith("\n")) {
    await emitTerminalOutput(sessionId, "\r\n");
  }

  // End command
  await emitCommandBlockEvent(sessionId, "command_end", command, exitCode);
}

/**
 * @deprecated Use emitCommandBlockEvent() or simulateCommand() instead.
 * This function signature doesn't match the actual event format.
 */
export async function emitCommandBlock(
  sessionId: string,
  command: string,
  output: string,
  exitCode: number | null = 0,
  _workingDirectory: string = "/home/user"
): Promise<void> {
  // Redirect to the proper simulation
  await simulateCommand(sessionId, command, output, exitCode ?? 0);
}

/**
 * Emit a directory changed event.
 * Use this to simulate directory changes in browser mode.
 */
export async function emitDirectoryChanged(sessionId: string, directory: string): Promise<void> {
  dispatchMockEvent("directory_changed", { session_id: sessionId, directory });
}

/**
 * Emit a session ended event.
 * Use this to simulate session termination in browser mode.
 */
export async function emitSessionEnded(sessionId: string): Promise<void> {
  dispatchMockEvent("session_ended", { session_id: sessionId });
}

/**
 * Emit an AI event.
 * Use this to simulate AI streaming responses in browser mode.
 */
export async function emitAiEvent(event: AiEventType): Promise<void> {
  dispatchMockEvent("ai-event", event);
}

/**
 * Simulate a complete AI response with streaming.
 * This emits started -> text_delta(s) -> completed events.
 */
export async function simulateAiResponse(response: string, delayMs: number = 50): Promise<void> {
  const turnId = `mock-turn-${Date.now()}`;

  // Emit started
  await emitAiEvent({ type: "started", turn_id: turnId });

  // Emit text deltas (word by word)
  const words = response.split(" ");
  let accumulated = "";
  for (const word of words) {
    const delta = accumulated ? ` ${word}` : word;
    accumulated += delta;
    await emitAiEvent({ type: "text_delta", delta, accumulated });
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }

  // Emit completed
  await emitAiEvent({
    type: "completed",
    response: accumulated,
    tokens_used: Math.floor(accumulated.length / 4),
    duration_ms: words.length * delayMs,
  });
}

/**
 * Simulate a sub-agent execution with tool calls.
 * This emits the proper sequence of sub-agent events.
 */
export async function simulateSubAgent(
  agentId: string,
  agentName: string,
  task: string,
  toolCalls: Array<{ name: string; args: unknown; result: unknown }>,
  response: string,
  delayMs: number = 20
): Promise<void> {
  // Emit sub-agent started
  await emitAiEvent({
    type: "sub_agent_started",
    agent_id: agentId,
    agent_name: agentName,
    task,
    depth: 1,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit tool calls
  for (const tool of toolCalls) {
    const requestId = `mock-req-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;

    await emitAiEvent({
      type: "sub_agent_tool_request",
      agent_id: agentId,
      tool_name: tool.name,
      args: tool.args,
      request_id: requestId,
    });
    await new Promise((resolve) => setTimeout(resolve, delayMs));

    await emitAiEvent({
      type: "sub_agent_tool_result",
      agent_id: agentId,
      tool_name: tool.name,
      result: tool.result,
      success: true,
      request_id: requestId,
    });
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  }

  // Emit sub-agent completed
  await emitAiEvent({
    type: "sub_agent_completed",
    agent_id: agentId,
    response,
    duration_ms: toolCalls.length * delayMs * 2 + 100,
  });
}

/**
 * Simulate an AI response that spawns a sub-agent.
 * This demonstrates the proper interleaving of sub-agent tool calls in the timeline.
 */
export async function simulateAiResponseWithSubAgent(
  subAgentName: string,
  subAgentTask: string,
  subAgentResponse: string,
  finalResponse: string,
  delayMs: number = 20
): Promise<void> {
  const turnId = `mock-turn-${Date.now()}`;
  const agentId = `mock-agent-${Date.now()}`;
  const subAgentToolRequestId = `mock-sub-req-${Date.now()}`;

  // Emit turn started
  await emitAiEvent({ type: "started", turn_id: turnId });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit sub-agent tool call (this creates the tool block in streamingBlocks)
  await emitAiEvent({
    type: "tool_request",
    tool_name: `sub_agent_${subAgentName.toLowerCase().replace(/\s+/g, "_")}`,
    args: { task: subAgentTask },
    request_id: subAgentToolRequestId,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit sub-agent started (this populates activeSubAgents)
  await emitAiEvent({
    type: "sub_agent_started",
    agent_id: agentId,
    agent_name: subAgentName,
    task: subAgentTask,
    depth: 1,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit some sub-agent tool calls
  const subToolReqId = `mock-sub-tool-${Date.now()}`;
  await emitAiEvent({
    type: "sub_agent_tool_request",
    agent_id: agentId,
    tool_name: "list_files",
    args: { path: "." },
    request_id: subToolReqId,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  await emitAiEvent({
    type: "sub_agent_tool_result",
    agent_id: agentId,
    tool_name: "list_files",
    result: ["file1.ts", "file2.ts"],
    success: true,
    request_id: subToolReqId,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit sub-agent completed
  await emitAiEvent({
    type: "sub_agent_completed",
    agent_id: agentId,
    response: subAgentResponse,
    duration_ms: 5000,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit sub-agent tool result (marks the tool call as completed)
  await emitAiEvent({
    type: "tool_result",
    tool_name: `sub_agent_${subAgentName.toLowerCase().replace(/\s+/g, "_")}`,
    result: subAgentResponse,
    success: true,
    request_id: subAgentToolRequestId,
  });
  await new Promise((resolve) => setTimeout(resolve, delayMs));

  // Emit final text response
  const words = finalResponse.split(" ");
  let accumulated = "";
  for (const word of words) {
    const delta = accumulated ? ` ${word}` : word;
    accumulated += delta;
    await emitAiEvent({ type: "text_delta", delta, accumulated });
    await new Promise((resolve) => setTimeout(resolve, delayMs / 2));
  }

  // Emit completed
  await emitAiEvent({
    type: "completed",
    response: accumulated,
    tokens_used: Math.floor(accumulated.length / 4),
    duration_ms: 6000,
    input_tokens: 100,
    output_tokens: 50,
  });
}

// =============================================================================
// Mock Settings Accessors (for e2e testing)
// =============================================================================

/**
 * Get the current mock settings.
 * Use this in e2e tests to verify settings state.
 */
export function getMockSettings(): typeof mockSettings {
  return structuredClone(mockSettings);
}

/**
 * Update mock settings.
 * Use this in e2e tests to set up specific test scenarios.
 */
export function setMockSettings(settings: Partial<typeof mockSettings>): void {
  mockSettings = { ...mockSettings, ...settings };
}

/**
 * Update a specific provider's visibility in mock settings.
 * This is a convenience function for e2e testing the provider toggle feature.
 */
export function setMockProviderVisibility(
  provider:
    | "vertex_ai"
    | "openrouter"
    | "anthropic"
    | "openai"
    | "ollama"
    | "gemini"
    | "groq"
    | "xai"
    | "zai",
  visible: boolean
): void {
  mockSettings.ai[provider].show_in_selector = visible;
}

// =============================================================================
// Setup Mock IPC
// =============================================================================

/**
 * Clean up mocks. Call this when unmounting or resetting.
 */
export function cleanupMocks(): void {
  clearMocks();
  console.log("[Mocks] Tauri mocks cleared");
}

export function setupMocks(): void {
  console.log("[Mocks] Setting up Tauri IPC mocks for browser development");

  // Set the browser mode flag BEFORE mockWindows creates __TAURI_INTERNALS__
  // This allows components to check isMockBrowserMode() after mocks are set up
  window.__MOCK_BROWSER_MODE__ = true;

  try {
    // Setup mock window context (required for Tauri internals)
    mockWindows("main");

    // Patch the Tauri event module's listen function to use our mock event system
    // ES module exports are read-only, so we use Object.defineProperty to override
    const originalListen = tauriEvent.listen;

    // Create our mock listen function
    const mockListen = async <T>(
      eventName: string,
      callback: (event: { event: string; payload: T }) => void
    ): Promise<() => void> => {
      console.log(`[Mock Events] listen("${eventName}") called`);

      // Register the callback with our mock event system
      const handlerId = mockRegisterListener(
        eventName,
        callback as (event: { event: string; payload: unknown }) => void
      );

      // Return an unlisten function
      return () => {
        mockUnregisterListener(handlerId);
      };
    };

    // Try to override the listen export using Object.defineProperty
    // Note: This usually fails because ES modules have read-only exports,
    // but we try anyway in case the bundler makes it writable
    try {
      Object.defineProperty(tauriEvent, "listen", {
        value: mockListen,
        writable: true,
        configurable: true,
      });
    } catch {
      // Expected to fail - we use the global fallback instead
      // Hooks check for window.__MOCK_LISTEN__ when in browser mode
    }

    // Store mock listen function globally as a fallback
    // Hooks can check for this when the module patch doesn't work
    (window as unknown as { __MOCK_LISTEN__?: typeof mockListen }).__MOCK_LISTEN__ = mockListen;

    // Expose mock event listeners for debugging in e2e tests
    (
      window as unknown as { __MOCK_EVENT_LISTENERS__?: typeof mockEventListeners }
    ).__MOCK_EVENT_LISTENERS__ = mockEventListeners;

    // Store reference to original for cleanup
    (
      window as unknown as { __MOCK_ORIGINAL_LISTEN__?: typeof originalListen }
    ).__MOCK_ORIGINAL_LISTEN__ = originalListen;

    // Expose mock event emitters globally for e2e testing
    (
      window as unknown as {
        __MOCK_EMIT_AI_EVENT__?: typeof emitAiEvent;
        __MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__?: typeof simulateAiResponseWithSubAgent;
        __MOCK_SIMULATE_AI_RESPONSE__?: typeof simulateAiResponse;
      }
    ).__MOCK_EMIT_AI_EVENT__ = emitAiEvent;
    (
      window as unknown as {
        __MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__?: typeof simulateAiResponseWithSubAgent;
      }
    ).__MOCK_SIMULATE_AI_RESPONSE_WITH_SUB_AGENT__ = simulateAiResponseWithSubAgent;
    (
      window as unknown as {
        __MOCK_SIMULATE_AI_RESPONSE__?: typeof simulateAiResponse;
      }
    ).__MOCK_SIMULATE_AI_RESPONSE__ = simulateAiResponse;

    // Expose command simulation functions for e2e testing
    (
      window as unknown as {
        __MOCK_SIMULATE_COMMAND__?: typeof simulateCommand;
        __MOCK_EMIT_COMMAND_BLOCK_EVENT__?: typeof emitCommandBlockEvent;
        __MOCK_EMIT_TERMINAL_OUTPUT__?: typeof emitTerminalOutput;
      }
    ).__MOCK_SIMULATE_COMMAND__ = simulateCommand;
    (
      window as unknown as {
        __MOCK_EMIT_COMMAND_BLOCK_EVENT__?: typeof emitCommandBlockEvent;
      }
    ).__MOCK_EMIT_COMMAND_BLOCK_EVENT__ = emitCommandBlockEvent;

    // Expose git state controls for e2e testing
    (
      window as unknown as {
        __MOCK_SET_GIT_STATE__?: typeof setMockGitState;
      }
    ).__MOCK_SET_GIT_STATE__ = setMockGitState;
    (
      window as unknown as {
        __MOCK_EMIT_TERMINAL_OUTPUT__?: typeof emitTerminalOutput;
      }
    ).__MOCK_EMIT_TERMINAL_OUTPUT__ = emitTerminalOutput;
  } catch (error) {
    console.error("[Mocks] Error during initial setup:", error);
  }

  mockIPC((cmd, args) => {
    console.log(`[Mock IPC] Command: ${cmd}`, args);

    switch (cmd) {
      // =========================================================================
      // PTY Commands
      // =========================================================================
      case "pty_create": {
        const payload = args as { workingDirectory?: string; rows?: number; cols?: number };
        // First create returns the stable id; subsequent creates get incrementing ids.
        const id =
          mockPtySessionCounter === 1
            ? "mock-session-001"
            : `mock-session-${String(mockPtySessionCounter).padStart(3, "0")}`;

        const session = {
          id,
          working_directory: payload.workingDirectory ?? "/home/user",
          rows: payload.rows ?? 24,
          cols: payload.cols ?? 80,
        };

        mockPtySessions[id] = session;
        mockPtySessionCounter += 1;
        return session;
      }

      case "pty_write":
        // Simulate writing to PTY - in real app this would send data to the terminal
        return undefined;

      case "pty_resize": {
        const resizePayload = args as { sessionId: string; rows: number; cols: number };
        const session = mockPtySessions[resizePayload.sessionId];
        if (session) {
          session.rows = resizePayload.rows;
          session.cols = resizePayload.cols;
        }
        return undefined;
      }

      case "pty_destroy":
        return undefined;

      case "pty_get_session": {
        const getPayload = args as { sessionId: string };
        return mockPtySessions[getPayload.sessionId] ?? null;
      }

      // =========================================================================
      // Shell Integration Commands
      // =========================================================================
      case "shell_integration_status":
        return { type: "Installed", version: "1.0.0" };

      case "shell_integration_install":
        return undefined;

      case "shell_integration_uninstall":
        return undefined;

      case "get_git_branch":
        // Return mock branch name for browser mode
        return mockGitBranch;

      case "git_status":
        // Return mock git status summary for browser mode
        return mockGitStatus;

      // =========================================================================
      // Theme Commands
      // =========================================================================
      case "list_themes":
        // Return empty array - no custom themes in mock mode
        return [];

      case "read_theme":
        return JSON.stringify({
          name: "Mock Theme",
          colors: {
            background: "#1e1e1e",
            foreground: "#d4d4d4",
          },
        });

      // =========================================================================
      // Workspace Commands
      // =========================================================================
      case "list_workspace_files":
        // Return mock file list
        return [
          { name: "src/App.tsx", path: "/home/user/src/App.tsx" },
          { name: "src/main.tsx", path: "/home/user/src/main.tsx" },
          { name: "package.json", path: "/home/user/package.json" },
        ];

      case "list_path_completions": {
        // Return mock path completions for tab completion feature
        const pathPayload = args as { sessionId: string; partialPath: string; limit?: number };
        const prefix = pathPayload.partialPath.split("/").pop() ?? "";
        const limit = pathPayload.limit ?? 20;

        // Mock completions - directories and files
        const allCompletions = [
          { name: "src/", insert_text: "src/", entry_type: "directory" as const },
          { name: "node_modules/", insert_text: "node_modules/", entry_type: "directory" as const },
          { name: "public/", insert_text: "public/", entry_type: "directory" as const },
          { name: "dist/", insert_text: "dist/", entry_type: "directory" as const },
          { name: ".git/", insert_text: ".git/", entry_type: "directory" as const },
          { name: "package.json", insert_text: "package.json", entry_type: "file" as const },
          { name: "tsconfig.json", insert_text: "tsconfig.json", entry_type: "file" as const },
          { name: "vite.config.ts", insert_text: "vite.config.ts", entry_type: "file" as const },
          { name: "README.md", insert_text: "README.md", entry_type: "file" as const },
          { name: ".gitignore", insert_text: ".gitignore", entry_type: "file" as const },
        ];

        // Filter by prefix (case-insensitive) and hidden file rules
        const showHidden = prefix.startsWith(".");
        const filtered = allCompletions.filter((c) => {
          const name = c.name.replace(/\/$/, "");
          const isHidden = name.startsWith(".");
          if (isHidden && !showHidden) return false;
          if (!prefix) return !isHidden;
          return name.toLowerCase().startsWith(prefix.toLowerCase());
        });

        // Sort: directories first, then alphabetically
        filtered.sort((a, b) => {
          const aIsDir = a.entry_type === "directory";
          const bIsDir = b.entry_type === "directory";
          if (aIsDir && !bIsDir) return -1;
          if (!aIsDir && bIsDir) return 1;
          return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
        });

        return filtered.slice(0, limit);
      }

      // =========================================================================
      // Sidecar Commands
      // =========================================================================
      case "sidecar_status":
        return {
          active_session: false,
          session_id: null,
          enabled: true,
          sessions_dir: "/home/user/.qbit/sessions",
          workspace_path: "/home/user",
        };

      // =========================================================================
      // Prompt Commands
      // =========================================================================
      case "list_prompts":
        return mockPrompts;

      case "read_prompt":
        return "# Mock Prompt\n\nThis is a mock prompt content for browser development.";

      // =========================================================================
      // AI Agent Commands
      // =========================================================================
      case "init_ai_agent":
      case "init_ai_agent_vertex":
        mockAiInitialized = true;
        mockConversationLength = 0;
        return undefined;

      case "send_ai_prompt":
        // In browser mode, we just return a mock response
        // Real streaming events would come from the backend
        mockConversationLength += 2; // User message + AI response
        return `mock-turn-id-${Date.now()}`;

      case "execute_ai_tool":
        return { success: true, result: "Mock tool execution result" };

      case "get_available_tools":
        return mockTools;

      case "list_workflows":
        return mockWorkflows;

      case "list_sub_agents":
        return mockSubAgents;

      case "shutdown_ai_agent":
        mockAiInitialized = false;
        mockConversationLength = 0;
        return undefined;

      case "is_ai_initialized":
        return mockAiInitialized;

      case "update_ai_workspace":
        return undefined;

      case "clear_ai_conversation":
        mockConversationLength = 0;
        return undefined;

      case "get_ai_conversation_length":
        return mockConversationLength;

      case "get_openrouter_api_key":
        return null; // No API key in mock mode

      case "load_env_file":
        return 0; // No variables loaded in mock mode

      case "get_vertex_ai_config":
        // Return mock credentials so the app can initialize in browser mode
        return {
          credentials_path: "/mock/path/to/credentials.json",
          project_id: "mock-project-id",
          location: "us-east5",
        };

      // =========================================================================
      // Session-Specific AI Commands (Per-Tab Isolation)
      // =========================================================================
      case "init_ai_session": {
        validateRequiredParams(cmd, args, ["sessionId", "config"]);
        const payload = args as { sessionId: string; config: unknown };
        mockSessionAiState.set(payload.sessionId, {
          initialized: true,
          conversationLength: 0,
          config: payload.config,
        });
        return undefined;
      }

      case "shutdown_ai_session": {
        validateRequiredParams(cmd, args, ["sessionId"]);
        const payload = args as { sessionId: string };
        mockSessionAiState.delete(payload.sessionId);
        return undefined;
      }

      case "is_ai_session_initialized": {
        validateRequiredParams(cmd, args, ["sessionId"]);
        const payload = args as { sessionId: string };
        return mockSessionAiState.has(payload.sessionId);
      }

      case "get_session_ai_config": {
        validateRequiredParams(cmd, args, ["sessionId"]);
        const payload = args as { sessionId: string };
        const state = mockSessionAiState.get(payload.sessionId);
        if (!state) return null;
        return {
          provider_name: "mock_provider",
          model_name: "mock-model",
          config: state.config,
        };
      }

      case "send_ai_prompt_session": {
        validateRequiredParams(cmd, args, ["sessionId", "prompt"]);
        const payload = args as { sessionId: string; prompt: string };
        const state = mockSessionAiState.get(payload.sessionId);
        if (state) {
          state.conversationLength += 2; // User message + AI response
        }
        return `mock-turn-id-${Date.now()}`;
      }

      case "clear_ai_conversation_session": {
        validateRequiredParams(cmd, args, ["sessionId"]);
        const payload = args as { sessionId: string };
        const state = mockSessionAiState.get(payload.sessionId);
        if (state) {
          state.conversationLength = 0;
        }
        return undefined;
      }

      case "get_ai_conversation_length_session": {
        validateRequiredParams(cmd, args, ["sessionId"]);
        const payload = args as { sessionId: string };
        const state = mockSessionAiState.get(payload.sessionId);
        return state?.conversationLength ?? 0;
      }

      // =========================================================================
      // Session Persistence Commands
      // =========================================================================
      case "list_ai_sessions":
        return mockSessions;

      case "find_ai_session": {
        const findPayload = args as { identifier: string };
        return mockSessions.find((s) => s.identifier === findPayload.identifier) ?? null;
      }

      case "load_ai_session": {
        const loadPayload = args as { identifier: string };
        const session = mockSessions.find((s) => s.identifier === loadPayload.identifier);
        if (!session) return null;
        return {
          ...session,
          transcript: ["User: Hello", "Assistant: Hi! How can I help you?"],
          messages: [
            { role: "user", content: "Hello" },
            { role: "assistant", content: "Hi! How can I help you?" },
          ],
        };
      }

      case "export_ai_session_transcript":
        return undefined;

      case "set_ai_session_persistence": {
        const persistPayload = args as { enabled: boolean };
        mockSessionPersistenceEnabled = persistPayload.enabled;
        return undefined;
      }

      case "is_ai_session_persistence_enabled":
        return mockSessionPersistenceEnabled;

      case "finalize_ai_session":
        return "/home/user/.qbit/sessions/mock-session.json";

      case "restore_ai_session": {
        const restorePayload = args as { identifier: string };
        const restoredSession = mockSessions.find(
          (s) => s.identifier === restorePayload.identifier
        );
        if (!restoredSession) {
          throw new Error(`Session not found: ${restorePayload.identifier}`);
        }
        mockConversationLength = restoredSession.total_messages;
        return {
          ...restoredSession,
          transcript: ["User: Hello", "Assistant: Hi! How can I help you?"],
          messages: [
            { role: "user", content: "Hello" },
            { role: "assistant", content: "Hi! How can I help you?" },
          ],
        };
      }

      // =========================================================================
      // HITL (Human-in-the-Loop) Commands
      // =========================================================================
      case "get_approval_patterns":
        return mockApprovalPatterns;

      case "get_tool_approval_pattern": {
        const patternPayload = args as { toolName: string };
        return mockApprovalPatterns.find((p) => p.tool_name === patternPayload.toolName) ?? null;
      }

      case "get_hitl_config":
        return mockHitlConfig;

      case "set_hitl_config": {
        const configPayload = args as { config: typeof mockHitlConfig };
        mockHitlConfig = configPayload.config;
        return undefined;
      }

      case "add_tool_always_allow": {
        const addPayload = args as { toolName: string };
        if (!mockHitlConfig.always_allow.includes(addPayload.toolName)) {
          mockHitlConfig.always_allow.push(addPayload.toolName);
        }
        return undefined;
      }

      case "remove_tool_always_allow": {
        const removePayload = args as { toolName: string };
        mockHitlConfig.always_allow = mockHitlConfig.always_allow.filter(
          (t) => t !== removePayload.toolName
        );
        return undefined;
      }

      case "reset_approval_patterns":
        return undefined;

      case "respond_to_tool_approval":
        return undefined;

      // =========================================================================
      // Indexer Commands
      // =========================================================================
      case "init_indexer": {
        const initPayload = args as { workspacePath: string };
        mockIndexerInitialized = true;
        mockIndexerWorkspace = initPayload.workspacePath;
        mockIndexedFileCount = 42; // Mock some indexed files
        return {
          files_indexed: 42,
          success: true,
          message: "Mock indexer initialized successfully",
        };
      }

      case "is_indexer_initialized":
        return mockIndexerInitialized;

      case "get_indexer_workspace":
        return mockIndexerWorkspace;

      case "get_indexed_file_count":
        return mockIndexedFileCount;

      case "index_file":
        mockIndexedFileCount += 1;
        return {
          files_indexed: 1,
          success: true,
          message: "File indexed successfully",
        };

      case "index_directory":
        mockIndexedFileCount += 10;
        return {
          files_indexed: 10,
          success: true,
          message: "Directory indexed successfully",
        };

      case "search_code":
        return [
          {
            file_path: "/home/user/qbit/src/lib/ai.ts",
            line_number: 42,
            line_content: "export async function initAiAgent(config: AiConfig): Promise<void> {",
            matches: ["initAiAgent"],
          },
          {
            file_path: "/home/user/qbit/src/lib/tauri.ts",
            line_number: 15,
            line_content: "export async function ptyCreate(",
            matches: ["ptyCreate"],
          },
        ];

      case "search_files":
        return [
          "/home/user/qbit/src/lib/ai.ts",
          "/home/user/qbit/src/lib/tauri.ts",
          "/home/user/qbit/src/lib/indexer.ts",
        ];

      case "analyze_file":
        return {
          symbols: [
            {
              name: "initAiAgent",
              kind: "function",
              line: 42,
              column: 0,
              scope: null,
              signature: "(config: AiConfig): Promise<void>",
              documentation: "Initialize the AI agent with the specified configuration",
            },
          ],
          metrics: {
            lines_of_code: 150,
            lines_of_comments: 30,
            blank_lines: 20,
            functions_count: 12,
            classes_count: 0,
            variables_count: 5,
            imports_count: 3,
            comment_ratio: 0.15,
          },
          dependencies: [
            { name: "@tauri-apps/api/core", kind: "import", source: null },
            { name: "@tauri-apps/api/event", kind: "import", source: null },
          ],
        };

      case "extract_symbols":
        return [
          {
            name: "initAiAgent",
            kind: "function",
            line: 42,
            column: 0,
            scope: null,
            signature: "(config: AiConfig): Promise<void>",
            documentation: "Initialize the AI agent",
          },
          {
            name: "sendPrompt",
            kind: "function",
            line: 100,
            column: 0,
            scope: null,
            signature: "(prompt: string): Promise<string>",
            documentation: "Send a prompt to the AI",
          },
        ];

      case "get_file_metrics":
        return {
          lines_of_code: 150,
          lines_of_comments: 30,
          blank_lines: 20,
          functions_count: 12,
          classes_count: 0,
          variables_count: 5,
          imports_count: 3,
          comment_ratio: 0.15,
        };

      case "detect_language":
        return "typescript";

      case "shutdown_indexer":
        mockIndexerInitialized = false;
        mockIndexerWorkspace = null;
        mockIndexedFileCount = 0;
        return undefined;

      // =========================================================================
      // Codebase Management Commands
      // =========================================================================
      case "list_indexed_codebases":
        return structuredClone(mockCodebases);

      case "add_indexed_codebase": {
        const addPayload = args as { path: string };
        const newCodebase: MockCodebase = {
          path: addPayload.path,
          file_count: Math.floor(Math.random() * 200) + 50,
          status: "synced",
          memory_file: undefined,
        };
        mockCodebases.push(newCodebase);
        return structuredClone(newCodebase);
      }

      case "remove_indexed_codebase": {
        const removePayload = args as { path: string };
        mockCodebases = mockCodebases.filter((cb) => cb.path !== removePayload.path);
        return undefined;
      }

      case "reindex_codebase": {
        const reindexPayload = args as { path: string };
        const codebase = mockCodebases.find((cb) => cb.path === reindexPayload.path);
        if (codebase) {
          codebase.file_count = Math.floor(Math.random() * 200) + 50;
          codebase.status = "synced";
          return structuredClone(codebase);
        }
        throw new Error(`Codebase not found: ${reindexPayload.path}`);
      }

      case "update_codebase_memory_file": {
        const updatePayload = args as { path: string; memoryFile: string | null };
        const codebase = mockCodebases.find((cb) => cb.path === updatePayload.path);
        if (codebase) {
          codebase.memory_file = updatePayload.memoryFile ?? undefined;
        }
        return undefined;
      }

      case "detect_memory_files": {
        // Simulate detecting memory files - randomly return one of the options
        const detectOptions = ["AGENTS.md", "CLAUDE.md", null];
        return detectOptions[Math.floor(Math.random() * detectOptions.length)];
      }

      // =========================================================================
      // Settings Commands
      // =========================================================================
      case "get_settings":
        return structuredClone(mockSettings);

      case "update_settings": {
        const updatePayload = args as { settings: typeof mockSettings };
        mockSettings = structuredClone(updatePayload.settings);
        return undefined;
      }

      case "get_setting": {
        const getPayload = args as { key: string };
        const keys = getPayload.key.split(".");
        let value: unknown = mockSettings;
        for (const k of keys) {
          if (value && typeof value === "object" && k in value) {
            value = (value as Record<string, unknown>)[k];
          } else {
            return null;
          }
        }
        return value;
      }

      case "set_setting": {
        const setPayload = args as { key: string; value: unknown };
        const keys = setPayload.key.split(".");
        let target: Record<string, unknown> = mockSettings as unknown as Record<string, unknown>;
        for (let i = 0; i < keys.length - 1; i++) {
          const k = keys[i];
          if (target[k] && typeof target[k] === "object") {
            target = target[k] as Record<string, unknown>;
          } else {
            return undefined;
          }
        }
        target[keys[keys.length - 1]] = setPayload.value;
        return undefined;
      }

      case "reset_settings":
        // Reset to defaults - in mock mode we just return
        return undefined;

      case "reload_settings":
        // Reload from disk - in mock mode we just return
        return undefined;

      case "settings_file_exists":
        return true;

      case "get_settings_path":
        return "/home/user/.qbit/settings.toml";

      // =========================================================================
      // Project Settings Commands (per-project .qbit/project.toml)
      // =========================================================================
      case "get_project_settings": {
        // Return mock project settings - provider, model, agent_mode
        return mockProjectSettings;
      }

      case "save_project_model": {
        const payload = args as { workspace: string; provider: string; model: string };
        mockProjectSettings.provider = payload.provider;
        mockProjectSettings.model = payload.model;
        console.log(`[Mock IPC] Saved project model: ${payload.provider}/${payload.model}`);
        return undefined;
      }

      case "save_project_agent_mode": {
        const payload = args as { workspace: string; mode: string };
        mockProjectSettings.agent_mode = payload.mode;
        console.log(`[Mock IPC] Saved project agent mode: ${payload.mode}`);
        return undefined;
      }

      // =========================================================================
      // Tauri Plugin Commands (event system)
      // Note: We patch tauriEvent.listen directly, so these handlers are just
      // for compatibility if any code calls invoke() directly
      // =========================================================================
      case "plugin:event|listen": {
        const payload = args as { event: string; handler: number };
        // Return the handler ID - actual registration happens via patched listen()
        return payload.handler;
      }

      case "plugin:event|unlisten": {
        const payload = args as { event: string; eventId: number };
        mockUnregisterListener(payload.eventId);
        return undefined;
      }

      case "plugin:event|emit": {
        // Emit is handled by our emit() calls, just acknowledge it
        return undefined;
      }

      // =========================================================================
      // Default: Unhandled command
      // =========================================================================
      default:
        // Don't warn for plugin commands we might not have implemented yet
        if (!cmd.startsWith("plugin:")) {
          console.warn(`[Mock IPC] Unhandled command: ${cmd}`, args);
        }
        return undefined;
    }
  });

  console.log("[Mocks] Tauri IPC mocks initialized successfully");
}
