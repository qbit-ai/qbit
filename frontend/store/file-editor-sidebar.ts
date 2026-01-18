import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

export interface EditorFileState {
  path: string;
  content: string;
  originalContent: string;
  language?: string;
  dirty: boolean;
  lastReadAt?: string;
  lastSavedAt?: string;
}

export interface FileEditorSessionState {
  open: boolean;
  width: number;
  // Multi-file support
  openFiles: Record<string, EditorFileState>; // keyed by path
  activeFilePath: string | null; // which tab is selected
  tabOrder: string[]; // ordered list of paths for tab display
  // Legacy compatibility getter
  recentFiles: string[];
  vimMode: boolean;
  vimModeState: "normal" | "insert" | "visual";
  wrap: boolean;
  lineNumbers: boolean;
  relativeLineNumbers: boolean;
  status?: string;
}

interface FileEditorSidebarState {
  sessions: Record<string, FileEditorSessionState>;
  ensureSession: (sessionId: string) => FileEditorSessionState;
  setOpen: (sessionId: string, open: boolean) => void;
  setWidth: (sessionId: string, width: number) => void;
  setStatus: (sessionId: string, status?: string) => void;
  // Multi-file operations
  openFile: (sessionId: string, file: EditorFileState) => void;
  setActiveFile: (sessionId: string, path: string) => void;
  updateFileContent: (sessionId: string, path: string, content: string) => void;
  markFileSaved: (sessionId: string, path: string, timestamp?: string) => void;
  closeFile: (sessionId: string, path?: string) => void;
  closeAllFiles: (sessionId: string) => void;
  closeOtherFiles: (sessionId: string, keepPath: string) => void;
  reorderTabs: (sessionId: string, fromIndex: number, toIndex: number) => void;
  // Editor settings
  setVimMode: (sessionId: string, enabled: boolean) => void;
  setVimModeState: (sessionId: string, state: "normal" | "insert" | "visual") => void;
  setWrap: (sessionId: string, enabled: boolean) => void;
  setLineNumbers: (sessionId: string, enabled: boolean) => void;
  setRelativeLineNumbers: (sessionId: string, enabled: boolean) => void;
  addRecentFile: (sessionId: string, path: string) => void;
  resetSession: (sessionId: string) => void;
}

const DEFAULT_WIDTH = 420;

function createDefaultSessionState(): FileEditorSessionState {
  return {
    open: false,
    width: DEFAULT_WIDTH,
    openFiles: {},
    activeFilePath: null,
    tabOrder: [],
    recentFiles: [],
    vimMode: true,
    vimModeState: "normal",
    wrap: false,
    lineNumbers: true,
    relativeLineNumbers: false,
    status: undefined,
  };
}

export const useFileEditorSidebarStore = create<FileEditorSidebarState>()(
  immer((set, get) => ({
    sessions: {},

    ensureSession: (sessionId) => {
      const state = get();
      if (!state.sessions[sessionId]) {
        set((draft) => {
          draft.sessions[sessionId] = createDefaultSessionState();
        });
      }
      return get().sessions[sessionId] ?? createDefaultSessionState();
    },

    setOpen: (sessionId, open) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, open };
      });
    },

    setWidth: (sessionId, width) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, width };
      });
    },

    setStatus: (sessionId, status) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, status };
      });
    },

    openFile: (sessionId, file) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const path = file.path;

        // Add to openFiles
        session.openFiles[path] = file;

        // Add to tabOrder if not already present
        if (!session.tabOrder.includes(path)) {
          session.tabOrder.push(path);
        }

        // Set as active
        session.activeFilePath = path;

        // Auto-open sidebar
        session.open = true;

        // Add to recent files
        session.recentFiles = [path, ...session.recentFiles.filter((p) => p !== path)].slice(0, 10);

        draft.sessions[sessionId] = session;
      });
    },

    setActiveFile: (sessionId, path) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        if (session.openFiles[path]) {
          session.activeFilePath = path;
        }
      });
    },

    updateFileContent: (sessionId, path, content) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const file = session.openFiles[path];
        if (!file) return;
        file.content = content;
        file.dirty = content !== file.originalContent;
      });
    },

    markFileSaved: (sessionId, path, timestamp) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const file = session.openFiles[path];
        if (!file) return;
        file.dirty = false;
        file.originalContent = file.content;
        file.lastSavedAt = timestamp ?? new Date().toISOString();
      });
    },

    closeFile: (sessionId, path) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;

        // If no path specified, close active file
        const targetPath = path ?? session.activeFilePath;
        if (!targetPath) return;

        // Remove from openFiles
        delete session.openFiles[targetPath];

        // Remove from tabOrder
        const tabIndex = session.tabOrder.indexOf(targetPath);
        if (tabIndex !== -1) {
          session.tabOrder.splice(tabIndex, 1);
        }

        // Update activeFilePath
        if (session.activeFilePath === targetPath) {
          if (session.tabOrder.length === 0) {
            session.activeFilePath = null;
          } else {
            // Select adjacent tab (prefer previous, fallback to next)
            const newIndex = Math.min(tabIndex, session.tabOrder.length - 1);
            session.activeFilePath = session.tabOrder[newIndex] ?? null;
          }
        }
      });
    },

    closeAllFiles: (sessionId) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        session.openFiles = {};
        session.tabOrder = [];
        session.activeFilePath = null;
      });
    },

    closeOtherFiles: (sessionId, keepPath) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        const fileToKeep = session.openFiles[keepPath];
        if (!fileToKeep) return;
        session.openFiles = { [keepPath]: fileToKeep };
        session.tabOrder = [keepPath];
        session.activeFilePath = keepPath;
      });
    },

    reorderTabs: (sessionId, fromIndex, toIndex) => {
      set((draft) => {
        const session = draft.sessions[sessionId];
        if (!session) return;
        if (fromIndex < 0 || fromIndex >= session.tabOrder.length) return;
        if (toIndex < 0 || toIndex >= session.tabOrder.length) return;
        const [removed] = session.tabOrder.splice(fromIndex, 1);
        if (removed) {
          session.tabOrder.splice(toIndex, 0, removed);
        }
      });
    },

    setVimMode: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, vimMode: enabled };
      });
    },

    setVimModeState: (sessionId, state) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, vimModeState: state };
      });
    },

    setWrap: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, wrap: enabled };
      });
    },

    setLineNumbers: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, lineNumbers: enabled };
      });
    },

    setRelativeLineNumbers: (sessionId, enabled) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, relativeLineNumbers: enabled };
      });
    },

    addRecentFile: (sessionId, path) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const existing = session.recentFiles.filter((p) => p !== path);
        draft.sessions[sessionId] = {
          ...session,
          recentFiles: [path, ...existing].slice(0, 10),
        };
      });
    },

    resetSession: (sessionId) => {
      set((draft) => {
        draft.sessions[sessionId] = createDefaultSessionState();
      });
    },
  }))
);

// Cached default state to avoid creating new objects on every selector call
// (prevents infinite loop in React's useSyncExternalStore)
const DEFAULT_SESSION_STATE: FileEditorSessionState = Object.freeze({
  ...createDefaultSessionState(),
  openFiles: Object.freeze({}) as unknown as Record<string, EditorFileState>,
  tabOrder: Object.freeze([]) as unknown as string[],
  recentFiles: Object.freeze([]) as unknown as string[],
});

export function selectSessionState(state: FileEditorSidebarState, sessionId: string) {
  return state.sessions[sessionId] ?? DEFAULT_SESSION_STATE;
}

// Helper to get active file from session state
export function selectActiveFile(session: FileEditorSessionState): EditorFileState | null {
  if (!session.activeFilePath) return null;
  return session.openFiles[session.activeFilePath] ?? null;
}
