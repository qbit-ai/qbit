import { useCallback, useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { HistoryMatch } from "@/hooks/useHistorySearch";
import { cn } from "@/lib/utils";

// Threshold for enabling virtualization (above this count, use virtual scrolling)
const HISTORY_VIRTUALIZATION_THRESHOLD = 100;

// Static style constants for virtualized items
const virtualItemBaseStyle = {
  position: "absolute",
  top: 0,
  left: 0,
  width: "100%",
} as const;

interface VirtualizedHistoryListProps {
  matches: HistoryMatch[];
  selectedIndex: number;
  searchQuery: string;
  onSelect: (match: HistoryMatch) => void;
  highlightMatch: (command: string, query: string) => React.ReactNode;
  listRef: React.RefObject<HTMLDivElement>;
}

/**
 * Virtualized history list component for handling large search results.
 * Only renders visible items, improving performance significantly.
 */
function VirtualizedHistoryList({
  matches,
  selectedIndex,
  searchQuery,
  onSelect,
  highlightMatch,
  listRef,
}: VirtualizedHistoryListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: matches.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40, // Estimated height per match item
    overscan: 5,
  });

  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div
      ref={parentRef}
      className="max-h-[300px] overflow-y-auto relative"
      style={{ contain: "strict" }}
      role="listbox"
    >
      <div
        style={{
          height: virtualizer.getTotalSize(),
          width: "100%",
          position: "relative",
        }}
      >
        {virtualItems.map((virtualRow) => {
          const match = matches[virtualRow.index];
          const isSelected = virtualRow.index === selectedIndex;

          return (
            <div
              key={`${match.index}-${match.command}`}
              data-index={virtualRow.index}
              role="option"
              aria-selected={isSelected}
              tabIndex={0}
              style={{
                ...virtualItemBaseStyle,
                transform: `translateY(${virtualRow.start}px)`,
              }}
              onClick={() => onSelect(match)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onSelect(match);
                }
              }}
              className={cn(
                "flex items-center gap-2 px-3 py-1.5 h-10",
                "cursor-pointer transition-colors",
                isSelected ? "bg-primary/10" : "hover:bg-card"
              )}
            >
              <span className="font-mono text-sm text-foreground truncate">
                {highlightMatch(match.command, searchQuery)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface HistorySearchPopupProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  matches: HistoryMatch[];
  selectedIndex: number;
  searchQuery: string;
  onSelect: (match: HistoryMatch) => void;
  children: React.ReactNode;
}

export function HistorySearchPopup({
  open,
  onOpenChange,
  matches,
  selectedIndex,
  searchQuery,
  onSelect,
  children,
}: HistorySearchPopupProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  /**
   * Memoized function to highlight the search query within the command text.
   * Case-insensitive highlighting.
   */
  const highlightMatch = useCallback(
    (command: string, query: string): React.ReactNode => {
      if (!query) return command;

      const lowerCommand = command.toLowerCase();
      const lowerQuery = query.toLowerCase();
      const parts: React.ReactNode[] = [];
      let lastIndex = 0;

      let matchIndex = lowerCommand.indexOf(lowerQuery, lastIndex);
      while (matchIndex !== -1) {
        // Add text before match
        if (matchIndex > lastIndex) {
          parts.push(command.slice(lastIndex, matchIndex));
        }

        // Add highlighted match
        parts.push(
          <span key={matchIndex} className="bg-yellow-500/30 text-yellow-600 dark:text-yellow-400">
            {command.slice(matchIndex, matchIndex + query.length)}
          </span>
        );

        lastIndex = matchIndex + query.length;
        matchIndex = lowerCommand.indexOf(lowerQuery, lastIndex);
      }

      // Add remaining text
      if (lastIndex < command.length) {
        parts.push(command.slice(lastIndex));
      }

      return parts;
    },
    []
  );

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
        <div className="absolute bottom-full left-0 mb-2 w-full max-w-[600px] z-50 bg-popover border border-border rounded-md shadow-md overflow-hidden">
          {/* Search input header */}
          <div className="px-3 py-2 border-b border-border bg-muted/30">
            <div className="flex items-center gap-2">
              <span className="text-muted-foreground text-xs">Search:</span>
              <span className="font-mono text-sm text-foreground">
                {searchQuery || (
                  <span className="text-muted-foreground italic">type to filter...</span>
                )}
              </span>
            </div>
          </div>

          {/* Match list */}
          {matches.length === 0 ? (
            <div className="py-3 text-center text-sm text-muted-foreground">
              {searchQuery ? "No matches found" : "No history"}
            </div>
          ) : matches.length < HISTORY_VIRTUALIZATION_THRESHOLD ? (
            // Non-virtualized rendering for small lists
            <div ref={listRef} className="max-h-[300px] overflow-y-auto py-1" role="listbox">
              {matches.map((match, index) => (
                <div
                  key={`${match.index}-${match.command}`}
                  role="option"
                  aria-selected={index === selectedIndex}
                  tabIndex={0}
                  data-index={index}
                  onClick={() => onSelect(match)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      onSelect(match);
                    }
                  }}
                  className={cn(
                    "flex items-center gap-2 px-3 py-1.5",
                    "cursor-pointer transition-colors",
                    index === selectedIndex ? "bg-primary/10" : "hover:bg-card"
                  )}
                >
                  <span className="font-mono text-sm text-foreground truncate">
                    {highlightMatch(match.command, searchQuery)}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            // Virtualized rendering for large lists
            <VirtualizedHistoryList
              matches={matches}
              selectedIndex={selectedIndex}
              searchQuery={searchQuery}
              onSelect={onSelect}
              highlightMatch={highlightMatch}
              listRef={listRef}
            />
          )}
        </div>
      )}
    </div>
  );
}
