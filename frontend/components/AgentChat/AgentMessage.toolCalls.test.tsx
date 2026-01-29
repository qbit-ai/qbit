import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AnyToolCall } from "@/lib/toolGrouping";
import type { AgentMessage as AgentMessageType, StreamingBlock } from "@/store";
import { useStore } from "@/store";
import { AgentMessage } from "./AgentMessage";

// Mock the store
vi.mock("@/store", async () => {
  const actual = await vi.importActual("@/store");
  return {
    ...actual,
    useStore: vi.fn(() => undefined),
  };
});

// Mock Markdown component
vi.mock("@/components/Markdown", () => ({
  Markdown: ({ content }: { content: string }) => <div data-testid="markdown">{content}</div>,
}));

// Mock ToolCallDisplay components to verify which one is used
vi.mock("@/components/ToolCallDisplay", () => ({
  MainToolGroup: ({ tools }: { tools: AnyToolCall[] }) => (
    <div data-testid="main-tool-group">
      Group of {tools.length}: {tools.map((t) => t.name).join(", ")}
    </div>
  ),
  ToolItem: ({ tool }: { tool: AnyToolCall }) => (
    <div data-testid="tool-item">Single Tool: {tool.name}</div>
  ),
  ToolDetailsModal: () => null,
  ToolGroupDetailsModal: () => null,
}));

// Helper to create test message
function createTestMessage(overrides: Partial<AgentMessageType> = {}): AgentMessageType {
  return {
    id: "test-message-id",
    sessionId: "test-session-id",
    role: "assistant",
    content: "",
    timestamp: new Date().toISOString(),
    streamingHistory: [],
    ...overrides,
  };
}

function createToolBlock(id: string, name: string): StreamingBlock {
  return {
    type: "tool",
    toolCall: {
      id,
      name,
      args: {},
      status: "completed",
      startedAt: new Date().toISOString(),
    },
  };
}

describe("AgentMessage Tool Call Rendering", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useStore).mockReturnValue(undefined);
  });

  it("renders run_command as a single ToolItem", () => {
    const message = createTestMessage({
      streamingHistory: [createToolBlock("1", "run_command")],
    });

    render(<AgentMessage message={message} />);

    expect(screen.getByTestId("tool-item")).toBeInTheDocument();
    expect(screen.getByText("Single Tool: run_command")).toBeInTheDocument();
    expect(screen.queryByTestId("main-tool-group")).not.toBeInTheDocument();
  });

  it("renders other tools as MainToolGroup", () => {
    const message = createTestMessage({
      streamingHistory: [createToolBlock("1", "read_file"), createToolBlock("2", "read_file")],
    });

    render(<AgentMessage message={message} />);

    expect(screen.getByTestId("main-tool-group")).toBeInTheDocument();
    expect(screen.getByText("Group of 2: read_file, read_file")).toBeInTheDocument();
    expect(screen.queryByTestId("tool-item")).not.toBeInTheDocument();
  });

  it("renders run_command separately from other tools", () => {
    const message = createTestMessage({
      streamingHistory: [
        createToolBlock("1", "read_file"),
        createToolBlock("2", "read_file"),
        createToolBlock("3", "run_command"),
        createToolBlock("4", "read_file"),
        createToolBlock("5", "read_file"),
      ],
    });

    render(<AgentMessage message={message} />);

    // Should have:
    // 1. MainToolGroup (read_file x2)
    // 2. ToolItem (run_command)
    // 3. MainToolGroup (read_file x2)

    const groups = screen.getAllByTestId("main-tool-group");
    const items = screen.getAllByTestId("tool-item");

    expect(groups).toHaveLength(2);
    expect(items).toHaveLength(1);

    expect(groups[0]).toHaveTextContent("Group of 2: read_file, read_file");
    expect(items[0]).toHaveTextContent("Single Tool: run_command");
    expect(groups[1]).toHaveTextContent("Group of 2: read_file, read_file");
  });

  it("renders interleaved text and tools correctly", () => {
    const message = createTestMessage({
      streamingHistory: [
        { type: "text", content: "Checking file..." },
        createToolBlock("1", "read_file"),
        createToolBlock("2", "read_file"),
        { type: "text", content: "Running command..." },
        createToolBlock("3", "run_command"),
      ],
    });

    render(<AgentMessage message={message} />);

    const markdowns = screen.getAllByTestId("markdown");
    expect(markdowns).toHaveLength(2);
    expect(markdowns[0]).toHaveTextContent("Checking file...");
    expect(markdowns[1]).toHaveTextContent("Running command...");

    expect(screen.getByTestId("main-tool-group")).toHaveTextContent(
      "Group of 2: read_file, read_file"
    );
    expect(screen.getByTestId("tool-item")).toHaveTextContent("Single Tool: run_command");
  });
});
