/**
 * Tests for the AI event handler registry.
 */

import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { dispatchEvent, eventHandlerRegistry } from "./registry";
import type { EventHandlerContext, EventHandlerRegistry } from "./types";

// Mock logger
vi.mock("@/lib/logger", () => ({
  logger: {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  },
}));

describe("eventHandlerRegistry", () => {
  it("contains handlers for all core lifecycle events", () => {
    expect(eventHandlerRegistry.started).toBeDefined();
    expect(eventHandlerRegistry.text_delta).toBeDefined();
    expect(eventHandlerRegistry.reasoning).toBeDefined();
    expect(eventHandlerRegistry.completed).toBeDefined();
    expect(eventHandlerRegistry.error).toBeDefined();
    expect(eventHandlerRegistry.system_hooks_injected).toBeDefined();
  });

  it("contains handlers for all tool events", () => {
    expect(eventHandlerRegistry.tool_request).toBeDefined();
    expect(eventHandlerRegistry.tool_approval_request).toBeDefined();
    expect(eventHandlerRegistry.tool_auto_approved).toBeDefined();
    expect(eventHandlerRegistry.tool_result).toBeDefined();
  });

  it("contains handlers for all workflow events", () => {
    expect(eventHandlerRegistry.workflow_started).toBeDefined();
    expect(eventHandlerRegistry.workflow_step_started).toBeDefined();
    expect(eventHandlerRegistry.workflow_step_completed).toBeDefined();
    expect(eventHandlerRegistry.workflow_completed).toBeDefined();
    expect(eventHandlerRegistry.workflow_error).toBeDefined();
  });

  it("contains handlers for all sub-agent events", () => {
    expect(eventHandlerRegistry.sub_agent_started).toBeDefined();
    expect(eventHandlerRegistry.sub_agent_tool_request).toBeDefined();
    expect(eventHandlerRegistry.sub_agent_tool_result).toBeDefined();
    expect(eventHandlerRegistry.sub_agent_completed).toBeDefined();
    expect(eventHandlerRegistry.sub_agent_error).toBeDefined();
  });

  it("contains handlers for all context management events", () => {
    expect(eventHandlerRegistry.context_warning).toBeDefined();
    expect(eventHandlerRegistry.compaction_started).toBeDefined();
    expect(eventHandlerRegistry.compaction_completed).toBeDefined();
    expect(eventHandlerRegistry.compaction_failed).toBeDefined();
    expect(eventHandlerRegistry.tool_response_truncated).toBeDefined();
  });

  it("contains handlers for all miscellaneous events", () => {
    expect(eventHandlerRegistry.plan_updated).toBeDefined();
    expect(eventHandlerRegistry.server_tool_started).toBeDefined();
    expect(eventHandlerRegistry.web_search_result).toBeDefined();
    expect(eventHandlerRegistry.web_fetch_result).toBeDefined();
  });

  it("has exactly 29 registered handlers", () => {
    const registeredHandlers = Object.keys(eventHandlerRegistry).filter(
      (key) => eventHandlerRegistry[key as keyof EventHandlerRegistry] !== undefined
    );
    expect(registeredHandlers.length).toBe(29);
  });
});

describe("dispatchEvent", () => {
  let mockCtx: EventHandlerContext;
  let mockState: Record<string, Mock>;

  beforeEach(() => {
    mockState = {
      clearAgentStreaming: vi.fn(),
      clearActiveToolCalls: vi.fn(),
      clearThinkingContent: vi.fn(),
      setAgentThinking: vi.fn(),
      setAgentResponding: vi.fn(),
      appendThinkingContent: vi.fn(),
      addStreamingSystemHooksBlock: vi.fn(),
      addSystemHookBlock: vi.fn(),
    };

    mockCtx = {
      sessionId: "test-session",
      getState: vi.fn(() => mockState) as unknown as EventHandlerContext["getState"],
      flushSessionDeltas: vi.fn(),
      batchTextDelta: vi.fn(),
      convertToolSource: vi.fn(),
    };
  });

  it("returns true when event is handled", () => {
    const event = {
      type: "started" as const,
      turn_id: "turn-1",
      session_id: "test-session",
    };
    const result = dispatchEvent(event, mockCtx);
    expect(result).toBe(true);
  });

  it("returns false for unknown event types", () => {
    // Use type assertion to test runtime behavior with unknown event type
    const event = {
      type: "unknown_event",
      session_id: "test-session",
    } as unknown as Parameters<typeof dispatchEvent>[0];
    const result = dispatchEvent(event, mockCtx);
    expect(result).toBe(false);
  });

  it("dispatches started event to correct handler", () => {
    const event = {
      type: "started" as const,
      turn_id: "turn-1",
      session_id: "test-session",
    };
    dispatchEvent(event, mockCtx);

    expect(mockCtx.getState).toHaveBeenCalled();
    expect(mockState.clearAgentStreaming).toHaveBeenCalledWith("test-session");
    expect(mockState.setAgentThinking).toHaveBeenCalledWith("test-session", true);
    expect(mockState.setAgentResponding).toHaveBeenCalledWith("test-session", true);
  });

  it("dispatches text_delta event to correct handler", () => {
    const event = {
      type: "text_delta" as const,
      delta: "Hello",
      accumulated: "Hello",
      session_id: "test-session",
    };
    dispatchEvent(event, mockCtx);

    expect(mockCtx.batchTextDelta).toHaveBeenCalledWith("test-session", "Hello");
    expect(mockState.setAgentThinking).toHaveBeenCalledWith("test-session", false);
  });

  it("dispatches reasoning event to correct handler", () => {
    const event = {
      type: "reasoning" as const,
      content: "Thinking about this...",
      session_id: "test-session",
    };
    dispatchEvent(event, mockCtx);

    expect(mockState.appendThinkingContent).toHaveBeenCalledWith(
      "test-session",
      "Thinking about this..."
    );
  });

  it("dispatches system_hooks_injected event to correct handler", () => {
    const event = {
      type: "system_hooks_injected" as const,
      hooks: ["hook1", "hook2"],
      session_id: "test-session",
    };
    dispatchEvent(event, mockCtx);

    expect(mockCtx.flushSessionDeltas).toHaveBeenCalledWith("test-session");
    expect(mockState.addStreamingSystemHooksBlock).toHaveBeenCalledWith("test-session", [
      "hook1",
      "hook2",
    ]);
    expect(mockState.addSystemHookBlock).toHaveBeenCalledWith("test-session", ["hook1", "hook2"]);
  });
});
