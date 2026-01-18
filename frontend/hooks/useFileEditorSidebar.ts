import { useCallback, useEffect, useMemo } from "react";
import { readWorkspaceFile, writeWorkspaceFile } from "@/lib/file-editor";
import { notify } from "@/lib/notify";
import {
  type EditorFileState,
  selectActiveFile,
  selectSessionState,
  useFileEditorSidebarStore,
} from "@/store/file-editor-sidebar";

function resolvePath(input: string, workingDirectory?: string) {
  if (!workingDirectory) return input;
  if (input.startsWith("/") || /^\w:[/\\]/.test(input)) return input;
  const trimmedBase = workingDirectory.endsWith("/")
    ? workingDirectory.slice(0, -1)
    : workingDirectory;
  const normalizedInput = input.replace(/^\.\//, "");
  return `${trimmedBase}/${normalizedInput}`;
}

function detectLanguageFromPath(path: string): string | undefined {
  const lower = path.toLowerCase();
  if (lower.endsWith(".ts") || lower.endsWith(".tsx")) return "typescript";
  if (lower.endsWith(".js") || lower.endsWith(".jsx")) return "javascript";
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".md") || lower.endsWith(".mdx")) return "markdown";
  if (lower.endsWith(".py")) return "python";
  if (lower.endsWith(".rs")) return "rust";
  return undefined;
}

export function useFileEditorSidebar(sessionId: string | null, workingDirectory?: string) {
  useEffect(() => {
    if (sessionId) {
      useFileEditorSidebarStore.getState().ensureSession(sessionId);
    }
  }, [sessionId]);

  const session = useFileEditorSidebarStore(
    useCallback((state) => (sessionId ? selectSessionState(state, sessionId) : null), [sessionId])
  );

  // Derive active file from session
  const activeFile = useMemo(() => (session ? selectActiveFile(session) : null), [session]);

  const actions = useMemo(() => {
    return {
      setOpen: (open: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setOpen(sessionId, open);
      },
      setWidth: (width: number) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setWidth(sessionId, width);
      },
      setStatus: (status?: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setStatus(sessionId, status);
      },
      setActiveFile: (path: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setActiveFile(sessionId, path);
      },
      updateFileContent: (path: string, content: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().updateFileContent(sessionId, path, content);
      },
      setVimMode: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setVimMode(sessionId, enabled);
      },
      setVimModeState: (state: "normal" | "insert" | "visual") => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setVimModeState(sessionId, state);
      },
      setWrap: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setWrap(sessionId, enabled);
      },
      setLineNumbers: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setLineNumbers(sessionId, enabled);
      },
      setRelativeLineNumbers: (enabled: boolean) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().setRelativeLineNumbers(sessionId, enabled);
      },
      addRecentFile: (path: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().addRecentFile(sessionId, path);
      },
      closeFile: (path?: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeFile(sessionId, path);
      },
      closeAllFiles: () => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeAllFiles(sessionId);
      },
      closeOtherFiles: (keepPath: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeOtherFiles(sessionId, keepPath);
      },
      reorderTabs: (fromIndex: number, toIndex: number) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().reorderTabs(sessionId, fromIndex, toIndex);
      },
    };
  }, [sessionId]);

  const openFile = useCallback(
    async (inputPath: string) => {
      if (!sessionId) {
        notify.error("No active session for file open");
        return;
      }
      const fullPath = resolvePath(inputPath, workingDirectory);

      // If file is already open, just switch to it
      const state = useFileEditorSidebarStore.getState();
      const currentSession = state.sessions[sessionId];
      if (currentSession?.openFiles[fullPath]) {
        state.setActiveFile(sessionId, fullPath);
        return;
      }

      actions.setStatus("Loading file...");
      try {
        const result = await readWorkspaceFile(fullPath);
        const file: EditorFileState = {
          path: fullPath,
          content: result.content,
          originalContent: result.content,
          language: detectLanguageFromPath(fullPath),
          dirty: false,
          lastReadAt: new Date().toISOString(),
          lastSavedAt: result.modifiedAt,
        };
        useFileEditorSidebarStore.getState().openFile(sessionId, file);
        actions.addRecentFile(fullPath);
      } catch (error) {
        notify.error(`Failed to open file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, sessionId, workingDirectory]
  );

  const saveFile = useCallback(
    async (path?: string) => {
      if (!sessionId || !session) return;

      const targetPath = path ?? session.activeFilePath;
      if (!targetPath) {
        notify.info("No file open to save");
        return;
      }

      const file = session.openFiles[targetPath];
      if (!file) {
        notify.error("File not found");
        return;
      }

      actions.setStatus("Saving...");
      try {
        const result = await writeWorkspaceFile(file.path, file.content, {
          expectedModifiedAt: file.lastSavedAt,
        });
        useFileEditorSidebarStore
          .getState()
          .markFileSaved(sessionId, targetPath, result.modifiedAt);
        notify.success("Saved");
      } catch (error) {
        notify.error(`Failed to save file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, session, sessionId]
  );

  const reloadFile = useCallback(
    async (path?: string) => {
      if (!sessionId || !session) return;

      const targetPath = path ?? session.activeFilePath;
      if (!targetPath) {
        notify.info("No file open to reload");
        return;
      }

      const file = session.openFiles[targetPath];
      if (!file) {
        notify.error("File not found");
        return;
      }

      actions.setStatus("Reloading...");
      try {
        const result = await readWorkspaceFile(file.path);
        const newFile: EditorFileState = {
          ...file,
          content: result.content,
          originalContent: result.content,
          dirty: false,
          lastReadAt: new Date().toISOString(),
          lastSavedAt: result.modifiedAt,
        };
        useFileEditorSidebarStore.getState().openFile(sessionId, newFile);
      } catch (error) {
        notify.error(`Failed to reload file: ${error}`);
      } finally {
        actions.setStatus(undefined);
      }
    },
    [actions, session, sessionId]
  );

  return {
    session,
    activeFile,
    // File operations
    openFile,
    saveFile,
    reloadFile,
    // Tab operations
    setActiveFile: actions.setActiveFile,
    closeFile: actions.closeFile,
    closeAllFiles: actions.closeAllFiles,
    closeOtherFiles: actions.closeOtherFiles,
    reorderTabs: actions.reorderTabs,
    // Editor state
    setOpen: actions.setOpen,
    setWidth: actions.setWidth,
    setStatus: actions.setStatus,
    updateFileContent: actions.updateFileContent,
    setVimMode: actions.setVimMode,
    setVimModeState: actions.setVimModeState,
    setWrap: actions.setWrap,
    setLineNumbers: actions.setLineNumbers,
    setRelativeLineNumbers: actions.setRelativeLineNumbers,
  };
}
