import { useCallback, useEffect, useState } from "react";
import { logger } from "@/lib/logger";
import { type FileInfo, listWorkspaceFiles } from "@/lib/tauri";

export function useFileCommands(workingDirectory?: string, query?: string) {
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  const loadFiles = useCallback(async () => {
    if (!workingDirectory) {
      setFiles([]);
      return;
    }

    setIsLoading(true);
    try {
      const result = await listWorkspaceFiles(workingDirectory, query, 5);
      setFiles(result);
    } catch (error) {
      logger.error("Failed to load files:", error);
      setFiles([]);
    } finally {
      setIsLoading(false);
    }
  }, [workingDirectory, query]);

  // Load files when working directory or query changes
  useEffect(() => {
    loadFiles();
  }, [loadFiles]);

  return { files, isLoading, reload: loadFiles };
}
