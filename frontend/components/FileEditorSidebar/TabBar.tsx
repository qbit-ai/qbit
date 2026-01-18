import { X } from "lucide-react";
import { useCallback, useRef } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { cn } from "@/lib/utils";
import type { EditorFileState } from "@/store/file-editor-sidebar";

interface TabBarProps {
  tabs: Array<{ path: string; file: EditorFileState }>;
  activeFilePath: string | null;
  onSelectTab: (path: string) => void;
  onCloseTab: (path: string) => void;
  onCloseOtherTabs: (path: string) => void;
  onCloseAllTabs: () => void;
}

function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

function getParentDir(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  return parts[parts.length - 2] ?? "";
}

export function TabBar({
  tabs,
  activeFilePath,
  onSelectTab,
  onCloseTab,
  onCloseOtherTabs,
  onCloseAllTabs,
}: TabBarProps) {
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    if (scrollContainerRef.current) {
      e.preventDefault();
      scrollContainerRef.current.scrollLeft += e.deltaY;
    }
  }, []);

  const handleMiddleClick = useCallback(
    (e: React.MouseEvent, path: string) => {
      if (e.button === 1) {
        e.preventDefault();
        onCloseTab(path);
      }
    },
    [onCloseTab]
  );

  if (tabs.length === 0) {
    return null;
  }

  // Check for duplicate filenames to show parent directory
  const fileNameCounts = new Map<string, number>();
  for (const tab of tabs) {
    const name = getFileName(tab.path);
    fileNameCounts.set(name, (fileNameCounts.get(name) ?? 0) + 1);
  }

  return (
    <div
      ref={scrollContainerRef}
      className="flex items-center border-b border-border bg-muted/30 overflow-x-auto scrollbar-none"
      onWheel={handleWheel}
    >
      {tabs.map((tab) => {
        const isActive = tab.path === activeFilePath;
        const fileName = getFileName(tab.path);
        const showParent = (fileNameCounts.get(fileName) ?? 0) > 1;
        const parentDir = showParent ? getParentDir(tab.path) : "";

        return (
          <ContextMenu key={tab.path}>
            <ContextMenuTrigger asChild>
              <button
                type="button"
                className={cn(
                  "group flex items-center gap-1.5 px-3 py-1.5 text-xs border-r border-border",
                  "hover:bg-muted/50 transition-colors shrink-0 max-w-[200px]",
                  isActive
                    ? "bg-background text-foreground border-b-2 border-b-primary -mb-px"
                    : "text-muted-foreground"
                )}
                onClick={() => onSelectTab(tab.path)}
                onMouseDown={(e) => handleMiddleClick(e, tab.path)}
                title={tab.path}
              >
                {/* Dirty indicator */}
                {tab.file.dirty && (
                  <span
                    className="w-2 h-2 rounded-full bg-amber-500 shrink-0"
                    title="Unsaved changes"
                  />
                )}

                {/* Filename with optional parent */}
                <span className="truncate">
                  {showParent && parentDir && (
                    <span className="text-muted-foreground/60">{parentDir}/</span>
                  )}
                  {fileName}
                </span>

                {/* Close button */}
                <button
                  type="button"
                  className={cn(
                    "ml-1 p-0.5 rounded hover:bg-muted-foreground/20 shrink-0",
                    "opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity",
                    isActive && "opacity-60"
                  )}
                  onClick={(e) => {
                    e.stopPropagation();
                    onCloseTab(tab.path);
                  }}
                  title="Close"
                >
                  <X className="w-3 h-3" />
                </button>
              </button>
            </ContextMenuTrigger>

            <ContextMenuContent>
              <ContextMenuItem onClick={() => onCloseTab(tab.path)}>Close</ContextMenuItem>
              <ContextMenuItem
                onClick={() => onCloseOtherTabs(tab.path)}
                disabled={tabs.length <= 1}
              >
                Close Others
              </ContextMenuItem>
              <ContextMenuSeparator />
              <ContextMenuItem onClick={onCloseAllTabs}>Close All</ContextMenuItem>
            </ContextMenuContent>
          </ContextMenu>
        );
      })}
    </div>
  );
}
