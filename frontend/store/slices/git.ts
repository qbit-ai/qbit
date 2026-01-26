/**
 * Git slice for the Zustand store.
 *
 * Manages git-related state per session including status, loading state, and commit messages.
 */

import type { GitStatusSummary } from "@/lib/tauri";
import type { SliceCreator } from "./types";

// State interface
export interface GitState {
  gitStatus: Record<string, GitStatusSummary | null>;
  gitStatusLoading: Record<string, boolean>;
  gitCommitMessage: Record<string, string>;
}

// Actions interface
export interface GitActions {
  setGitStatus: (sessionId: string, status: GitStatusSummary | null) => void;
  setGitStatusLoading: (sessionId: string, loading: boolean) => void;
  setGitCommitMessage: (sessionId: string, message: string) => void;
  initGitState: (sessionId: string) => void;
  cleanupGitState: (sessionId: string) => void;
}

// Combined slice interface
export interface GitSlice extends GitState, GitActions {}

// Initial state
export const initialGitState: GitState = {
  gitStatus: {},
  gitStatusLoading: {},
  gitCommitMessage: {},
};

/**
 * Creates the git slice.
 * This slice manages all git-related state and actions.
 */
export const createGitSlice: SliceCreator<GitSlice> = (set) => ({
  // State
  ...initialGitState,

  // Actions
  setGitStatus: (sessionId, status) =>
    set((state) => {
      state.gitStatus[sessionId] = status;
    }),

  setGitStatusLoading: (sessionId, loading) =>
    set((state) => {
      state.gitStatusLoading[sessionId] = loading;
    }),

  setGitCommitMessage: (sessionId, message) =>
    set((state) => {
      state.gitCommitMessage[sessionId] = message;
    }),

  initGitState: (sessionId) =>
    set((state) => {
      state.gitStatus[sessionId] = null;
      // Start with loading true so git badge shows loading spinner immediately
      state.gitStatusLoading[sessionId] = true;
      state.gitCommitMessage[sessionId] = "";
    }),

  cleanupGitState: (sessionId) =>
    set((state) => {
      delete state.gitStatus[sessionId];
      delete state.gitStatusLoading[sessionId];
      delete state.gitCommitMessage[sessionId];
    }),
});

/**
 * Selector for git status of a session.
 */
export const selectGitStatus = <T extends GitState>(
  state: T,
  sessionId: string
): GitStatusSummary | null => state.gitStatus[sessionId] ?? null;

/**
 * Selector for git loading state of a session.
 */
export const selectGitStatusLoading = <T extends GitState>(state: T, sessionId: string): boolean =>
  state.gitStatusLoading[sessionId] ?? false;

/**
 * Selector for git commit message of a session.
 */
export const selectGitCommitMessage = <T extends GitState>(state: T, sessionId: string): string =>
  state.gitCommitMessage[sessionId] ?? "";
