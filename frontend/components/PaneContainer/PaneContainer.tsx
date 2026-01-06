/**
 * PaneContainer - Recursive component that renders the pane tree layout.
 * Uses react-resizable-panels for split views.
 */

import { useCallback } from "react";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import type { PaneNode } from "@/lib/pane-utils";
import { useStore } from "@/store";
import { PaneLeaf } from "./PaneLeaf";

interface PaneContainerProps {
  node: PaneNode;
  tabId: string;
  onOpenGitPanel?: () => void;
}

export function PaneContainer({ node, tabId, onOpenGitPanel }: PaneContainerProps) {
  const resizePane = useStore((state) => state.resizePane);

  const handleLayout = useCallback(
    (sizes: number[]) => {
      if (node.type === "split" && sizes.length === 2) {
        // Convert percentage to ratio (0-1)
        const ratio = sizes[0] / 100;
        resizePane(tabId, node.id, ratio);
      }
    },
    [node, tabId, resizePane]
  );

  // Leaf node - render the actual pane content
  if (node.type === "leaf") {
    return (
      <PaneLeaf
        paneId={node.id}
        sessionId={node.sessionId}
        tabId={tabId}
        onOpenGitPanel={onOpenGitPanel}
      />
    );
  }

  // Split node - render nested resizable panels
  // Note: "horizontal" split (panes stacked above/below) uses "vertical" direction for the panel group
  // "vertical" split (panes side by side) uses "horizontal" direction for the panel group
  const panelDirection = node.direction === "horizontal" ? "vertical" : "horizontal";

  return (
    <ResizablePanelGroup direction={panelDirection} onLayout={handleLayout} className="h-full">
      <ResizablePanel defaultSize={node.ratio * 100} minSize={10}>
        <PaneContainer node={node.children[0]} tabId={tabId} onOpenGitPanel={onOpenGitPanel} />
      </ResizablePanel>
      <ResizableHandle className="bg-border/50 hover:bg-border transition-colors" />
      <ResizablePanel defaultSize={(1 - node.ratio) * 100} minSize={10}>
        <PaneContainer node={node.children[1]} tabId={tabId} onOpenGitPanel={onOpenGitPanel} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
