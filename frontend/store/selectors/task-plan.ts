/**
 * Combined Task Plan Selectors
 *
 * This module provides optimized selectors for accessing task plan state.
 * The goal is to reduce the number of store subscriptions by combining related
 * state into a single selector call.
 *
 * Key optimizations:
 * 1. Single selector returns all task plan state - reduces subscription count
 * 2. Memoization per session - prevents unnecessary recalculations
 * 3. Stable empty references - avoids creating new objects
 * 4. Cross-session isolation - changes to one session don't invalidate others
 */

import type { TaskPlan } from "../index";
import { useStore } from "../index";

/**
 * Combined task plan state returned by the selector.
 * Contains all the state needed by InlineTaskPlan and related components.
 */
export interface TaskPlanState {
  plan: TaskPlan | null;
}

/**
 * Cache for memoized task plan state.
 * Key: sessionId
 * Value: { inputs, result } where inputs are the raw state values used to compute result
 */
interface CacheEntry {
  // Input reference for shallow comparison
  plan: TaskPlan | null | undefined;
  // Computed result
  result: TaskPlanState;
}

const cache = new Map<string, CacheEntry>();

/**
 * Get the raw state values for task plan.
 */
function getRawTaskPlanInputs(state: ReturnType<typeof useStore.getState>, sessionId: string) {
  return {
    plan: state.sessions[sessionId]?.plan,
  };
}

/**
 * Check if cache entry is still valid (shallow equality on all inputs).
 */
function isCacheValid(
  cached: CacheEntry,
  inputs: ReturnType<typeof getRawTaskPlanInputs>
): boolean {
  return cached.plan === inputs.plan;
}

/**
 * Create a new TaskPlanState from raw inputs.
 */
function createTaskPlanState(inputs: ReturnType<typeof getRawTaskPlanInputs>): TaskPlanState {
  return {
    plan: inputs.plan ?? null,
  };
}

/**
 * Memoized selector for task plan state.
 *
 * Returns a stable reference if none of the inputs have changed.
 * This allows React components to skip re-renders when the task plan
 * state hasn't actually changed.
 *
 * @param state - The full store state
 * @param sessionId - The session to get task plan state for
 * @returns Combined task plan state
 */
export function selectTaskPlanState(
  state: ReturnType<typeof useStore.getState>,
  sessionId: string
): TaskPlanState {
  const inputs = getRawTaskPlanInputs(state, sessionId);

  const cached = cache.get(sessionId);
  if (cached && isCacheValid(cached, inputs)) {
    return cached.result;
  }

  const result = createTaskPlanState(inputs);
  cache.set(sessionId, {
    plan: inputs.plan,
    result,
  });

  return result;
}

/**
 * Hook version of selectTaskPlanState.
 * Automatically subscribes to task plan state changes.
 *
 * Usage:
 *   const taskPlanState = useTaskPlanState(sessionId);
 *   // Access: taskPlanState.plan
 */
export function useTaskPlanState(sessionId: string): TaskPlanState {
  return useStore((state) => selectTaskPlanState(state, sessionId));
}
