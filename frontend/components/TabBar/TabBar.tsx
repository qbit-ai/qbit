import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Bot,
  FileCode,
  History,
  Home,
  Loader2,
  Plus,
  Settings,
  Terminal,
  Wrench,
  X,
} from "lucide-react";
import React from "react";
import { useMockDevTools } from "@/components/MockDevTools";
import { NotificationWidget } from "@/components/NotificationWidget";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { shutdownAiSession } from "@/lib/ai";
import { logger } from "@/lib/logger";
import { ptyDestroy } from "@/lib/tauri";
import { liveTerminalManager, TerminalInstanceManager } from "@/lib/terminal";
import { cn } from "@/lib/utils";
import { isMockBrowserMode } from "@/mocks";
import { type Session, useStore } from "@/store";

const startDrag = async (e: React.MouseEvent) => {
  e.preventDefault();
  try {
    await getCurrentWindow().startDragging();
  } catch (err) {
    logger.error("Failed to start dragging:", err);
  }
};

interface TabBarProps {
  onNewTab: () => void;
  onOpenSettings?: () => void;
  onToggleFileEditorPanel?: () => void;
  onOpenHistory?: () => void;
  showTabNumbers?: boolean;
}

/**
 * Toggle button for Mock Dev Tools - only rendered in browser mode
 */
function MockDevToolsToggle() {
  const { toggle, isOpen } = useMockDevTools();

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          onClick={toggle}
          onMouseDown={(e) => e.stopPropagation()}
          title="Toggle Mock Dev Tools"
          className={cn(
            "h-6 w-6",
            "text-[var(--ansi-yellow)] hover:text-[var(--ansi-yellow)] hover:bg-[var(--ansi-yellow)]/10",
            isOpen && "bg-[var(--ansi-yellow)]/20"
          )}
        >
          <Wrench className="w-4 h-4" />
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        <p>Toggle Mock Dev Tools</p>
      </TooltipContent>
    </Tooltip>
  );
}

export function TabBar({
  onNewTab,
  onOpenSettings,
  onToggleFileEditorPanel,
  onOpenHistory,
  showTabNumbers,
}: TabBarProps) {
  const sessions = useStore((state) => state.sessions);
  const tabLayouts = useStore((state) => state.tabLayouts);
  const activeSessionId = useStore((state) => state.activeSessionId);
  const setActiveSession = useStore((state) => state.setActiveSession);
  const getTabSessionIds = useStore((state) => state.getTabSessionIds);
  const closeTab = useStore((state) => state.closeTab);

  const pendingCommand = useStore((state) => state.pendingCommand);
  const isAgentResponding = useStore((state) => state.isAgentResponding);
  const tabHasNewActivity = useStore((state) => state.tabHasNewActivity);

  // Only show sessions that are tab roots (have an entry in tabLayouts)
  // Pane sessions are contained within a tab's layout and should not appear as separate tabs
  const sessionList = Object.values(sessions)
    .filter((session) => session.id in tabLayouts)
    .sort((a, b) => {
      // Home tab always first
      if (a.tabType === "home") return -1;
      if (b.tabType === "home") return 1;
      return 0;
    });

  const handleCloseTab = React.useCallback(
    async (e: React.MouseEvent, tabId: string) => {
      e.stopPropagation();

      // Get the root session to check tab type
      const rootSession = sessions[tabId];
      const tabType = rootSession?.tabType ?? "terminal";

      // Only perform PTY/AI cleanup for terminal tabs
      if (tabType === "terminal") {
        try {
          // Get all session IDs for this tab (root + all pane sessions)
          const sessionIds = getTabSessionIds(tabId);

          // If no panes found, fall back to just the tabId (backward compatibility)
          const idsToCleanup = sessionIds.length > 0 ? sessionIds : [tabId];

          // Shutdown AI and PTY for ALL sessions in this tab (in parallel)
          await Promise.all(
            idsToCleanup.map(async (sessionId) => {
              try {
                await shutdownAiSession(sessionId);
              } catch (err) {
                logger.error(`Failed to shutdown AI session ${sessionId}:`, err);
              }
              try {
                await ptyDestroy(sessionId);
              } catch (err) {
                logger.error(`Failed to destroy PTY ${sessionId}:`, err);
              }
              // Cleanup terminal instances
              TerminalInstanceManager.dispose(sessionId);
              liveTerminalManager.dispose(sessionId);
            })
          );
        } catch (err) {
          logger.error(`Error closing tab ${tabId}:`, err);
        }
      }

      // Remove all frontend state for the tab
      closeTab(tabId);
    },
    [sessions, getTabSessionIds, closeTab]
  );

  return (
    <TooltipProvider delayDuration={300}>
      {/* biome-ignore lint/a11y/noStaticElementInteractions: div is used for window drag region */}
      <div
        className="relative z-[200] flex items-center h-[38px] bg-card border-b border-[var(--border-subtle)] pl-[78px] pr-1 gap-1"
        onMouseDown={startDrag}
      >
        <Tabs
          value={activeSessionId || undefined}
          onValueChange={setActiveSession}
          className="min-w-0"
          onMouseDown={(e) => e.stopPropagation()}
        >
          <TabsList className="h-7 bg-transparent p-0 gap-1 w-full justify-start">
            {sessionList.map((session, index) => {
              const tabId = session.id;
              const isActive = session.id === activeSessionId;
              const isTerminalTab = (session.tabType ?? "terminal") === "terminal";

              const sessionIds = isTerminalTab ? getTabSessionIds(tabId) : [];
              const idsToCheck = sessionIds.length > 0 ? sessionIds : [tabId];

              const isBusy =
                isTerminalTab &&
                idsToCheck.some(
                  (id) => pendingCommand[id] != null || isAgentResponding[id] === true
                );

              const hasNewActivity =
                isTerminalTab && !isActive && tabHasNewActivity[tabId] === true;

              return (
                <TabItem
                  key={session.id}
                  session={session}
                  isActive={isActive}
                  isBusy={isBusy}
                  onClose={(e) => handleCloseTab(e, session.id)}
                  canClose={session.tabType !== "home"}
                  tabNumber={index < 9 ? index + 1 : undefined}
                  showTabNumber={showTabNumbers}
                  hasNewActivity={hasNewActivity}
                />
              );
            })}
          </TabsList>
        </Tabs>

        {/* New tab button */}
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              aria-label="New tab"
              title="New tab"
              onClick={onNewTab}
              onMouseDown={(e) => e.stopPropagation()}
              className="h-6 w-6 text-muted-foreground hover:text-foreground hover:bg-[var(--bg-hover)]"
            >
              <Plus className="w-4 h-4" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            <p>New tab (⌘T)</p>
          </TooltipContent>
        </Tooltip>

        {/* Drag region - empty space extends to fill remaining width */}
        <div className="flex-1 h-full min-w-[100px]" />

        {/* Mock Dev Tools toggle - only in browser mode */}
        {isMockBrowserMode() && <MockDevToolsToggle />}

        {/* File Editor panel toggle */}
        {onToggleFileEditorPanel && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={onToggleFileEditorPanel}
                onMouseDown={(e) => e.stopPropagation()}
                className="h-6 w-6 text-muted-foreground hover:text-foreground hover:bg-[var(--bg-hover)]"
              >
                <FileCode className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>File Editor (⇧⌘E)</p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* History button */}
        {onOpenHistory && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={onOpenHistory}
                onMouseDown={(e) => e.stopPropagation()}
                className="h-6 w-6 text-muted-foreground hover:text-foreground hover:bg-[var(--bg-hover)]"
              >
                <History className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>Session History</p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* Settings button */}
        {onOpenSettings && (
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                onClick={onOpenSettings}
                onMouseDown={(e) => e.stopPropagation()}
                className="h-6 w-6 text-muted-foreground hover:text-foreground hover:bg-[var(--bg-hover)]"
              >
                <Settings className="w-4 h-4" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <p>Settings (⌘,)</p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* Separator */}
        <div className="h-4 w-px bg-border mx-1" />

        {/* Notification widget */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: div is used to prevent drag propagation to notification widget */}
        <div className="relative" onMouseDown={(e) => e.stopPropagation()}>
          <NotificationWidget />
        </div>
      </div>
    </TooltipProvider>
  );
}

interface TabItemProps {
  session: Session;
  isActive: boolean;
  isBusy: boolean;
  onClose: (e: React.MouseEvent) => void;
  canClose: boolean;
  tabNumber?: number;
  showTabNumber?: boolean;
  hasNewActivity: boolean;
}

const TabItem = React.memo(function TabItem({
  session,
  isActive,
  isBusy,
  onClose,
  canClose,
  tabNumber,
  showTabNumber,
  hasNewActivity,
}: TabItemProps) {
  const setCustomTabName = useStore((state) => state.setCustomTabName);
  const [isEditing, setIsEditing] = React.useState(false);
  const [editValue, setEditValue] = React.useState("");
  const inputRef = React.useRef<HTMLInputElement>(null);

  const tabType = session.tabType ?? "terminal";

  // Determine display name:
  // - home: no text label (icon only)
  // - settings: use session.name (or custom name)
  // - terminal: custom name > process name > directory name
  const { displayName, dirName, isCustomName, isProcessName } = React.useMemo(() => {
    if (tabType === "home") {
      return {
        displayName: "", // No text for home tab - icon only
        dirName: "",
        isCustomName: false,
        isProcessName: false,
      };
    }

    if (tabType === "settings") {
      const name = session.customName || session.name || "Settings";
      return {
        displayName: name,
        dirName: session.name || "Settings",
        isCustomName: !!session.customName,
        isProcessName: false,
      };
    }

    const dir = session.workingDirectory.split(/[/\\]/).pop() || "Terminal";
    const name = session.customName || session.processName || dir;
    return {
      displayName: name,
      dirName: dir,
      isCustomName: !!session.customName,
      isProcessName: !session.customName && !!session.processName,
    };
  }, [session.customName, session.name, session.processName, session.workingDirectory, tabType]);

  // Focus input when entering edit mode
  React.useEffect(() => {
    if (isEditing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [isEditing]);

  const handleDoubleClick = React.useCallback(
    (e: React.MouseEvent) => {
      if (tabType !== "terminal") return;
      e.preventDefault();
      e.stopPropagation();
      setIsEditing(true);
      setEditValue(session.customName || dirName);
    },
    [session.customName, dirName, tabType]
  );

  const handleSave = React.useCallback(() => {
    const trimmed = editValue.trim();
    setCustomTabName(session.id, trimmed || null);
    setIsEditing(false);
  }, [editValue, session.id, setCustomTabName]);

  const handleKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSave();
      } else if (e.key === "Escape") {
        e.preventDefault();
        setIsEditing(false);
      }
    },
    [handleSave]
  );

  const getTabIcon = () => {
    switch (tabType) {
      case "home":
        return Home;
      case "settings":
        return Settings;
      default:
        // For terminal tabs, icon depends on session mode
        return session.mode === "agent" ? Bot : Terminal;
    }
  };
  const ModeIcon = getTabIcon();

  // Generate tooltip text showing full context
  const tooltipText = React.useMemo(() => {
    if (tabType === "home") return "Home";
    if (tabType === "settings") return displayName;
    if (isCustomName) return `Custom name: ${displayName}\nDirectory: ${session.workingDirectory}`;
    if (isProcessName) return `Running: ${displayName}\nDirectory: ${session.workingDirectory}`;
    return session.workingDirectory;
  }, [isCustomName, isProcessName, displayName, session.workingDirectory, tabType]);

  return (
    <div className="group relative flex items-center">
      <Tooltip>
        <TooltipTrigger asChild>
          <TabsTrigger
            value={session.id}
            className={cn(
              "relative flex items-center gap-2 px-3 py-1.5 rounded-t-md min-w-0 max-w-[200px] text-[11px]",
              tabType === "terminal" && "font-mono",
              "data-[state=active]:bg-muted data-[state=active]:text-foreground data-[state=active]:shadow-none",
              "data-[state=inactive]:text-muted-foreground data-[state=inactive]:hover:bg-[var(--bg-hover)] data-[state=inactive]:hover:text-foreground",
              "border-none focus-visible:ring-0 focus-visible:ring-offset-0 transition-colors",
              canClose && "pr-7" // Add padding for close button
            )}
          >
            {/* Active indicator underline */}
            {isActive && <span className="absolute bottom-0 left-0 right-0 h-px bg-accent" />}

            {/* Busy spinner - only shown when tab is busy */}
            {isBusy && (
              <Loader2
                className={cn(
                  "w-3.5 h-3.5 flex-shrink-0 animate-spin",
                  isActive ? "text-accent" : "text-muted-foreground"
                )}
              />
            )}

            {/* New activity indicator dot - shown when inactive tab has new activity */}
            {hasNewActivity && !isBusy && (
              <span
                className="activity-dot w-1.5 h-1.5 flex-shrink-0 rounded-full bg-[var(--ansi-yellow)]"
                aria-label="New activity"
              />
            )}

            {/* Icon for non-terminal tabs (home, settings) - these don't have text labels */}
            {tabType !== "terminal" && !isBusy && (
              <ModeIcon
                className={cn(
                  "w-3.5 h-3.5 flex-shrink-0",
                  isActive ? "text-accent" : "text-muted-foreground"
                )}
              />
            )}

            {/* Tab name or edit input - not rendered for home tab (icon only) */}
            {tabType !== "home" &&
              (isEditing ? (
                <input
                  ref={inputRef}
                  type="text"
                  value={editValue}
                  onChange={(e) => setEditValue(e.target.value)}
                  onBlur={handleSave}
                  onKeyDown={handleKeyDown}
                  onClick={(e) => e.stopPropagation()}
                  className={cn(
                    "truncate text-[11px] bg-transparent border-none outline-none",
                    tabType === "terminal" && "font-mono",
                    "focus:ring-1 focus:ring-accent rounded px-1 min-w-[60px] max-w-[140px]"
                  )}
                />
              ) : (
                /* biome-ignore lint/a11y/noStaticElementInteractions: span is used for inline text with double-click rename */
                <span
                  className={cn(
                    "truncate",
                    tabType === "terminal" && "cursor-text",
                    isProcessName && !hasNewActivity && "text-accent",
                    hasNewActivity && "text-[var(--ansi-yellow)]"
                  )}
                  onDoubleClick={handleDoubleClick}
                >
                  {displayName}
                </span>
              ))}

            {/* Tab number badge - shown when Cmd is held */}
            {showTabNumber && tabNumber !== undefined && (
              <span className="flex-shrink-0 ml-1 px-1 min-w-[14px] h-[14px] flex items-center justify-center bg-accent text-accent-foreground text-[9px] font-semibold rounded">
                {tabNumber}
              </span>
            )}
          </TabsTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom" className="whitespace-pre-wrap">
          <p className="text-xs">{tooltipText}</p>
        </TooltipContent>
      </Tooltip>

      {/* Close button - positioned outside Tooltip to avoid event interference */}
      {canClose && (
        <button
          type="button"
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onClose(e);
          }}
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
          }}
          className={cn(
            "absolute right-1 p-0.5 rounded opacity-0 group-hover:opacity-100 transition-opacity",
            "hover:bg-destructive/20 text-muted-foreground hover:text-destructive",
            "z-10"
          )}
          title="Close tab"
        >
          <X className="w-3 h-3" />
        </button>
      )}
    </div>
  );
});
