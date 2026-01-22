import { open } from "@tauri-apps/plugin-dialog";
import { Check, FolderPlus, Loader2, RefreshCw, Trash2, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  addIndexedCodebase,
  type CodebaseInfo,
  detectMemoryFiles,
  listIndexedCodebases,
  reindexCodebase,
  removeIndexedCodebase,
  updateCodebaseMemoryFile,
} from "@/lib/indexer";
import { logger } from "@/lib/logger";
import { notify } from "@/lib/notify";

const MEMORY_FILE_OPTIONS = [
  { value: "none", label: "None" },
  { value: "AGENTS.md", label: "AGENTS.md" },
  { value: "CLAUDE.md", label: "CLAUDE.md" },
] as const;

export function CodebasesSettings() {
  const [codebases, setCodebases] = useState<CodebaseInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isAdding, setIsAdding] = useState(false);
  const [reindexingPaths, setReindexingPaths] = useState<Set<string>>(new Set());
  const [removingPaths, setRemovingPaths] = useState<Set<string>>(new Set());
  const [updatingMemoryFilePaths, setUpdatingMemoryFilePaths] = useState<Set<string>>(new Set());

  // Load codebases on mount
  const loadCodebases = useCallback(async () => {
    try {
      const list = await listIndexedCodebases();

      // Auto-detect memory files for codebases that don't have one set
      const updatedList = await Promise.all(
        list.map(async (codebase) => {
          if (codebase.memory_file === undefined || codebase.memory_file === null) {
            try {
              const detected = await detectMemoryFiles(codebase.path);
              if (detected) {
                // Update the backend with the detected value
                await updateCodebaseMemoryFile(codebase.path, detected);
                return { ...codebase, memory_file: detected };
              }
            } catch (err) {
              logger.error(`Failed to detect memory files for ${codebase.path}:`, err);
            }
          }
          return codebase;
        })
      );

      setCodebases(updatedList);
    } catch (err) {
      logger.error("Failed to load codebases:", err);
      notify.error("Failed to load indexed codebases");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadCodebases();
  }, [loadCodebases]);

  // Add new codebase via folder picker
  const handleAddCodebase = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select folder to index",
      });

      if (!selected) {
        return; // User cancelled
      }

      setIsAdding(true);
      const result = await addIndexedCodebase(selected);

      // Try to auto-detect memory file for the new codebase
      try {
        const detected = await detectMemoryFiles(result.path);
        if (detected) {
          await updateCodebaseMemoryFile(result.path, detected);
          result.memory_file = detected;
        }
      } catch (err) {
        logger.error(`Failed to detect memory files for ${result.path}:`, err);
      }

      setCodebases((prev) => [...prev, result]);
      notify.success(`Added ${result.path}`);
    } catch (err) {
      logger.error("Failed to add codebase:", err);
      notify.error(err instanceof Error ? err.message : "Failed to add codebase");
    } finally {
      setIsAdding(false);
    }
  }, []);

  // Reindex a codebase
  const handleReindex = useCallback(async (path: string) => {
    setReindexingPaths((prev) => new Set(prev).add(path));
    try {
      const result = await reindexCodebase(path);
      setCodebases((prev) => prev.map((cb) => (cb.path === path ? result : cb)));
      notify.success(`Re-indexed ${path}`);
    } catch (err) {
      logger.error("Failed to reindex:", err);
      notify.error(err instanceof Error ? err.message : "Failed to reindex");
    } finally {
      setReindexingPaths((prev) => {
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
    }
  }, []);

  // Remove a codebase
  const handleRemove = useCallback(async (path: string) => {
    setRemovingPaths((prev) => new Set(prev).add(path));
    try {
      await removeIndexedCodebase(path);
      setCodebases((prev) => prev.filter((cb) => cb.path !== path));
      notify.success(`Removed ${path}`);
    } catch (err) {
      logger.error("Failed to remove codebase:", err);
      notify.error(err instanceof Error ? err.message : "Failed to remove codebase");
    } finally {
      setRemovingPaths((prev) => {
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
    }
  }, []);

  // Update memory file selection
  const handleMemoryFileChange = useCallback(async (path: string, value: string) => {
    setUpdatingMemoryFilePaths((prev) => new Set(prev).add(path));
    try {
      const memoryFile = value === "none" ? null : value;
      await updateCodebaseMemoryFile(path, memoryFile);
      setCodebases((prev) =>
        prev.map((cb) => (cb.path === path ? { ...cb, memory_file: memoryFile ?? undefined } : cb))
      );
    } catch (err) {
      logger.error("Failed to update memory file:", err);
      notify.error(err instanceof Error ? err.message : "Failed to update memory file");
    } finally {
      setUpdatingMemoryFilePaths((prev) => {
        const next = new Set(prev);
        next.delete(path);
        return next;
      });
    }
  }, []);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="w-6 h-6 text-muted-foreground animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-1">
          <h3 className="text-sm font-medium text-foreground">Indexed folders</h3>
          <p className="text-xs text-muted-foreground">
            Manage codebases indexed for AI context and code search
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={handleAddCodebase} disabled={isAdding}>
          {isAdding ? (
            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
          ) : (
            <FolderPlus className="w-4 h-4 mr-2" />
          )}
          Index new folder
        </Button>
      </div>

      {/* Codebase list */}
      {codebases.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground text-sm">
          No codebases indexed yet. Click "Index new folder" to add one.
        </div>
      ) : (
        <div className="space-y-2">
          {codebases.map((codebase) => {
            const isReindexing = reindexingPaths.has(codebase.path);
            const isRemoving = removingPaths.has(codebase.path);
            const isUpdatingMemoryFile = updatingMemoryFilePaths.has(codebase.path);
            const isDisabled = isReindexing || isRemoving;

            return (
              <div
                key={codebase.path}
                className="flex items-center justify-between px-4 py-3 rounded-lg border border-[var(--color-border-medium)] bg-[var(--bg-secondary)]"
              >
                <div className="flex-1 min-w-0 mr-4">
                  <div className="text-sm font-medium text-foreground truncate">
                    {codebase.path}
                  </div>
                  <div className="flex items-center gap-2 mt-1">
                    {codebase.status === "synced" && (
                      <>
                        <Check className="w-3 h-3 text-green-500" />
                        <span className="text-xs text-green-600">Synced</span>
                      </>
                    )}
                    {codebase.status === "not_indexed" && (
                      <>
                        <XCircle className="w-3 h-3 text-amber-500" />
                        <span className="text-xs text-amber-600">Not indexed</span>
                      </>
                    )}
                    {codebase.status === "indexing" && (
                      <>
                        <Loader2 className="w-3 h-3 text-blue-500 animate-spin" />
                        <span className="text-xs text-blue-600">Indexing...</span>
                      </>
                    )}
                    {codebase.status === "error" && (
                      <>
                        <XCircle className="w-3 h-3 text-red-500" />
                        <span className="text-xs text-red-600">{codebase.error || "Error"}</span>
                      </>
                    )}
                    {codebase.file_count > 0 && (
                      <span className="text-xs text-muted-foreground">
                        ({codebase.file_count.toLocaleString()} files)
                      </span>
                    )}
                  </div>
                </div>

                {/* Memory File Dropdown */}
                <div className="flex items-center gap-2 mr-2">
                  <Select
                    value={codebase.memory_file ?? "none"}
                    onValueChange={(value) => handleMemoryFileChange(codebase.path, value)}
                    disabled={isDisabled || isUpdatingMemoryFile}
                  >
                    <SelectTrigger size="sm" className="w-[130px]">
                      {isUpdatingMemoryFile ? (
                        <Loader2 className="w-3 h-3 animate-spin" />
                      ) : (
                        <SelectValue placeholder="Memory file" />
                      )}
                    </SelectTrigger>
                    <SelectContent>
                      {MEMORY_FILE_OPTIONS.map((option) => (
                        <SelectItem key={option.value} value={option.value}>
                          {option.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => handleReindex(codebase.path)}
                    disabled={isDisabled}
                    title="Re-index"
                  >
                    {isReindexing ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <RefreshCw className="w-4 h-4" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-destructive hover:text-destructive"
                    onClick={() => handleRemove(codebase.path)}
                    disabled={isDisabled}
                    title="Remove"
                  >
                    {isRemoving ? (
                      <Loader2 className="w-4 h-4 animate-spin" />
                    ) : (
                      <Trash2 className="w-4 h-4" />
                    )}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
