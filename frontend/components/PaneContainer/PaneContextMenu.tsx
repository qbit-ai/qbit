import { Columns2, ExternalLink, MousePointerClick, Rows2 } from "lucide-react";
import { useCallback } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuShortcut,
} from "@/components/ui/context-menu";
import { usePaneControls } from "@/hooks/usePaneControls";
import { countLeafPanes } from "@/lib/pane-utils";
import type { PaneId } from "@/store";
import { useStore } from "@/store";

interface PaneContextMenuProps {
  children: React.ReactNode;
  paneId: PaneId;
  sessionId: string;
  tabId: string;
}

export function PaneContextMenu({ children, paneId, sessionId, tabId }: PaneContextMenuProps) {
  const startPaneMove = useStore((state) => state.startPaneMove);
  const movePaneToNewTab = useStore((state) => state.movePaneToNewTab);
  const paneCount = useStore((state) => {
    const layout = state.tabLayouts[tabId];
    return layout ? countLeafPanes(layout.root) : 1;
  });

  const { handleSplitPane } = usePaneControls(tabId);

  const hasMultiplePanes = paneCount > 1;
  const canSplit = paneCount < 4;

  const handleMovePane = useCallback(() => {
    startPaneMove(tabId, paneId, sessionId);
  }, [tabId, paneId, sessionId, startPaneMove]);

  const handleConvertToTab = useCallback(() => {
    movePaneToNewTab(tabId, paneId);
  }, [tabId, paneId, movePaneToNewTab]);

  return (
    <ContextMenu>
      {children}
      <ContextMenuContent>
        <ContextMenuItem onClick={() => handleSplitPane("vertical")} disabled={!canSplit}>
          <Columns2 className="w-3.5 h-3.5" />
          Split Vertically
          <ContextMenuShortcut>⌘D</ContextMenuShortcut>
        </ContextMenuItem>
        <ContextMenuItem onClick={() => handleSplitPane("horizontal")} disabled={!canSplit}>
          <Rows2 className="w-3.5 h-3.5" />
          Split Horizontally
          <ContextMenuShortcut>⇧⌘D</ContextMenuShortcut>
        </ContextMenuItem>

        {hasMultiplePanes && (
          <>
            <ContextMenuSeparator />
            <ContextMenuItem onClick={handleMovePane}>
              <MousePointerClick className="w-3.5 h-3.5" />
              Move Pane...
            </ContextMenuItem>
            <ContextMenuItem onClick={handleConvertToTab}>
              <ExternalLink className="w-3.5 h-3.5" />
              Convert to Tab
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
    </ContextMenu>
  );
}
