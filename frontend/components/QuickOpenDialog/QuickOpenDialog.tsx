import { FileText } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useFileEditorSidebar } from "@/hooks/useFileEditorSidebar";
import { type FileInfo, listWorkspaceFiles } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useFocusedSessionId, useStore } from "@/store";

interface QuickOpenDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workingDirectory?: string;
}

export function QuickOpenDialog({
  open,
  onOpenChange,
  workingDirectory: workingDirectoryProp,
}: QuickOpenDialogProps) {
  const activeSessionId = useStore((state) => state.activeSessionId);
  const focusedSessionId = useFocusedSessionId(activeSessionId);
  const storeWorkingDirectory = useStore((state) =>
    focusedSessionId ? state.sessions[focusedSessionId]?.workingDirectory : undefined
  );
  const workingDirectory = workingDirectoryProp ?? storeWorkingDirectory;
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<FileInfo[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isSearching, setIsSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const { openFile } = useFileEditorSidebar(workingDirectory);

  // Reset state when opening
  useEffect(() => {
    if (open) {
      setQuery("");
      setResults([]);
      setSelectedIndex(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Search files as user types
  useEffect(() => {
    if (!open || !query.trim() || !workingDirectory) {
      setResults([]);
      setSelectedIndex(0);
      return;
    }

    let cancelled = false;
    const timer = setTimeout(async () => {
      setIsSearching(true);
      try {
        const files = await listWorkspaceFiles(workingDirectory, query, 50);
        if (!cancelled) {
          setResults(files);
          setSelectedIndex(0);
        }
      } catch {
        if (!cancelled) {
          setResults([]);
        }
      } finally {
        if (!cancelled) {
          setIsSearching(false);
        }
      }
    }, 100);

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [open, query, workingDirectory]);

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selected = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
      selected?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  const handleSelect = useCallback(
    (file: FileInfo) => {
      onOpenChange(false);
      const fullPath = workingDirectory
        ? `${workingDirectory.replace(/\/$/, "")}/${file.relative_path}`
        : file.relative_path;
      void openFile(fullPath);
    },
    [onOpenChange, openFile, workingDirectory]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        const selected = results[selectedIndex];
        if (selected) {
          handleSelect(selected);
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        onOpenChange(false);
      }
    },
    [results, selectedIndex, handleSelect, onOpenChange]
  );

  if (!open) return null;

  return createPortal(
    <>
      {/* Backdrop */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: backdrop dismiss */}
      <div className="fixed inset-0 z-50 bg-black/50" onClick={() => onOpenChange(false)} />

      {/* Dialog */}
      <div className="fixed left-1/2 top-[20%] -translate-x-1/2 z-50 w-[500px] max-w-[90vw]">
        <div className="bg-popover border border-border rounded-lg shadow-xl overflow-hidden">
          {/* Search input */}
          <div className="flex items-center border-b border-border px-3">
            <FileText className="w-4 h-4 text-muted-foreground shrink-0" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Search files by name..."
              className="flex-1 bg-transparent py-3 px-2 text-sm outline-none text-foreground placeholder:text-muted-foreground"
            />
            {isSearching && (
              <span className="text-xs text-muted-foreground animate-pulse">...</span>
            )}
          </div>

          {/* Results */}
          <div ref={listRef} className="max-h-[300px] overflow-y-auto py-1" role="listbox">
            {query && results.length === 0 && !isSearching && (
              <div className="py-6 text-center text-sm text-muted-foreground">No files found</div>
            )}
            {results.map((file, index) => (
              <div
                key={file.relative_path}
                role="option"
                aria-selected={index === selectedIndex}
                data-index={index}
                onClick={() => handleSelect(file)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleSelect(file);
                }}
                tabIndex={-1}
                className={cn(
                  "flex items-center gap-2 px-3 py-1.5 cursor-pointer transition-colors",
                  index === selectedIndex ? "bg-primary/10" : "hover:bg-muted/50"
                )}
              >
                <FileText className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                <span className="text-sm text-foreground truncate">{file.name}</span>
                {file.relative_path !== file.name && (
                  <span className="text-xs text-muted-foreground truncate ml-auto">
                    {file.relative_path.slice(0, -(file.name.length + 1))}
                  </span>
                )}
              </div>
            ))}
          </div>

          {/* Footer hint */}
          <div className="px-3 py-1.5 border-t border-border text-[11px] text-muted-foreground flex items-center gap-3">
            <span>
              <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">↑↓</kbd> navigate
            </span>
            <span>
              <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">↵</kbd> open
            </span>
            <span>
              <kbd className="px-1 py-0.5 bg-muted rounded text-[10px]">esc</kbd> close
            </span>
          </div>
        </div>
      </div>
    </>,
    document.body
  );
}
