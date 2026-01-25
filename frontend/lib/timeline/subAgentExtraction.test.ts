import { describe, expect, it } from "vitest";
import type { GroupedStreamingBlock } from "@/lib/toolGrouping";
import type { ActiveSubAgent } from "@/store";
import { extractSubAgentBlocks } from "./subAgentExtraction";

describe("extractSubAgentBlocks", () => {
  describe("basic extraction", () => {
    it("should return empty arrays for empty inputs", () => {
      const result = extractSubAgentBlocks([], []);
      expect(result.subAgentBlocks).toEqual([]);
      expect(result.contentBlocks).toEqual([]);
    });

    it("should pass through text blocks unchanged", () => {
      const blocks: GroupedStreamingBlock[] = [
        { type: "text", content: "Hello world" },
        { type: "text", content: "More text" },
      ];

      const result = extractSubAgentBlocks(blocks, []);
      expect(result.subAgentBlocks).toEqual([]);
      expect(result.contentBlocks).toEqual(blocks);
    });

    it("should pass through regular tool calls unchanged", () => {
      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-1",
            name: "read_file",
            args: { path: "/test.txt" },
            status: "completed",
          },
        },
      ];

      const result = extractSubAgentBlocks(blocks, []);
      expect(result.subAgentBlocks).toEqual([]);
      expect(result.contentBlocks).toEqual(blocks);
    });

    it("should pass through udiff_result blocks unchanged", () => {
      const blocks: GroupedStreamingBlock[] = [
        { type: "udiff_result", response: "diff output", durationMs: 100 },
      ];

      const result = extractSubAgentBlocks(blocks, []);
      expect(result.subAgentBlocks).toEqual([]);
      expect(result.contentBlocks).toEqual(blocks);
    });
  });

  describe("sub-agent inlining by parentRequestId", () => {
    it("should inline single sub-agent at correct position (replacing tool call)", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "explore codebase",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        { type: "text", content: "Before" },
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-1",
            name: "sub_agent_explore",
            args: { task: "explore" },
            status: "completed",
          },
        },
        { type: "text", content: "After" },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      // subAgentBlocks is kept empty for backward compatibility
      expect(result.subAgentBlocks).toEqual([]);

      // Sub-agent should be inlined at correct position in contentBlocks
      expect(result.contentBlocks).toHaveLength(3);
      expect(result.contentBlocks[0]).toEqual({ type: "text", content: "Before" });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent });
      expect(result.contentBlocks[2]).toEqual({ type: "text", content: "After" });
    });

    it("should inline multiple sub-agents in order preserving interleaved content", () => {
      const subAgent1: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task 1",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const subAgent2: ActiveSubAgent = {
        agentId: "agent-2",
        agentName: "coder",
        parentRequestId: "tool-sub-2",
        task: "task 2",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:01Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-1",
            name: "sub_agent_explore",
            args: {},
            status: "completed",
          },
        },
        { type: "text", content: "Between" },
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-2",
            name: "sub_agent_coder",
            args: {},
            status: "completed",
          },
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent1, subAgent2]);

      expect(result.subAgentBlocks).toEqual([]);

      // Order should be preserved: sub_agent_1, text, sub_agent_2
      expect(result.contentBlocks).toHaveLength(3);
      expect(result.contentBlocks[0]).toEqual({ type: "sub_agent", subAgent: subAgent1 });
      expect(result.contentBlocks[1]).toEqual({ type: "text", content: "Between" });
      expect(result.contentBlocks[2]).toEqual({ type: "sub_agent", subAgent: subAgent2 });
    });

    it("should not match sub-agent more than once", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      // Two tools with the same ID (shouldn't happen but test the guard)
      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-1",
            name: "sub_agent_explore",
            args: {},
            status: "completed",
          },
        },
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-1",
            name: "sub_agent_explore",
            args: {},
            status: "completed",
          },
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      // Only one should be matched and inlined
      const subAgentBlocks = result.contentBlocks.filter((b) => b.type === "sub_agent");
      expect(subAgentBlocks).toHaveLength(1);
    });
  });

  describe("tool group handling with inline ordering", () => {
    it("should inline sub-agent at correct position within tool group", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          tools: [
            { id: "tool-1", name: "read_file", args: {}, status: "completed" },
            { id: "tool-sub-1", name: "sub_agent_explore", args: {}, status: "completed" },
            { id: "tool-2", name: "write_file", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      expect(result.subAgentBlocks).toEqual([]);

      // Order should be: tool (read_file), sub_agent, tool (write_file)
      expect(result.contentBlocks).toHaveLength(3);
      expect(result.contentBlocks[0]).toEqual({
        type: "tool",
        toolCall: { id: "tool-1", name: "read_file", args: {}, status: "completed" },
      });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent });
      expect(result.contentBlocks[2]).toEqual({
        type: "tool",
        toolCall: { id: "tool-2", name: "write_file", args: {}, status: "completed" },
      });
    });

    it("should preserve tool group when sub-agent is at end", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          tools: [
            { id: "tool-1", name: "read_file", args: {}, status: "completed" },
            { id: "tool-2", name: "write_file", args: {}, status: "completed" },
            { id: "tool-sub-1", name: "sub_agent_explore", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      expect(result.subAgentBlocks).toEqual([]);

      // Order should be: tool_group (read, write), sub_agent
      expect(result.contentBlocks).toHaveLength(2);
      expect(result.contentBlocks[0]).toEqual({
        type: "tool_group",
        tools: [
          { id: "tool-1", name: "read_file", args: {}, status: "completed" },
          { id: "tool-2", name: "write_file", args: {}, status: "completed" },
        ],
      });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent });
    });

    it("should convert to single tool when only one remains after sub-agent", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          tools: [
            { id: "tool-sub-1", name: "sub_agent_explore", args: {}, status: "completed" },
            { id: "tool-1", name: "read_file", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      expect(result.subAgentBlocks).toEqual([]);

      // Order should be: sub_agent, single tool (not a group)
      expect(result.contentBlocks).toHaveLength(2);
      expect(result.contentBlocks[0]).toEqual({ type: "sub_agent", subAgent });
      expect(result.contentBlocks[1]).toEqual({
        type: "tool",
        toolCall: { id: "tool-1", name: "read_file", args: {}, status: "completed" },
      });
    });

    it("should output only sub-agents when all tools in group are sub-agents", () => {
      const subAgent1: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task 1",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const subAgent2: ActiveSubAgent = {
        agentId: "agent-2",
        agentName: "coder",
        parentRequestId: "tool-sub-2",
        task: "task 2",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:01Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          tools: [
            { id: "tool-sub-1", name: "sub_agent_explore", args: {}, status: "completed" },
            { id: "tool-sub-2", name: "sub_agent_coder", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent1, subAgent2]);

      expect(result.subAgentBlocks).toEqual([]);

      // Only sub-agents, in order
      expect(result.contentBlocks).toHaveLength(2);
      expect(result.contentBlocks[0]).toEqual({ type: "sub_agent", subAgent: subAgent1 });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent: subAgent2 });
    });
  });

  describe("fallback for legacy data without parentRequestId", () => {
    it("should fall back to index-based matching when no parentRequestId", () => {
      // Legacy sub-agent without parentRequestId
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "", // Empty = legacy
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool",
          toolCall: {
            id: "tool-sub-1",
            name: "sub_agent_explore",
            args: {},
            status: "completed",
          },
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      expect(result.subAgentBlocks).toEqual([]);

      // Should still inline by index fallback
      expect(result.contentBlocks).toHaveLength(1);
      expect(result.contentBlocks[0]).toEqual({ type: "sub_agent", subAgent });
    });
  });

  describe("unmatched sub-agents fallback", () => {
    it("should append unmatched sub-agents at the end of contentBlocks", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "unmatched-id",
        task: "task",
        depth: 1,
        status: "running",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      // No matching tool call in blocks
      const blocks: GroupedStreamingBlock[] = [{ type: "text", content: "Some text" }];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      expect(result.subAgentBlocks).toEqual([]);

      // Sub-agent should appear at end (fallback for state race conditions)
      expect(result.contentBlocks).toHaveLength(2);
      expect(result.contentBlocks[0]).toEqual({ type: "text", content: "Some text" });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent });
    });
  });

  describe("preserves tool group properties", () => {
    it("should preserve toolName property on tool_group", () => {
      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          toolName: "read_file",
          tools: [
            { id: "tool-1", name: "read_file", args: {}, status: "completed" },
            { id: "tool-2", name: "read_file", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, []);

      expect(result.contentBlocks).toHaveLength(1);
      expect(result.contentBlocks[0]).toEqual(blocks[0]);
    });

    it("should preserve toolName on remaining tools when sub-agent is extracted", () => {
      const subAgent: ActiveSubAgent = {
        agentId: "agent-1",
        agentName: "explore",
        parentRequestId: "tool-sub-1",
        task: "task",
        depth: 1,
        status: "completed",
        toolCalls: [],
        startedAt: "2024-01-01T00:00:00Z",
      };

      const blocks: GroupedStreamingBlock[] = [
        {
          type: "tool_group",
          toolName: "read_file",
          tools: [
            { id: "tool-1", name: "read_file", args: {}, status: "completed" },
            { id: "tool-sub-1", name: "sub_agent_explore", args: {}, status: "completed" },
            { id: "tool-2", name: "read_file", args: {}, status: "completed" },
          ],
        },
      ];

      const result = extractSubAgentBlocks(blocks, [subAgent]);

      // First tool, sub-agent, then remaining tool
      expect(result.contentBlocks).toHaveLength(3);
      expect(result.contentBlocks[0]).toEqual({
        type: "tool",
        toolCall: { id: "tool-1", name: "read_file", args: {}, status: "completed" },
      });
      expect(result.contentBlocks[1]).toEqual({ type: "sub_agent", subAgent });
      // The remaining single tool should have toolName preserved if we had a group
      expect(result.contentBlocks[2]).toEqual({
        type: "tool",
        toolCall: { id: "tool-2", name: "read_file", args: {}, status: "completed" },
      });
    });
  });
});
