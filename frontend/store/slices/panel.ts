/**
 * Panel slice for the Zustand store.
 *
 * Manages UI panel open/close state with mutual exclusion for right-side panels.
 * Right-side panels (git, context, fileEditor, sidecar) are mutually exclusive -
 * only one can be open at a time. SessionBrowser is independent (it's a dialog).
 */

import type { SliceCreator } from "./types";

// State interface
export interface PanelState {
  gitPanelOpen: boolean;
  contextPanelOpen: boolean;
  fileEditorPanelOpen: boolean;
  sidecarPanelOpen: boolean;
  sessionBrowserOpen: boolean;
}

// Actions interface
export interface PanelActions {
  openGitPanel: () => void;
  openContextPanel: () => void;
  openFileEditorPanel: () => void;
  openSidecarPanel: () => void;
  openSessionBrowser: () => void;
  closePanels: () => void;
  closeSessionBrowser: () => void;
  toggleFileEditorPanel: () => void;
  setSessionBrowserOpen: (open: boolean) => void;
}

// Combined slice interface
export interface PanelSlice extends PanelState, PanelActions {}

// Initial state
export const initialPanelState: PanelState = {
  gitPanelOpen: false,
  contextPanelOpen: false,
  fileEditorPanelOpen: false,
  sidecarPanelOpen: false,
  sessionBrowserOpen: false,
};

/**
 * Creates the panel slice.
 * Right-side panels are mutually exclusive: opening one closes the others.
 * SessionBrowser is independent (dialog overlay, not a side panel).
 */
export const createPanelSlice: SliceCreator<PanelSlice> = (set) => ({
  ...initialPanelState,

  openGitPanel: () =>
    set((state) => {
      state.gitPanelOpen = true;
      state.contextPanelOpen = false;
      state.fileEditorPanelOpen = false;
      state.sidecarPanelOpen = false;
    }),

  openContextPanel: () =>
    set((state) => {
      state.contextPanelOpen = true;
      state.gitPanelOpen = false;
      state.fileEditorPanelOpen = false;
      state.sidecarPanelOpen = false;
    }),

  openFileEditorPanel: () =>
    set((state) => {
      state.fileEditorPanelOpen = true;
      state.gitPanelOpen = false;
      state.contextPanelOpen = false;
      state.sidecarPanelOpen = false;
    }),

  openSidecarPanel: () =>
    set((state) => {
      state.sidecarPanelOpen = true;
      state.gitPanelOpen = false;
      state.contextPanelOpen = false;
      state.fileEditorPanelOpen = false;
    }),

  openSessionBrowser: () =>
    set((state) => {
      state.sessionBrowserOpen = true;
    }),

  closePanels: () =>
    set((state) => {
      state.gitPanelOpen = false;
      state.contextPanelOpen = false;
      state.fileEditorPanelOpen = false;
      state.sidecarPanelOpen = false;
    }),

  closeSessionBrowser: () =>
    set((state) => {
      state.sessionBrowserOpen = false;
    }),

  toggleFileEditorPanel: () =>
    set((state) => {
      const next = !state.fileEditorPanelOpen;
      state.fileEditorPanelOpen = next;
      if (next) {
        state.gitPanelOpen = false;
        state.contextPanelOpen = false;
        state.sidecarPanelOpen = false;
      }
    }),

  setSessionBrowserOpen: (open) =>
    set((state) => {
      state.sessionBrowserOpen = open;
    }),
});

// Selectors
export const selectGitPanelOpen = <T extends PanelState>(state: T): boolean => state.gitPanelOpen;

export const selectContextPanelOpen = <T extends PanelState>(state: T): boolean =>
  state.contextPanelOpen;

export const selectFileEditorPanelOpen = <T extends PanelState>(state: T): boolean =>
  state.fileEditorPanelOpen;

export const selectSidecarPanelOpen = <T extends PanelState>(state: T): boolean =>
  state.sidecarPanelOpen;

export const selectSessionBrowserOpen = <T extends PanelState>(state: T): boolean =>
  state.sessionBrowserOpen;
