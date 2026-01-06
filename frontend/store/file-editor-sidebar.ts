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
  activeFile: EditorFileState | null;
  recentFiles: string[];
  vimMode: boolean;
  vimModeState: "normal" | "insert" | "visual";
  wrap: boolean;
  status?: string;
}

interface FileEditorSidebarState {
  sessions: Record<string, FileEditorSessionState>;
  ensureSession: (sessionId: string) => FileEditorSessionState;
  setOpen: (sessionId: string, open: boolean) => void;
  setWidth: (sessionId: string, width: number) => void;
  setStatus: (sessionId: string, status?: string) => void;
  openFile: (sessionId: string, file: EditorFileState) => void;
  updateContent: (sessionId: string, content: string) => void;
  markSaved: (sessionId: string, timestamp?: string) => void;
  setVimMode: (sessionId: string, enabled: boolean) => void;
  setVimModeState: (sessionId: string, state: "normal" | "insert" | "visual") => void;
  setWrap: (sessionId: string, enabled: boolean) => void;
  addRecentFile: (sessionId: string, path: string) => void;
  closeFile: (sessionId: string) => void;
  resetSession: (sessionId: string) => void;
}

const DEFAULT_WIDTH = 420;

function createDefaultSessionState(): FileEditorSessionState {
  return {
    open: false,
    width: DEFAULT_WIDTH,
    activeFile: null,
    recentFiles: [],
    vimMode: true,
    vimModeState: "normal",
    wrap: false,
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
        draft.sessions[sessionId] = {
          ...session,
          activeFile: file,
          recentFiles: [file.path, ...session.recentFiles.filter((p) => p !== file.path)].slice(
            0,
            10
          ),
        };
      });
    },

    updateContent: (sessionId, content) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const activeFile = session.activeFile;
        if (!activeFile) return;
        draft.sessions[sessionId] = {
          ...session,
          activeFile: { ...activeFile, content, dirty: content !== activeFile.originalContent },
        };
      });
    },

    markSaved: (sessionId, timestamp) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        const activeFile = session.activeFile;
        if (!activeFile) return;
        draft.sessions[sessionId] = {
          ...session,
          activeFile: {
            ...activeFile,
            dirty: false,
            originalContent: activeFile.content,
            lastSavedAt: timestamp ?? new Date().toISOString(),
          },
        };
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

    closeFile: (sessionId) => {
      set((draft) => {
        const session = draft.sessions[sessionId] ?? createDefaultSessionState();
        draft.sessions[sessionId] = { ...session, activeFile: null };
      });
    },

    resetSession: (sessionId) => {
      set((draft) => {
        draft.sessions[sessionId] = createDefaultSessionState();
      });
    },
  }))
);

export function selectSessionState(state: FileEditorSidebarState, sessionId: string) {
  return state.sessions[sessionId] ?? createDefaultSessionState();
}
