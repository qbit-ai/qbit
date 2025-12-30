/**
 * PaneLeaf - Individual pane content renderer.
 * Displays either UnifiedTimeline+UnifiedInput (timeline mode) or Terminal (fullterm mode).
 * Handles focus management and visual indicators.
 */

import { useCallback } from "react";
import { ToolApprovalDialog } from "@/components/AgentChat";
import { Terminal } from "@/components/Terminal";
import { UnifiedInput } from "@/components/UnifiedInput";
import { UnifiedTimeline } from "@/components/UnifiedTimeline";
import { countLeafPanes } from "@/lib/pane-utils";
import type { PaneId } from "@/store";
import { useStore } from "@/store";

interface PaneLeafProps {
  paneId: PaneId;
  sessionId: string;
  tabId: string;
}

export function PaneLeaf({ paneId, sessionId, tabId }: PaneLeafProps) {
  const focusPane = useStore((state) => state.focusPane);
  const tabLayout = useStore((state) => state.tabLayouts[tabId]);
  const focusedPaneId = tabLayout?.focusedPaneId;
  const session = useStore((state) => state.sessions[sessionId]);

  const isFocused = focusedPaneId === paneId;
  const paneCount = tabLayout?.root ? countLeafPanes(tabLayout.root) : 1;
  const showFocusIndicator = isFocused && paneCount > 1;
  const renderMode = session?.renderMode ?? "timeline";
  const workingDirectory = session?.workingDirectory;

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

  return (
    <section
      className="h-full w-full flex flex-col relative overflow-hidden"
      tabIndex={-1}
      onClick={handleFocus}
      onKeyDown={handleFocus}
      onFocus={handleFocus}
      aria-label={`Pane: ${session.name || "Terminal"}`}
    >
      {/* Focus indicator overlay - only show when multiple panes exist */}
      {showFocusIndicator && (
        <div
          className="absolute inset-0 pointer-events-none z-50 border border-accent"
          aria-hidden="true"
        />
      )}
      {renderMode === "fullterm" ? (
        // Full terminal mode
        <div className="flex-1 min-h-0 p-1">
          <Terminal sessionId={sessionId} />
        </div>
      ) : (
        // Timeline mode with unified input
        <>
          <div className="flex-1 min-h-0 min-w-0 overflow-auto">
            <UnifiedTimeline sessionId={sessionId} />
          </div>
          <UnifiedInput sessionId={sessionId} workingDirectory={workingDirectory} />
          <ToolApprovalDialog sessionId={sessionId} />
        </>
      )}
    </section>
  );
}
