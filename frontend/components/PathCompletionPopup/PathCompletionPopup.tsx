import { File, Folder, Link2 } from "lucide-react";
import { useEffect, useRef } from "react";
import type { PathCompletion } from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface PathCompletionPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  completions: PathCompletion[];
  selectedIndex: number;
  onSelect: (completion: PathCompletion) => void;
  children: React.ReactNode;
}

function getIcon(entryType: PathCompletion["entry_type"]) {
  switch (entryType) {
    case "directory":
      return <Folder className="h-4 w-4 text-blue-500" />;
    case "symlink":
      return <Link2 className="h-4 w-4 text-cyan-500" />;
    default:
      return <File className="h-4 w-4 text-muted-foreground" />;
  }
}

export function PathCompletionPopup({
  open,
  onOpenChange,
  completions,
  selectedIndex,
  onSelect,
  children,
}: PathCompletionPopupProps) {
  const containerRef = useRef<HTMLDivElement>(null);
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
  }, [open, onOpenChange]);

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

  return (
    <div ref={containerRef} className="relative flex-1 flex min-w-0">
      {children}
      {open && (
        <div
          ref={listRef}
          className="absolute bottom-full left-0 mb-2 w-[350px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden"
        >
          {completions.length === 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">
              No completions found
            </div>
          ) : (
            <div className="max-h-[200px] overflow-y-auto py-1" role="listbox">
              {completions.map((completion, index) => (
                <div
                  key={completion.insert_text}
                  role="option"
                  aria-selected={index === selectedIndex}
                  tabIndex={0}
                  data-index={index}
                  onClick={() => onSelect(completion)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(completion);
                    }
                  }}
                  className={cn(
                    "flex items-center gap-2 px-3 py-1.5",
                    "cursor-pointer transition-colors",
                    index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
                  )}
                >
                  {getIcon(completion.entry_type)}
                  <span className="font-mono text-sm text-foreground truncate">
                    {completion.name}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
