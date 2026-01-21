import { ArrowLeft, ArrowRight, Copy, File, Folder, X } from "lucide-react";
import { useCallback, useRef } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { useCopyToClipboard } from "@/hooks/useCopyToClipboard";
import { cn } from "@/lib/utils";
import { getTabDisplayName, isTabDirty, type Tab } from "@/store/file-editor-sidebar";

interface TabBarProps {
  tabs: Tab[];
  activeTabId: string | null;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string) => void;
  onCloseOtherTabs: (tabId: string) => void;
  onCloseAllTabs: () => void;
  onReorderTabs?: (fromIndex: number, toIndex: number) => void;
}

function getParentDir(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 2) return "";
  return parts[parts.length - 2] ?? "";
}

export function TabBar({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onCloseOtherTabs,
  onCloseAllTabs,
  onReorderTabs,
}: TabBarProps) {
  const { copied, copy } = useCopyToClipboard();
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  const handleWheel = useCallback((e: React.WheelEvent) => {
    if (scrollContainerRef.current) {
      e.preventDefault();
      scrollContainerRef.current.scrollLeft += e.deltaY;
    }
  }, []);

  const handleMiddleClick = useCallback(
    (e: React.MouseEvent, tabId: string) => {
      if (e.button === 1) {
        e.preventDefault();
        onCloseTab(tabId);
      }
    },
    [onCloseTab]
  );

  if (tabs.length === 0) {
    return null;
  }

  // Check for duplicate display names to show parent directory for file tabs
  const displayNameCounts = new Map<string, number>();
  for (const tab of tabs) {
    const name = getTabDisplayName(tab);
    displayNameCounts.set(name, (displayNameCounts.get(name) ?? 0) + 1);
  }

  return (
    <div
      ref={scrollContainerRef}
      className="flex items-center border-b border-border bg-muted/30 overflow-x-auto scrollbar-none"
      onWheel={handleWheel}
    >
      {tabs.map((tab, index) => {
        const isActive = tab.id === activeTabId;
        const displayName = getTabDisplayName(tab);
        const isDirty = isTabDirty(tab);
        const showParent = tab.type === "file" && (displayNameCounts.get(displayName) ?? 0) > 1;
        const parentDir = showParent ? getParentDir(tab.file.path) : "";
        const canMoveLeft = index > 0;
        const canMoveRight = index < tabs.length - 1;

        return (
          <ContextMenu key={tab.id}>
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
                onClick={() => onSelectTab(tab.id)}
                onMouseDown={(e) => handleMiddleClick(e, tab.id)}
                title={tab.type === "file" ? tab.file.path : "File Browser"}
              >
                {/* Tab type icon */}
                {tab.type === "browser" ? (
                  <Folder className="w-3.5 h-3.5 text-blue-500 shrink-0" />
                ) : (
                  <File className="w-3.5 h-3.5 shrink-0" />
                )}

                {/* Dirty indicator */}
                {isDirty && (
                  <span
                    className="w-2 h-2 rounded-full bg-amber-500 shrink-0"
                    title="Unsaved changes"
                  />
                )}

                {/* Tab name with optional parent */}
                <span className="truncate">
                  {showParent && parentDir && (
                    <span className="text-muted-foreground/60">{parentDir}/</span>
                  )}
                  {displayName}
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
                    onCloseTab(tab.id);
                  }}
                  title="Close"
                >
                  <X className="w-3 h-3" />
                </button>
              </button>
            </ContextMenuTrigger>

            <ContextMenuContent>
              {tab.type === "file" && (
                <ContextMenuItem
                  onClick={async () => {
                    await copy(tab.file.path);
                  }}
                >
                  <Copy className="mr-2 h-3.5 w-3.5" />
                  {copied ? "Copied!" : "Copy path"}
                </ContextMenuItem>
              )}
              {tab.type === "file" && <ContextMenuSeparator />}

              {/* Move tab options */}
              {onReorderTabs && (
                <>
                  <ContextMenuItem
                    onClick={() => onReorderTabs(index, index - 1)}
                    disabled={!canMoveLeft}
                  >
                    <ArrowLeft className="mr-2 h-3.5 w-3.5" />
                    Move Left
                  </ContextMenuItem>
                  <ContextMenuItem
                    onClick={() => onReorderTabs(index, index + 1)}
                    disabled={!canMoveRight}
                  >
                    <ArrowRight className="mr-2 h-3.5 w-3.5" />
                    Move Right
                  </ContextMenuItem>
                  <ContextMenuSeparator />
                </>
              )}

              <ContextMenuItem onClick={() => onCloseTab(tab.id)}>Close</ContextMenuItem>
              <ContextMenuItem onClick={() => onCloseOtherTabs(tab.id)} disabled={tabs.length <= 1}>
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
