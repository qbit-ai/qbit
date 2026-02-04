import { useEffect, useState } from "react";
import { logger } from "@/lib/logger";
import { buildFileIndex, type FileIndex } from "@/lib/fileIndex";
import {
  getAllIndexedFiles,
  getIndexerWorkspace,
  indexDirectory,
  initIndexer,
  isIndexerInitialized,
} from "@/lib/indexer";

/**
 * React hook to fetch and cache the file index for a workspace.
 * Automatically initializes the indexer if needed.
 * Returns a FileIndex for fast file path validation, or null while loading/on error.
 *
 * @param workspaceRoot - The workspace root directory to fetch files for
 * @returns FileIndex for O(1) path lookups, or null if not available
 */
export function useFileIndex(workspaceRoot: string | undefined): FileIndex | null {
  const [fileIndex, setFileIndex] = useState<FileIndex | null>(null);

  useEffect(() => {
    if (!workspaceRoot || workspaceRoot === "") {
      setFileIndex(null);
      return;
    }

    let cancelled = false;

    async function ensureIndexerAndFetch() {
      try {
        // Check if indexer is initialized for the current workspace
        const initialized = await isIndexerInitialized();
        const currentWorkspace = initialized ? await getIndexerWorkspace() : null;

        // Initialize indexer if not initialized or if workspace changed
        if (!initialized || currentWorkspace !== workspaceRoot) {
          // We've already checked workspaceRoot is truthy at the start of the effect
          const root = workspaceRoot as string;
          await initIndexer(root);
          // Index the directory in background (don't await to avoid blocking UI)
          indexDirectory(root).catch((err) => {
            logger.warn("Background indexing failed:", err);
          });
        }

        // Fetch the file list
        const files = await getAllIndexedFiles();
        if (!cancelled && workspaceRoot) {
          const index = buildFileIndex(files, workspaceRoot);
          setFileIndex(index);
        }
      } catch (error) {
        logger.error("Failed to initialize/fetch file index:", error);
        if (!cancelled) {
          setFileIndex(null);
        }
      }
    }

    ensureIndexerAndFetch();

    return () => {
      cancelled = true;
    };
  }, [workspaceRoot]);

  return fileIndex;
}
