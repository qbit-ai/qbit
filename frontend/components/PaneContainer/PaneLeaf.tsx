/**
 * PaneLeaf - Individual pane content renderer.
 * Displays either UnifiedTimeline+UnifiedInput (timeline mode) or Terminal (fullterm mode).
 * Handles focus management and visual indicators.
 *
 * Terminal rendering is handled via React portals (see TerminalLayer) to prevent
 * unmount/remount when pane structure changes during splits.
 */

import React, { useCallback } from "react";
import { ToolApprovalDialog } from "@/components/AgentChat";
import { HomeView } from "@/components/HomeView";
import { SettingsTabContent } from "@/components/Settings/SettingsTabContent";
import { UnifiedInput } from "@/components/UnifiedInput";
import { UnifiedTimeline } from "@/components/UnifiedTimeline";
import { useTerminalPortalTarget } from "@/hooks/useTerminalPortal";
import { countLeafPanes } from "@/lib/pane-utils";
import type { PaneId } from "@/store";
import { useStore } from "@/store";

interface PaneLeafProps {
  paneId: PaneId;
  sessionId: string;
  tabId: string;
  onOpenGitPanel?: () => void;
}

export const PaneLeaf = React.memo(function PaneLeaf({
  paneId,
  sessionId,
  tabId,
  onOpenGitPanel,
}: PaneLeafProps) {
  const focusPane = useStore((state) => state.focusPane);
  const tabLayout = useStore((state) => state.tabLayouts[tabId]);
  const focusedPaneId = tabLayout?.focusedPaneId;
  const session = useStore((state) => state.sessions[sessionId]);

  // Register portal target for this pane's Terminal
  // The actual Terminal is rendered via TerminalLayer using React portals
  const terminalPortalRef = useTerminalPortalTarget(sessionId);

  const isFocused = focusedPaneId === paneId;
  const paneCount = tabLayout?.root ? countLeafPanes(tabLayout.root) : 1;
  const showFocusIndicator = isFocused && paneCount > 1;
  const renderMode = session?.renderMode ?? "timeline";
  const workingDirectory = session?.workingDirectory;
  const tabType = session?.tabType ?? "terminal";

  const handleFocus = useCallback(() => {
    if (!isFocused) {
      focusPane(tabId, paneId);
    }
  }, [tabId, paneId, isFocused, focusPane]);

  // Don't render if session doesn't exist
  if (!session) {
    return (
      <div className="h-full w-full flex items-center justify-center text-muted-foreground">
        Session not found
      </div>
    );
  }

  // Route content based on tab type
  const renderTabContent = () => {
    switch (tabType) {
      case "home":
        return <HomeView />;
      case "settings":
        return <SettingsTabContent />;
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
                <UnifiedInput
                  sessionId={sessionId}
                  workingDirectory={workingDirectory}
                  onOpenGitPanel={onOpenGitPanel}
                />
                <ToolApprovalDialog sessionId={sessionId} />
              </>
            )}
          </>
        );
    }
  };

  return (
    <section
      className="h-full w-full flex flex-col relative overflow-hidden"
      tabIndex={-1}
      onClick={handleFocus}
      onKeyDown={handleFocus}
      onFocus={handleFocus}
      aria-label={`Pane: ${session.name || "Terminal"}`}
      data-pane-drop-zone={sessionId}
    >
      {/* Focus indicator overlay - only show when multiple panes exist */}
      {showFocusIndicator && (
        <div
          className="absolute inset-0 pointer-events-none z-50 border border-accent"
          aria-hidden="true"
        />
      )}
      {renderTabContent()}
    </section>
  );
});
