import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { readWorkspaceFile, unwatchAllFiles, unwatchFile, watchFile } from "@/lib/file-editor";
import {
  fileTabIdFromPath,
  useFileEditorSidebarStore,
} from "@/store/file-editor-sidebar";

interface FileChangedPayload {
  path: string;
  modifiedAt: string | null;
}

/**
 * Watches open file tabs for external filesystem changes.
 * - Auto-reloads files that have no pending local edits.
 * - Marks files as externally modified when local edits exist (triggers conflict banner).
 */
export function useFileWatcher() {
  const watchedPathsRef = useRef(new Set<string>());

  // Sync watches with open file tabs
  useEffect(() => {
    const store = useFileEditorSidebarStore.getState();
    const currentPaths = new Set<string>();

    // Collect all open file tab paths
    for (const tab of Object.values(store.tabs)) {
      if (tab.type === "file") {
        currentPaths.add(tab.file.path);
      }
    }

    // Watch new files
    for (const path of currentPaths) {
      if (!watchedPathsRef.current.has(path)) {
        watchFile(path).catch(() => {});
        watchedPathsRef.current.add(path);
      }
    }

    // Unwatch closed files
    for (const path of watchedPathsRef.current) {
      if (!currentPaths.has(path)) {
        unwatchFile(path).catch(() => {});
        watchedPathsRef.current.delete(path);
      }
    }
  });

  // Subscribe to store changes to watch/unwatch as tabs open/close
  useEffect(() => {
    const unsub = useFileEditorSidebarStore.subscribe((state) => {
      const currentPaths = new Set<string>();

      for (const tab of Object.values(state.tabs)) {
        if (tab.type === "file") {
          currentPaths.add(tab.file.path);
        }
      }

      // Watch new files
      for (const path of currentPaths) {
        if (!watchedPathsRef.current.has(path)) {
          watchFile(path).catch(() => {});
          watchedPathsRef.current.add(path);
        }
      }

      // Unwatch closed files
      for (const path of watchedPathsRef.current) {
        if (!currentPaths.has(path)) {
          unwatchFile(path).catch(() => {});
          watchedPathsRef.current.delete(path);
        }
      }
    });

    return () => {
      unsub();
      // Cleanup: unwatch all on unmount
      unwatchAllFiles().catch(() => {});
      watchedPathsRef.current.clear();
    };
  }, []);

  // Listen for file-changed events from the backend
  useEffect(() => {
    const unlisten = listen<FileChangedPayload>("file-changed", async (event) => {
      const { path, modifiedAt } = event.payload;
      const store = useFileEditorSidebarStore.getState();
      const tabId = fileTabIdFromPath(path);
      const tab = store.tabs[tabId];

      if (!tab || tab.type !== "file") return;

      const file = tab.file;

      // Skip if the modifiedAt matches our last known save time
      // (this means WE wrote the file, not an external change)
      if (modifiedAt && file.lastSavedAt && modifiedAt === file.lastSavedAt) {
        return;
      }

      if (file.dirty) {
        // File has unsaved local changes — show conflict banner
        store.markExternallyModified(tabId);
      } else {
        // No local changes — auto-reload silently
        try {
          const result = await readWorkspaceFile(path);
          store.acceptExternalChange(tabId, result.content, result.modifiedAt);
        } catch {
          // File may have been deleted; ignore
        }
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
