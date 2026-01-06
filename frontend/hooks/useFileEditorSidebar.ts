import { useCallback, useEffect, useMemo } from "react";
import { readWorkspaceFile, writeWorkspaceFile } from "@/lib/file-editor";
import { notify } from "@/lib/notify";
import {
  type EditorFileState,
  selectSessionState,
  useFileEditorSidebarStore,
} from "@/store/file-editor-sidebar";

function resolvePath(input: string, workingDirectory?: string) {
  if (!workingDirectory) return input;
  if (input.startsWith("/") || /^\w:[\\/]/.test(input)) return input;
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
      updateContent: (content: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().updateContent(sessionId, content);
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
      addRecentFile: (path: string) => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().addRecentFile(sessionId, path);
      },
      closeFile: () => {
        if (!sessionId) return;
        useFileEditorSidebarStore.getState().closeFile(sessionId);
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

  const saveActiveFile = useCallback(async () => {
    if (!sessionId) return;
    if (!session?.activeFile) {
      notify.info("No file open to save");
      return;
    }
    const file = session.activeFile;
    actions.setStatus("Saving...");
    try {
      const result = await writeWorkspaceFile(file.path, file.content, {
        expectedModifiedAt: file.lastSavedAt,
      });
      useFileEditorSidebarStore.getState().markSaved(sessionId, result.modifiedAt);
      notify.success("Saved");
    } catch (error) {
      notify.error(`Failed to save file: ${error}`);
    } finally {
      actions.setStatus(undefined);
    }
  }, [actions, session?.activeFile, sessionId]);

  const reloadActiveFile = useCallback(async () => {
    if (!sessionId) return;
    if (!session?.activeFile) {
      notify.info("No file open to reload");
      return;
    }
    const file = session.activeFile;
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
  }, [actions, session?.activeFile, sessionId]);

  return {
    session,
    openFile,
    saveActiveFile,
    reloadActiveFile,
    setOpen: actions.setOpen,
    setWidth: actions.setWidth,
    setStatus: actions.setStatus,
    updateContent: actions.updateContent,
    setVimMode: actions.setVimMode,
    setWrap: actions.setWrap,
    setVimModeState: actions.setVimModeState,
    closeFile: actions.closeFile,
  };
}
