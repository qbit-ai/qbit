import type { ActiveToolCall, FinalizedStreamingBlock, StreamingBlock, ToolCall } from "@/store";

/** Union type for both finalized and active tool calls */
export type AnyToolCall = ToolCall | ActiveToolCall;

/** Input block type - works with both streaming and finalized blocks */
type InputBlock = StreamingBlock | FinalizedStreamingBlock;

/** A group of consecutive tool calls (same type or mixed) */
export interface ToolGroup {
  type: "tool_group";
  toolName?: string;
  tools: AnyToolCall[];
}

/** Grouped streaming block - either text, single tool, udiff result, or tool group */
export type GroupedStreamingBlock =
  | { type: "text"; content: string }
  | { type: "tool"; toolCall: AnyToolCall }
  | { type: "udiff_result"; response: string; durationMs: number }
  | ToolGroup;

/**
 * Groups consecutive tool calls of the same type.
 * Text blocks pass through unchanged and break tool grouping.
 * Single tools are kept as-is, 2+ consecutive same tools become a group.
 * Works with both StreamingBlock[] and FinalizedStreamingBlock[].
 */
export function groupConsecutiveTools(blocks: InputBlock[]): GroupedStreamingBlock[] {
  const result: GroupedStreamingBlock[] = [];
  let currentGroup: AnyToolCall[] = [];
  let currentToolName: string | null = null;

  const flushGroup = () => {
    if (currentGroup.length === 0) return;

    if (currentGroup.length === 1) {
      // Single tool - keep as individual
      result.push({ type: "tool", toolCall: currentGroup[0] });
    } else if (currentToolName) {
      // Multiple tools - create group
      result.push({
        type: "tool_group",
        toolName: currentToolName,
        tools: [...currentGroup],
      });
    }
    currentGroup = [];
    currentToolName = null;
  };

  for (const block of blocks) {
    if (block.type === "text" || block.type === "udiff_result") {
      // Text and udiff_result blocks break any current group and pass through
      flushGroup();
      result.push(block);
    } else {
      // Tool block
      const tool = block.toolCall;

      if (currentToolName === null) {
        // Start new potential group
        currentToolName = tool.name;
        currentGroup.push(tool);
      } else if (tool.name === currentToolName) {
        // Same tool type - add to group
        currentGroup.push(tool);
      } else {
        // Different tool type - flush and start new
        flushGroup();
        currentToolName = tool.name;
        currentGroup.push(tool);
      }
    }
  }

  // Flush any remaining group
  flushGroup();

  return result;
}

/**
 * Groups ANY consecutive tool calls (regardless of tool name).
 * Text blocks pass through unchanged and break tool grouping.
 * Single tools are kept as-is, 2+ consecutive tools become a group.
 */
export function groupConsecutiveToolsByAny(blocks: InputBlock[]): GroupedStreamingBlock[] {
  const result: GroupedStreamingBlock[] = [];
  let currentGroup: AnyToolCall[] = [];

  const flushGroup = () => {
    if (currentGroup.length === 0) return;

    if (currentGroup.length === 1) {
      // Single tool - keep as individual
      result.push({ type: "tool", toolCall: currentGroup[0] });
    } else {
      // Multiple tools - create group (no toolName for mixed groups)
      result.push({
        type: "tool_group",
        tools: [...currentGroup],
      });
    }
    currentGroup = [];
  };

  for (const block of blocks) {
    if (block.type === "text" || block.type === "udiff_result") {
      // Text and udiff_result blocks break any current group and pass through
      flushGroup();
      result.push(block);
    } else {
      // Tool block - add to current group
      currentGroup.push(block.toolCall);
    }
  }

  // Flush any remaining group
  flushGroup();

  return result;
}

/**
 * Sort tools by startedAt descending (newest first).
 * Tools without startedAt remain in their original relative order at the end.
 */
export function sortToolsByStartedAtDesc(tools: AnyToolCall[]): AnyToolCall[] {
  const withTimestamp: Array<{ tool: AnyToolCall; timestamp: number; originalIndex: number }> = [];
  const withoutTimestamp: Array<{ tool: AnyToolCall; originalIndex: number }> = [];

  tools.forEach((tool, index) => {
    if ("startedAt" in tool && tool.startedAt) {
      withTimestamp.push({
        tool,
        timestamp: new Date(tool.startedAt).getTime(),
        originalIndex: index,
      });
    } else {
      withoutTimestamp.push({ tool, originalIndex: index });
    }
  });

  // Sort timestamped tools by timestamp descending
  withTimestamp.sort((a, b) => b.timestamp - a.timestamp);

  // Concatenate: timestamped tools first, then non-timestamped in original order
  return [...withTimestamp.map((t) => t.tool), ...withoutTimestamp.map((t) => t.tool)];
}

/**
 * Sort tools by startedAt ascending (oldest first).
 * Tools without startedAt remain in their original relative order at the end.
 */
export function sortToolsByStartedAtAsc(tools: AnyToolCall[]): AnyToolCall[] {
  const withTimestamp: Array<{ tool: AnyToolCall; timestamp: number; originalIndex: number }> = [];
  const withoutTimestamp: Array<{ tool: AnyToolCall; originalIndex: number }> = [];

  tools.forEach((tool, index) => {
    if ("startedAt" in tool && tool.startedAt) {
      withTimestamp.push({
        tool,
        timestamp: new Date(tool.startedAt).getTime(),
        originalIndex: index,
      });
    } else {
      withoutTimestamp.push({ tool, originalIndex: index });
    }
  });

  // Sort timestamped tools by timestamp ascending
  withTimestamp.sort((a, b) => a.timestamp - b.timestamp);

  // Concatenate: timestamped tools first, then non-timestamped in original order
  return [...withTimestamp.map((t) => t.tool), ...withoutTimestamp.map((t) => t.tool)];
}

/**
 * Computes total duration for a group of tools.
 * Returns { durationMs, label } where label includes "So far" if any tool is still running.
 */
export function computeToolGroupDuration(
  tools: AnyToolCall[],
  now: number = Date.now()
): { durationMs: number | null; label: string } {
  // Get earliest startedAt
  const startTimes = tools
    .filter((t): t is ActiveToolCall => "startedAt" in t && !!t.startedAt)
    .map((t) => new Date(t.startedAt).getTime());

  if (startTimes.length === 0) {
    return { durationMs: null, label: "" };
  }

  const earliestStart = Math.min(...startTimes);

  // Check if any tool is still running (missing completedAt)
  const hasRunning = tools.some(
    (t) => "startedAt" in t && t.status === "running" && (!("completedAt" in t) || !t.completedAt)
  );

  // Get latest completedAt or use current time if still running
  let endTime = now;
  if (!hasRunning) {
    const completedTimes = tools
      .filter(
        (t): t is ActiveToolCall & { completedAt: string } =>
          "completedAt" in t && typeof t.completedAt === "string"
      )
      .map((t) => new Date(t.completedAt).getTime());

    if (completedTimes.length > 0) {
      endTime = Math.max(...completedTimes);
    }
  }

  const durationMs = endTime - earliestStart;

  // Format label
  let label: string;
  if (durationMs < 1000) {
    label = `${durationMs}ms`;
  } else {
    label = `${(durationMs / 1000).toFixed(1)}s`;
  }

  if (hasRunning) {
    label += " so far";
  }

  return { durationMs, label };
}

/**
 * Primary argument mapping for each tool type.
 * Returns the key name to extract from args for inline display.
 */
const primaryArgKeys: Record<string, string> = {
  read_file: "path",
  write_file: "path",
  edit_file: "path",
  list_files: "path",
  grep_file: "pattern",
  run_pty_cmd: "command",
  shell: "command",
  web_fetch: "url",
  web_search: "query",
  web_search_answer: "query",
  apply_patch: "path",
};

/**
 * Extracts the primary argument from a tool call for inline display.
 * Returns null if no primary arg is defined or found.
 */
export function getPrimaryArgument(tool: AnyToolCall): string | null {
  const key = primaryArgKeys[tool.name];
  if (!key) return null;

  const value = tool.args[key];
  if (typeof value !== "string") return null;

  return value;
}

/**
 * Formats a primary argument for display.
 * - For file paths: extracts basename
 * - For patterns/queries: truncates with quotes
 * - For URLs: extracts domain
 * - For commands: first word + truncate
 */
export function formatPrimaryArg(tool: AnyToolCall, maxLength = 30): string | null {
  const value = getPrimaryArgument(tool);
  if (!value) return null;

  const toolName = tool.name;

  // File paths - extract basename
  if (["read_file", "write_file", "edit_file", "apply_patch"].includes(toolName)) {
    const parts = value.split("/");
    return parts[parts.length - 1];
  }

  // Directory paths - show last segment with trailing slash
  if (toolName === "list_files") {
    const parts = value.replace(/\/$/, "").split("/");
    const last = parts[parts.length - 1];
    return last ? `${last}/` : value;
  }

  // Patterns/queries - wrap in quotes, truncate
  if (["grep_file", "web_search", "web_search_answer"].includes(toolName)) {
    const truncated = value.length > maxLength ? `${value.slice(0, maxLength - 3)}...` : value;
    return `"${truncated}"`;
  }

  // URLs - extract domain
  if (toolName === "web_fetch") {
    try {
      const url = new URL(value);
      return url.hostname;
    } catch {
      return value.slice(0, maxLength);
    }
  }

  // Commands - truncate
  if (["run_pty_cmd", "shell"].includes(toolName)) {
    return value.length > maxLength ? `${value.slice(0, maxLength - 3)}...` : value;
  }

  return value.slice(0, maxLength);
}

/**
 * Gets the aggregate status for a group of tools.
 * - If any running → "running"
 * - If any error → "error"
 * - If all completed → "completed"
 * - Otherwise → first tool's status
 */
export function getGroupStatus(tools: AnyToolCall[]): AnyToolCall["status"] {
  const hasRunning = tools.some((t) => t.status === "running");
  if (hasRunning) return "running";

  const hasError = tools.some((t) => t.status === "error");
  if (hasError) return "error";

  const allCompleted = tools.every((t) => t.status === "completed");
  if (allCompleted) return "completed";

  return tools[0]?.status ?? "pending";
}
