/**
 * PaneLeaf - Individual pane content renderer.
 * Displays either UnifiedTimeline+UnifiedInput (timeline mode) or Terminal (fullterm mode).
 * Handles focus management and visual indicators.
 *
 * Terminal rendering is handled via React portals (see TerminalLayer) to prevent
 * unmount/remount when pane structure changes during splits.
 *
 * HomeView and SettingsTabContent are lazy-loaded to improve initial bundle size
 * and load performance. These tab types are less frequently used than the default
 * terminal view, so deferring their load is beneficial.
 *
 * Performance: Uses usePaneLeafState selector to subscribe only to relevant state,
 * preventing re-renders when unrelated session or layout properties change.
 */

import React, { lazy, Suspense, useCallback } from "react";
import { ToolApprovalDialog } from "@/components/AgentChat";
import { UnifiedInput } from "@/components/UnifiedInput";
import { UnifiedTimeline } from "@/components/UnifiedTimeline";
import { ContextMenuTrigger } from "@/components/ui/context-menu";
import { useTerminalPortalTarget } from "@/hooks/useTerminalPortal";
import { countLeafPanes } from "@/lib/pane-utils";
import type { PaneId } from "@/store";
import { useStore } from "@/store";
import { usePaneLeafState } from "@/store/selectors/pane-leaf";
import { PaneContextMenu } from "./PaneContextMenu";
import { PaneMoveOverlay } from "./PaneMoveOverlay";

// Lazy-load tab-specific components to reduce initial bundle size
// HomeView (~50KB) and SettingsTabContent (~80KB) are only needed when
// the user opens those specific tab types
const HomeView = lazy(() => import("@/components/HomeView").then((m) => ({ default: m.HomeView })));
const SettingsTabContent = lazy(() =>
  import("@/components/Settings/SettingsTabContent").then((m) => ({
    default: m.SettingsTabContent,
  }))
);

// Loading fallback component for lazy-loaded tab content
function TabLoadingFallback() {
  return (
    <div className="h-full w-full flex items-center justify-center">
      <div className="animate-pulse text-muted-foreground">Loading...</div>
    </div>
  );
}

interface PaneLeafProps {
  paneId: PaneId;
  sessionId: string;
  tabId: string;
}

export const PaneLeaf = React.memo(function PaneLeaf({ paneId, sessionId, tabId }: PaneLeafProps) {
  // Use combined selector for efficient state access - only re-renders when
  // specific properties change, not when entire Session/TabLayout objects change
  const { focusedPaneId, renderMode, tabType, sessionExists, sessionName } = usePaneLeafState(
    tabId,
    sessionId
  );

  // Action is stable (doesn't change between renders)
  const focusPane = useStore((state) => state.focusPane);

  // Get pane count - subscribe to a primitive number instead of the full tree object
  const paneCount = useStore((state) => countLeafPanes(state.tabLayouts[tabId]?.root));

  // Register portal target for this pane's Terminal
  // The actual Terminal is rendered via TerminalLayer using React portals
  const terminalPortalRef = useTerminalPortalTarget(sessionId);

  const isFocused = focusedPaneId === paneId;
  const showFocusIndicator = isFocused && paneCount > 1;

  const handleFocus = useCallback(() => {
    if (!isFocused) {
      focusPane(tabId, paneId);
    }
  }, [tabId, paneId, isFocused, focusPane]);

  // Don't render if session doesn't exist
  if (!sessionExists) {
    return (
      <div className="h-full w-full flex items-center justify-center text-muted-foreground">
        Session not found
      </div>
    );
  }

  // Route content based on tab type
  // HomeView and SettingsTabContent are lazy-loaded with Suspense boundaries
  const renderTabContent = () => {
    switch (tabType) {
      case "home":
        return (
          <Suspense fallback={<TabLoadingFallback />}>
            <HomeView />
          </Suspense>
        );
      case "settings":
        return (
          <Suspense fallback={<TabLoadingFallback />}>
            <SettingsTabContent />
          </Suspense>
        );
      default:
        return (
          <>
            {/* Terminal portal target - the actual Terminal is rendered via TerminalLayer
                using React portals to prevent unmount/remount when pane structure changes.
                This div serves as the portal destination where the Terminal will appear.
                onMouseDownCapture ensures focus switches even though xterm.js captures clicks. */}
            <div
              ref={terminalPortalRef}
              className={renderMode === "fullterm" ? "flex-1 min-h-0 p-1" : "hidden"}
              onMouseDownCapture={handleFocus}
            />
            {renderMode !== "fullterm" && (
              // Timeline mode with unified input
              <>
                <div className="flex-1 min-h-0 min-w-0 flex flex-col overflow-hidden">
                  <UnifiedTimeline sessionId={sessionId} />
                </div>
                <UnifiedInput sessionId={sessionId} />
                <ToolApprovalDialog sessionId={sessionId} />
              </>
            )}
          </>
        );
    }
  };

  // Only show context menu for terminal tabs
  const isTerminal = tabType === "terminal" || tabType === undefined;

  const sectionContent = (
    <section
      className="h-full w-full flex flex-col relative overflow-hidden"
      tabIndex={-1}
      onClick={handleFocus}
      onKeyDown={handleFocus}
      onFocus={handleFocus}
      aria-label={`Pane: ${sessionName || "Terminal"}`}
      data-pane-drop-zone={sessionId}
    >
      {/* Focus indicator overlay - only show when multiple panes exist */}
      {showFocusIndicator && (
        <div
          className="absolute inset-0 pointer-events-none z-50 border border-accent"
          aria-hidden="true"
        />
      )}
      {/* Move overlay - shown when pane move mode is active */}
      {isTerminal && <PaneMoveOverlay paneId={paneId} />}
      {renderTabContent()}
    </section>
  );

  if (isTerminal) {
    return (
      <PaneContextMenu paneId={paneId} sessionId={sessionId} tabId={tabId}>
        <ContextMenuTrigger asChild>{sectionContent}</ContextMenuTrigger>
      </PaneContextMenu>
    );
  }

  return sectionContent;
});
