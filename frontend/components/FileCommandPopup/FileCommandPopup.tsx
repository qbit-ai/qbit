import { useEffect, useRef } from "react";
import type { FileInfo } from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface FileCommandPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Already-filtered files to display */
  files: FileInfo[];
  selectedIndex: number;
  onSelect: (file: FileInfo) => void;
  containerRef: React.RefObject<HTMLElement | null>;
}

export function FileCommandPopup({
  open,
  onOpenChange,
  files,
  selectedIndex,
  onSelect,
  containerRef,
}: FileCommandPopupProps) {
  const listRef = useRef<HTMLDivElement>(null);

  // Close popup when clicking outside
  useEffect(() => {
    if (!open) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onOpenChange(false);
      }
    };

    // Use capture phase to catch clicks before they're handled
    document.addEventListener("mousedown", handleClickOutside, true);
    return () => document.removeEventListener("mousedown", handleClickOutside, true);
  }, [open, onOpenChange, containerRef]);

  // Close popup when window loses focus (e.g., switching tabs)
  useEffect(() => {
    if (!open) return;

    const handleBlur = () => onOpenChange(false);
    window.addEventListener("blur", handleBlur);
    return () => window.removeEventListener("blur", handleBlur);
  }, [open, onOpenChange]);

  // Scroll selected item into view
  useEffect(() => {
    if (open && listRef.current) {
      const selectedElement = listRef.current.querySelector(`[data-index="${selectedIndex}"]`);
      selectedElement?.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex, open]);

  if (!open) return null;

  return (
    <div className="absolute bottom-full left-0 mb-2 w-[400px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden">
      {files.length === 0 ? (
        <div className="py-3 text-center text-sm text-muted-foreground">No files found</div>
      ) : (
        <div ref={listRef} className="max-h-[200px] overflow-y-auto py-1" role="listbox">
          {files.map((file, index) => (
            <div
              key={file.relative_path}
              role="option"
              aria-selected={index === selectedIndex}
              tabIndex={0}
              data-index={index}
              onClick={() => onSelect(file)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelect(file);
                }
              }}
              className={cn(
                "flex flex-col gap-0.5 px-3 py-2",
                "cursor-pointer transition-colors",
                index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
              )}
            >
              <span className="font-mono text-sm text-foreground">{file.name}</span>
              <span className="font-mono text-xs text-muted-foreground truncate">
                {file.relative_path}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// Export helper to filter files by query
export function filterFiles(files: FileInfo[], query: string): FileInfo[] {
  if (!query) return files;
  const lowerQuery = query.toLowerCase();
  return files.filter(
    (file) =>
      file.name.toLowerCase().includes(lowerQuery) ||
      file.relative_path.toLowerCase().includes(lowerQuery)
  );
}
