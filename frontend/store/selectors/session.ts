/**
 * Combined Session Selectors
 *
 * This module provides optimized selectors for accessing session-specific state.
 * The goal is to reduce the number of store subscriptions by combining related
 * state into a single selector call.
 *
 * Key optimizations:
 * 1. Single selector returns all session state - reduces subscription count
 * 2. Memoization per session - prevents unnecessary recalculations
 * 3. Stable empty references - avoids creating new arrays/objects
 * 4. Cross-session isolation - changes to one session don't invalidate others
 */

import {
  type ActiveSubAgent,
  type ActiveToolCall,
  type ActiveWorkflow,
  type PendingCommand,
  type StreamingBlock,
  type UnifiedBlock,
  useStore,
} from "../index";

/**
 * Combined session state returned by the selector.
 * Contains all the state needed by UnifiedTimeline in one object.
 */
export interface SessionState {
  timeline: UnifiedBlock[];
  streamingBlocks: StreamingBlock[];
  pendingCommand: PendingCommand | null;
  isAgentThinking: boolean;
  thinkingContent: string;
  activeWorkflow: ActiveWorkflow | null;
  activeSubAgents: ActiveSubAgent[];
  activeToolCalls: ActiveToolCall[];
  workingDirectory: string;
  isCompacting: boolean;
  streamingTextLength: number;
  streamingBlockRevision: number;
}

// Stable empty arrays to avoid creating new references
const EMPTY_TIMELINE: UnifiedBlock[] = [];
const EMPTY_STREAMING_BLOCKS: StreamingBlock[] = [];
const EMPTY_SUB_AGENTS: ActiveSubAgent[] = [];
const EMPTY_TOOL_CALLS: ActiveToolCall[] = [];

/**
 * Cache for memoized session state.
 * Key: sessionId
 * Value: { inputs, result } where inputs are the raw state values used to compute result
 */
interface CacheEntry {
  // Input references for shallow comparison
  timeline: UnifiedBlock[] | undefined;
  streamingBlocks: StreamingBlock[] | undefined;
  pendingCommand: PendingCommand | null | undefined;
  isAgentThinking: boolean | undefined;
  thinkingContent: string | undefined;
  activeWorkflow: ActiveWorkflow | null | undefined;
  activeSubAgents: ActiveSubAgent[] | undefined;
  activeToolCalls: ActiveToolCall[] | undefined;
  workingDirectory: string | undefined;
  isCompacting: boolean | undefined;
  streamingTextLength: number;
  streamingBlockRevision: number | undefined;
  // Computed result
  result: SessionState;
}

const cache = new Map<string, CacheEntry>();

/**
 * Get the raw state values for a session.
 * Uses stable empty references for missing data.
 */
function getRawSessionInputs(state: ReturnType<typeof useStore.getState>, sessionId: string) {
  return {
    timeline: state.timelines[sessionId],
    streamingBlocks: state.streamingBlocks[sessionId],
    pendingCommand: state.pendingCommand[sessionId],
    isAgentThinking: state.isAgentThinking[sessionId],
    thinkingContent: state.thinkingContent[sessionId],
    activeWorkflow: state.activeWorkflows[sessionId],
    activeSubAgents: state.activeSubAgents[sessionId],
    activeToolCalls: state.activeToolCalls[sessionId],
    workingDirectory: state.sessions[sessionId]?.workingDirectory,
    isCompacting: state.isCompacting[sessionId],
    streamingTextLength: state.agentStreaming[sessionId]?.length ?? 0,
    streamingBlockRevision: state.streamingBlockRevision[sessionId],
  };
}

/**
 * Check if cache entry is still valid (shallow equality on all inputs).
 */
function isCacheValid(cached: CacheEntry, inputs: ReturnType<typeof getRawSessionInputs>): boolean {
  return (
    cached.timeline === inputs.timeline &&
    cached.streamingBlocks === inputs.streamingBlocks &&
    cached.pendingCommand === inputs.pendingCommand &&
    cached.isAgentThinking === inputs.isAgentThinking &&
    cached.thinkingContent === inputs.thinkingContent &&
    cached.activeWorkflow === inputs.activeWorkflow &&
    cached.activeSubAgents === inputs.activeSubAgents &&
    cached.activeToolCalls === inputs.activeToolCalls &&
    cached.workingDirectory === inputs.workingDirectory &&
    cached.isCompacting === inputs.isCompacting &&
    cached.streamingTextLength === inputs.streamingTextLength &&
    cached.streamingBlockRevision === inputs.streamingBlockRevision
  );
}

/**
 * Create a new SessionState from raw inputs.
 * Uses stable empty references for missing data.
 */
function createSessionState(inputs: ReturnType<typeof getRawSessionInputs>): SessionState {
  return {
    timeline: inputs.timeline ?? EMPTY_TIMELINE,
    streamingBlocks: inputs.streamingBlocks ?? EMPTY_STREAMING_BLOCKS,
    pendingCommand: inputs.pendingCommand ?? null,
    isAgentThinking: inputs.isAgentThinking ?? false,
    thinkingContent: inputs.thinkingContent ?? "",
    activeWorkflow: inputs.activeWorkflow ?? null,
    activeSubAgents: inputs.activeSubAgents ?? EMPTY_SUB_AGENTS,
    activeToolCalls: inputs.activeToolCalls ?? EMPTY_TOOL_CALLS,
    workingDirectory: inputs.workingDirectory ?? "",
    isCompacting: inputs.isCompacting ?? false,
    streamingTextLength: inputs.streamingTextLength,
    streamingBlockRevision: inputs.streamingBlockRevision ?? 0,
  };
}

/**
 * Memoized selector for session state.
 *
 * Returns a stable reference if none of the inputs have changed.
 * This allows React components to skip re-renders when the session
 * state hasn't actually changed.
 *
 * @param state - The full store state
 * @param sessionId - The session to get state for
 * @returns Combined session state
 */
export function selectSessionState(
  state: ReturnType<typeof useStore.getState>,
  sessionId: string
): SessionState {
  const inputs = getRawSessionInputs(state, sessionId);
  const cached = cache.get(sessionId);

  // Return cached result if inputs haven't changed
  if (cached && isCacheValid(cached, inputs)) {
    return cached.result;
  }

  // Compute new result
  const result = createSessionState(inputs);

  // Update cache
  cache.set(sessionId, {
    ...inputs,
    result,
  });

  return result;
}

/**
 * React hook for accessing combined session state.
 *
 * This replaces multiple individual useStore calls with a single subscription.
 * The selector is memoized per session, so changes to other sessions won't
 * cause re-renders.
 *
 * @param sessionId - The session to get state for
 * @returns Combined session state
 */
export function useSessionState(sessionId: string): SessionState {
  return useStore((state) => selectSessionState(state, sessionId));
}

/**
 * Clear the cache for a session.
 * Called when a session is removed.
 */
export function clearSessionCache(sessionId: string): void {
  cache.delete(sessionId);
}

/**
 * Clear all session caches.
 * Useful for testing.
 */
export function clearAllSessionCaches(): void {
  cache.clear();
}
