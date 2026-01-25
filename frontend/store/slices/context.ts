/**
 * Context slice for the Zustand store.
 *
 * Manages context window metrics, token usage tracking, and compaction state per session.
 */

import type { SliceCreator } from "./types";

// Types
/** Context window utilization metrics for a session */
export interface ContextMetrics {
  /** Current context utilization (0.0 to 1.0) */
  utilization: number;
  /** Number of tokens currently used */
  usedTokens: number;
  /** Maximum tokens available in context window */
  maxTokens: number;
  /** True if utilization is at warning level (>=75%) */
  isWarning: boolean;
}

// State interface
export interface ContextState {
  /** Accumulated token usage per session (input/output separately) */
  sessionTokenUsage: Record<string, { input: number; output: number }>;
  /** Context window utilization per session */
  contextMetrics: Record<string, ContextMetrics>;
  /** Number of compactions performed per session */
  compactionCount: Record<string, number>;
  /** Whether compaction is currently in progress */
  isCompacting: Record<string, boolean>;
  /** Whether session is dead (compaction failed critically) */
  isSessionDead: Record<string, boolean>;
  /** Last compaction error (for retry UI) */
  compactionError: Record<string, string | null>;
}

// Actions interface
export interface ContextActions {
  /** Set context metrics for a session (partial update) */
  setContextMetrics: (sessionId: string, metrics: Partial<ContextMetrics>) => void;
  /** Set whether compaction is in progress */
  setCompacting: (sessionId: string, isCompacting: boolean) => void;
  /** Handle successful compaction */
  handleCompactionSuccess: (sessionId: string) => void;
  /** Handle failed compaction */
  handleCompactionFailed: (sessionId: string, error: string) => void;
  /** Clear compaction error */
  clearCompactionError: (sessionId: string) => void;
  /** Set session as dead (unrecoverable) */
  setSessionDead: (sessionId: string, isDead: boolean) => void;
  /** Initialize context state for a new session */
  initContextState: (sessionId: string) => void;
  /** Clean up context state when session is removed */
  cleanupContextState: (sessionId: string) => void;
}

// Combined slice interface
export interface ContextSlice extends ContextState, ContextActions {}

// Initial state
export const initialContextState: ContextState = {
  sessionTokenUsage: {},
  contextMetrics: {},
  compactionCount: {},
  isCompacting: {},
  isSessionDead: {},
  compactionError: {},
};

// Default context metrics for new sessions
const DEFAULT_CONTEXT_METRICS: ContextMetrics = {
  utilization: 0,
  usedTokens: 0,
  maxTokens: 0,
  isWarning: false,
};

/**
 * Creates the context slice.
 * This slice manages all context window and compaction-related state and actions.
 */
export const createContextSlice: SliceCreator<ContextSlice> = (set) => ({
  // State
  ...initialContextState,

  // Actions
  setContextMetrics: (sessionId, metrics) =>
    set((state) => {
      const current = state.contextMetrics[sessionId] ?? DEFAULT_CONTEXT_METRICS;
      state.contextMetrics[sessionId] = { ...current, ...metrics };
    }),

  setCompacting: (sessionId, isCompacting) =>
    set((state) => {
      state.isCompacting[sessionId] = isCompacting;
    }),

  handleCompactionSuccess: (sessionId) =>
    set((state) => {
      state.compactionCount[sessionId] = (state.compactionCount[sessionId] ?? 0) + 1;
      state.isCompacting[sessionId] = false;
      state.compactionError[sessionId] = null;
      state.isSessionDead[sessionId] = false;
    }),

  handleCompactionFailed: (sessionId, error) =>
    set((state) => {
      state.isCompacting[sessionId] = false;
      state.compactionError[sessionId] = error;
      // Note: isSessionDead is set by the event handler based on severity
    }),

  clearCompactionError: (sessionId) =>
    set((state) => {
      state.compactionError[sessionId] = null;
    }),

  setSessionDead: (sessionId, isDead) =>
    set((state) => {
      state.isSessionDead[sessionId] = isDead;
    }),

  initContextState: (sessionId) =>
    set((state) => {
      state.contextMetrics[sessionId] = { ...DEFAULT_CONTEXT_METRICS };
      state.compactionCount[sessionId] = 0;
      state.isCompacting[sessionId] = false;
      state.isSessionDead[sessionId] = false;
      state.compactionError[sessionId] = null;
    }),

  cleanupContextState: (sessionId) =>
    set((state) => {
      delete state.sessionTokenUsage[sessionId];
      delete state.contextMetrics[sessionId];
      delete state.compactionCount[sessionId];
      delete state.isCompacting[sessionId];
      delete state.isSessionDead[sessionId];
      delete state.compactionError[sessionId];
    }),
});

// Stable empty context metrics for selectors
const EMPTY_CONTEXT_METRICS: ContextMetrics = {
  utilization: 0,
  usedTokens: 0,
  maxTokens: 0,
  isWarning: false,
};

/**
 * Selector for context metrics of a session.
 */
export const selectContextMetrics = <T extends ContextState>(
  state: T,
  sessionId: string
): ContextMetrics => state.contextMetrics[sessionId] ?? EMPTY_CONTEXT_METRICS;

/**
 * Selector for compaction count of a session.
 */
export const selectCompactionCount = <T extends ContextState>(
  state: T,
  sessionId: string
): number => state.compactionCount[sessionId] ?? 0;

/**
 * Selector for whether compaction is in progress.
 */
export const selectIsCompacting = <T extends ContextState>(state: T, sessionId: string): boolean =>
  state.isCompacting[sessionId] ?? false;

/**
 * Selector for whether session is dead.
 */
export const selectIsSessionDead = <T extends ContextState>(state: T, sessionId: string): boolean =>
  state.isSessionDead[sessionId] ?? false;

/**
 * Selector for compaction error.
 */
export const selectCompactionError = <T extends ContextState>(
  state: T,
  sessionId: string
): string | null => state.compactionError[sessionId] ?? null;

/**
 * Selector for session token usage.
 */
export const selectSessionTokenUsage = <T extends ContextState>(
  state: T,
  sessionId: string
): { input: number; output: number } =>
  state.sessionTokenUsage[sessionId] ?? { input: 0, output: 0 };
