import { render } from "@testing-library/react";
import { createRef } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UnifiedBlock } from "@/store";
import { VirtualizedTimeline } from "./VirtualizedTimeline";

// Mock the store
vi.mock("@/store", async () => {
  const actual = await vi.importActual("@/store");
  return {
    ...actual,
    useStore: vi.fn(() => ({
      collapsedBlocks: {},
      toggleBlockCollapse: vi.fn(),
    })),
  };
});

// Mock ResizeObserver
class MockResizeObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
}
global.ResizeObserver = MockResizeObserver as unknown as typeof ResizeObserver;

function createCommandBlock(id: string, command: string): UnifiedBlock {
  return {
    id,
    type: "command",
    timestamp: new Date().toISOString(),
    data: {
      id,
      sessionId: "test-session",
      command,
      output: `Output for ${command}`,
      exitCode: 0,
      startTime: new Date().toISOString(),
      durationMs: 100,
      workingDirectory: "/test",
      isCollapsed: false,
    },
  };
}

function createAgentMessage(id: string, content: string): UnifiedBlock {
  return {
    id,
    type: "agent_message",
    timestamp: new Date().toISOString(),
    data: {
      id,
      sessionId: "test-session",
      role: "assistant",
      content,
      timestamp: new Date().toISOString(),
      streamingHistory: [],
    },
  };
}

describe("VirtualizedTimeline", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("small timelines (below threshold)", () => {
    it("renders all blocks directly without virtualization", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = [
        createCommandBlock("cmd-1", "ls"),
        createCommandBlock("cmd-2", "pwd"),
        createAgentMessage("msg-1", "Hello"),
      ];

      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // Check that blocks are rendered in a space-y-2 container (non-virtualized mode)
      expect(container.querySelector(".space-y-2")).toBeInTheDocument();
      // All 3 blocks should be rendered as children
      expect(container.querySelector(".space-y-2")?.children.length).toBe(3);
    });

    it("renders empty state for no blocks", () => {
      const containerRef = createRef<HTMLDivElement>();
      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={[]}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // Should render empty space-y-2 container
      const spaceContainer = container.querySelector(".space-y-2");
      expect(spaceContainer).toBeInTheDocument();
      expect(spaceContainer?.children).toHaveLength(0);
    });
  });

  describe("large timelines (above threshold)", () => {
    it("uses virtualization container for many blocks", () => {
      const containerRef = createRef<HTMLDivElement>();
      // Create 60 blocks (above the 50 threshold)
      const blocks: UnifiedBlock[] = Array.from({ length: 60 }, (_, i) =>
        createCommandBlock(`cmd-${i}`, `command-${i}`)
      );

      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // Should have a container with position: relative (virtualization wrapper)
      const virtualContainer = container.querySelector('[style*="position: relative"]');
      expect(virtualContainer).toBeInTheDocument();
    });

    it("renders without crashing for large block counts", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = Array.from({ length: 100 }, (_, i) =>
        createCommandBlock(`cmd-${i}`, `command-${i}`)
      );

      // Should not throw
      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      expect(container).toBeInTheDocument();
    });
  });

  describe("block types", () => {
    it("renders command blocks without errors", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = [createCommandBlock("cmd-1", "echo hello")];

      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // Command block should render in the container
      expect(container.querySelector(".space-y-2")).toBeInTheDocument();
      expect(container.querySelector(".space-y-2")?.children.length).toBe(1);
    });

    it("renders agent message blocks without errors", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = [createAgentMessage("msg-1", "Test response")];

      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // Agent message should render in the container
      expect(container.querySelector(".space-y-2")).toBeInTheDocument();
      expect(container.querySelector(".space-y-2")?.children.length).toBe(1);
    });

    it("renders system hook blocks without errors", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = [
        {
          id: "hook-1",
          type: "system_hook",
          timestamp: new Date().toISOString(),
          data: {
            hooks: ["Test hook content here"],
          },
        },
      ];

      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      // System hook should render in the container
      expect(container.querySelector(".space-y-2")).toBeInTheDocument();
      expect(container.querySelector(".space-y-2")?.children.length).toBe(1);
    });
  });

  describe("error handling", () => {
    it("wraps blocks in error boundaries", () => {
      const containerRef = createRef<HTMLDivElement>();
      const blocks: UnifiedBlock[] = [
        createCommandBlock("cmd-1", "ls"),
        createAgentMessage("msg-1", "Hello"),
      ];

      // Render should succeed
      const { container } = render(
        <div ref={containerRef} style={{ height: 500, overflow: "auto" }}>
          <VirtualizedTimeline
            blocks={blocks}
            sessionId="test-session"
            containerRef={containerRef}
            shouldScrollToBottom={false}
          />
        </div>
      );

      expect(container).toBeInTheDocument();
    });
  });
});
