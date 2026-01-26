import { ChevronRight, Home, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { type DirEntry, listDirectory } from "@/lib/file-editor";
import { type EntryType, getEntryIcon } from "@/lib/file-icons";
import { cn } from "@/lib/utils";

interface FileBrowserProps {
  currentPath: string;
  workingDirectory?: string;
  onNavigate: (path: string) => void;
  onOpenFile: (path: string) => void;
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function getParentPath(path: string): string {
  const parts = path.split("/").filter(Boolean);
  if (parts.length <= 1) return "/";
  parts.pop();
  return `/${parts.join("/")}`;
}

function getBreadcrumbs(path: string, workingDirectory?: string): { name: string; path: string }[] {
  const parts = path.split("/").filter(Boolean);
  const crumbs: { name: string; path: string }[] = [];

  let currentPath = "";
  for (const part of parts) {
    currentPath += `/${part}`;
    crumbs.push({ name: part, path: currentPath });
  }

  // If we're inside the working directory, simplify the display
  if (workingDirectory && path.startsWith(workingDirectory)) {
    const wdParts = workingDirectory.split("/").filter(Boolean);
    // Show workspace as first crumb
    return [{ name: "~", path: workingDirectory }, ...crumbs.slice(wdParts.length)];
  }

  return crumbs;
}

export function FileBrowser({
  currentPath,
  workingDirectory,
  onNavigate,
  onOpenFile,
}: FileBrowserProps) {
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDirectory = useCallback(async (path: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = await listDirectory(path);
      setEntries(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setEntries([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDirectory(currentPath || workingDirectory || "");
  }, [currentPath, workingDirectory, loadDirectory]);

  const handleEntryClick = useCallback(
    (entry: DirEntry) => {
      if (entry.entryType === "directory") {
        onNavigate(entry.path);
      } else {
        onOpenFile(entry.path);
      }
    },
    [onNavigate, onOpenFile]
  );

  const handleGoUp = useCallback(() => {
    const parent = getParentPath(currentPath || workingDirectory || "");
    onNavigate(parent);
  }, [currentPath, workingDirectory, onNavigate]);

  const handleGoHome = useCallback(() => {
    onNavigate(workingDirectory || "");
  }, [workingDirectory, onNavigate]);

  const handleRefresh = useCallback(() => {
    loadDirectory(currentPath || workingDirectory || "");
  }, [currentPath, workingDirectory, loadDirectory]);

  const breadcrumbs = getBreadcrumbs(currentPath || workingDirectory || "", workingDirectory);
  const canGoUp = (currentPath || workingDirectory || "") !== "/" && breadcrumbs.length > 0;

  return (
    <div className="flex flex-col h-full">
      {/* Toolbar */}
      <div className="flex items-center gap-1 px-2 py-1.5 border-b border-border bg-muted/30">
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleGoHome}
          title="Go to workspace root"
        >
          <Home className="w-3.5 h-3.5" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleGoUp}
          disabled={!canGoUp}
          title="Go up"
        >
          <ChevronRight className="w-3.5 h-3.5 rotate-180" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={handleRefresh}
          disabled={loading}
          title="Refresh"
        >
          <RefreshCw className={cn("w-3.5 h-3.5", loading && "animate-spin")} />
        </Button>

        {/* Breadcrumbs */}
        <div className="flex items-center gap-0.5 ml-2 min-w-0 overflow-hidden">
          {breadcrumbs.map((crumb, i) => (
            <div key={crumb.path} className="flex items-center shrink-0">
              {i > 0 && <ChevronRight className="w-3 h-3 text-muted-foreground/50 mx-0.5" />}
              <button
                type="button"
                onClick={() => onNavigate(crumb.path)}
                className="text-xs text-muted-foreground hover:text-foreground truncate max-w-[100px]"
                title={crumb.path}
              >
                {crumb.name}
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto">
        {loading && entries.length === 0 && (
          <div className="flex items-center justify-center h-full text-muted-foreground text-xs">
            Loading...
          </div>
        )}

        {error && (
          <div className="flex items-center justify-center h-full text-destructive text-xs p-4 text-center">
            {error}
          </div>
        )}

        {!loading && !error && entries.length === 0 && (
          <div className="flex items-center justify-center h-full text-muted-foreground text-xs">
            Empty directory
          </div>
        )}

        {entries.length > 0 && (
          <div className="divide-y divide-border/50">
            {entries.map((entry) => (
              <button
                key={entry.path}
                type="button"
                className="w-full flex items-center gap-2 px-3 py-2 hover:bg-muted/50 transition-colors text-left"
                onClick={() => handleEntryClick(entry)}
                onDoubleClick={() => {
                  if (entry.entryType !== "directory") {
                    onOpenFile(entry.path);
                  }
                }}
              >
                {getEntryIcon(entry.entryType as EntryType, entry.name)}
                <span className="flex-1 truncate text-xs">{entry.name}</span>
                {entry.entryType === "file" && entry.size !== undefined && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {formatFileSize(entry.size)}
                  </span>
                )}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
