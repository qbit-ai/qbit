import { useEffect, useRef } from "react";
import { type EntryType, getEntryIcon } from "@/lib/file-icons";
import type { PathCompletion } from "@/lib/tauri";
import { cn } from "@/lib/utils";

interface PathCompletionPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  completions: PathCompletion[];
  totalCount: number;
  selectedIndex: number;
  onSelect: (completion: PathCompletion) => void;
  children: React.ReactNode;
}

/** Renders a name with matched characters highlighted */
function HighlightedName({ name, indices }: { name: string; indices: number[] }) {
  if (indices.length === 0) {
    return <span>{name}</span>;
  }

  const indexSet = new Set(indices);
  const chars = [...name];

  return (
    <span>
      {chars.map((char, i) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: Characters are static, never reordered
        <span key={i} className={indexSet.has(i) ? "text-primary font-semibold" : ""}>
          {char}
        </span>
      ))}
    </span>
  );
}

export function PathCompletionPopup({
  open,
  onOpenChange,
  completions,
  totalCount,
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
          data-testid="path-completion-popup"
          className="absolute bottom-full left-0 mb-2 min-w-[300px] max-w-[500px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden"
        >
          {/* Result count badge - shown when there are more matches than displayed */}
          {totalCount > completions.length && (
            <div className="px-3 py-1 text-xs text-muted-foreground border-b border-border bg-muted/30">
              Showing {completions.length} of {totalCount} matches
            </div>
          )}

          {completions.length === 0 ? (
            <div className="py-3 text-center text-[13px] text-muted-foreground">
              No completions found
            </div>
          ) : (
            <div
              ref={listRef}
              className="max-h-[530px] overflow-y-scroll py-1"
              style={{ scrollbarGutter: "stable" }}
              role="listbox"
            >
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
                  {getEntryIcon(completion.entry_type as EntryType, completion.name)}
                  <span className="font-mono text-[13px] text-foreground truncate">
                    <HighlightedName name={completion.name} indices={completion.match_indices} />
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
