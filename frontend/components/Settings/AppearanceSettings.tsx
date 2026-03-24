import { ChevronRight, Paintbrush } from "lucide-react";
import { Switch } from "@/components/ui/switch";
import { useStore } from "@/store";
import {
  defaultDisplaySettings,
  type DisplaySettings,
  selectDisplaySettings,
} from "@/store/slices";
import { cn } from "@/lib/utils";

interface ToggleRowProps {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  /** Indent the row to show nesting */
  nested?: boolean;
  /** Gray out when a parent toggle makes these irrelevant */
  dimmed?: boolean;
}

function ToggleRow({ id, label, description, checked, onCheckedChange, nested, dimmed }: ToggleRowProps) {
  return (
    <div className={cn("flex items-center justify-between", nested && "pl-4", dimmed && "opacity-40 pointer-events-none")}>
      <div className={cn("space-y-1", nested && "flex items-start gap-1.5")}>
        {nested && <ChevronRight className="w-3 h-3 mt-0.5 flex-shrink-0 text-muted-foreground/50" />}
        <div>
          <label htmlFor={id} className="text-sm font-medium text-foreground cursor-pointer">
            {label}
          </label>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
      </div>
      <Switch id={id} checked={checked} onCheckedChange={onCheckedChange} />
    </div>
  );
}

export function AppearanceSettings() {
  const displaySettings = useStore(selectDisplaySettings);
  const setDisplaySettings = useStore((state) => state.setDisplaySettings);

  const update = (patch: Partial<DisplaySettings>) => {
    setDisplaySettings({ ...displaySettings, ...patch });
  };

  const allShown = Object.values(displaySettings).every(Boolean);
  const allHidden = Object.values(displaySettings).every((v) => !v);

  const tabBarSubOptions: Array<keyof DisplaySettings> = [
    "showHomeTab",
    "showFileEditorButton",
    "showHistoryButton",
    "showSettingsButton",
    "showNotificationBell",
  ];

  const contextBarSubOptions: Array<keyof DisplaySettings> = [
    "showWorkingDirectory",
    "showGitBranch",
  ];

  const statusBarSubOptions: Array<keyof DisplaySettings> = [
    "showInputModeToggle",
    "showStatusBadge",
    "showAgentModeSelector",
    "showContextUsage",
    "showMcpBadge",
  ];

  const tabBarParentOn = displaySettings.showTabBar || tabBarSubOptions.some((k) => displaySettings[k]);
  const contextBarParentOn = displaySettings.showTerminalContext || contextBarSubOptions.some((k) => displaySettings[k]);
  const statusBarParentOn = displaySettings.showStatusBar || statusBarSubOptions.some((k) => displaySettings[k]);

  return (
    <div className="space-y-6">
      {/* Appearance header */}
      <div className="space-y-4">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Paintbrush className="w-4 h-4 text-accent" />
            <h3 className="text-sm font-semibold text-foreground">Appearance</h3>
          </div>
          <p className="text-xs text-muted-foreground">
            Choose which UI elements are visible. Toggle off to hide an element from the interface.
          </p>
        </div>

        <div className="rounded-lg border border-[var(--border-medium)] bg-card/50 divide-y divide-[var(--border-subtle)]">
          {/* Tab Bar section */}
          <div className="px-4 py-3 space-y-3">
            <ToggleRow
              id="show-tab-bar-buttons"
              label="Tab Bar"
              description="Show the right-side tab bar icons"
              checked={tabBarParentOn}
              onCheckedChange={(checked) => {
                if (checked) {
                  update({ showTabBar: true });
                } else {
                  update({
                    showTabBar: false,
                    ...Object.fromEntries(tabBarSubOptions.map((k) => [k, false])),
                  });
                }
              }}
            />
            <div className={cn("space-y-3 pl-2", !tabBarParentOn && "opacity-40 pointer-events-none")}>
              <ToggleRow
                id="show-home-tab"
                label="Home Tab"
                description="Show the home tab in the tab bar"
                checked={displaySettings.showHomeTab}
                onCheckedChange={(checked) => update({ showHomeTab: checked })}
                nested
              />
              <ToggleRow
                id="show-file-editor-button"
                label="File Editor"
                description="Show the file editor panel button"
                checked={displaySettings.showFileEditorButton}
                onCheckedChange={(checked) => update({ showFileEditorButton: checked })}
                nested
              />
              <ToggleRow
                id="show-history-button"
                label="Session History"
                description="Show the session history button"
                checked={displaySettings.showHistoryButton}
                onCheckedChange={(checked) => update({ showHistoryButton: checked })}
                nested
              />
              <ToggleRow
                id="show-settings-button"
                label="Settings"
                description="Show the settings button"
                checked={displaySettings.showSettingsButton}
                onCheckedChange={(checked) => update({ showSettingsButton: checked })}
                nested
              />
              <ToggleRow
                id="show-notification-bell"
                label="Notification Bell"
                description="Show the notification bell"
                checked={displaySettings.showNotificationBell}
                onCheckedChange={(checked) => update({ showNotificationBell: checked })}
                nested
              />
            </div>
          </div>

          {/* Terminal Context section */}
          <div className="px-4 py-3 space-y-3">
            <ToggleRow
              id="show-terminal-context"
              label="Terminal Context"
              description="Show the path and git info bar"
              checked={contextBarParentOn}
              onCheckedChange={(checked) => {
                if (checked) {
                  update({ showTerminalContext: true });
                } else {
                  update({
                    showTerminalContext: false,
                    ...Object.fromEntries(contextBarSubOptions.map((k) => [k, false])),
                  });
                }
              }}
            />
            <div className={cn("space-y-3 pl-2", !contextBarParentOn && "opacity-40 pointer-events-none")}>
              <ToggleRow
                id="show-working-directory"
                label="Working Directory"
                description="Show the current working directory path badge"
                checked={displaySettings.showWorkingDirectory}
                onCheckedChange={(checked) => update({ showWorkingDirectory: checked })}
                nested
              />
              <ToggleRow
                id="show-git-branch"
                label="Git Branch"
                description="Show the git branch and diff stats badge"
                checked={displaySettings.showGitBranch}
                onCheckedChange={(checked) => update({ showGitBranch: checked })}
                nested
              />
            </div>
          </div>

          {/* Status Bar section */}
          <div className="px-4 py-3 space-y-3">
            <ToggleRow
              id="show-status-bar"
              label="Status Bar"
              description="Show the entire bottom status bar"
              checked={statusBarParentOn}
              onCheckedChange={(checked) => {
                if (checked) {
                  update({ showStatusBar: true });
                } else {
                  update({
                    showStatusBar: false,
                    ...Object.fromEntries(statusBarSubOptions.map((k) => [k, false])),
                  });
                }
              }}
            />
            <div className={cn("space-y-3 pl-2", !statusBarParentOn && "opacity-40 pointer-events-none")}>
              <ToggleRow
                id="show-input-mode-toggle"
                label="Input Mode Toggle"
                description="Show the full Terminal / AI segmented toggle instead of collapsing it"
                checked={displaySettings.showInputModeToggle}
                onCheckedChange={(checked) => update({ showInputModeToggle: checked })}
                nested
              />
              <ToggleRow
                id="show-status-badge"
                label="Status & Model Badge"
                description="Show the connection status and active model name badge"
                checked={displaySettings.showStatusBadge}
                onCheckedChange={(checked) => update({ showStatusBadge: checked })}
                nested
              />
              <ToggleRow
                id="show-agent-mode-selector"
                label="Agent Mode Selector"
                description="Show the agent mode (Auto / Plan / etc.) dropdown"
                checked={displaySettings.showAgentModeSelector}
                onCheckedChange={(checked) => update({ showAgentModeSelector: checked })}
                nested
              />
              <ToggleRow
                id="show-context-usage"
                label="Token Usage"
                description="Show the context window / token usage percentage badge"
                checked={displaySettings.showContextUsage}
                onCheckedChange={(checked) => update({ showContextUsage: checked })}
                nested
              />
              <ToggleRow
                id="show-mcp-badge"
                label="MCP Servers Badge"
                description="Show the MCP servers connected indicator"
                checked={displaySettings.showMcpBadge}
                onCheckedChange={(checked) => update({ showMcpBadge: checked })}
                nested
              />
            </div>
          </div>
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            disabled={allShown}
            onClick={() =>
              setDisplaySettings({
                showTabBar: true,
                showHomeTab: true,
                showFileEditorButton: true,
                showHistoryButton: true,
                showSettingsButton: true,
                showNotificationBell: true,
                showTerminalContext: true,
                showWorkingDirectory: true,
                showGitBranch: true,
                showStatusBar: true,
                showInputModeToggle: true,
                showStatusBadge: true,
                showAgentModeSelector: true,
                showContextUsage: true,
                showMcpBadge: true,
              })
            }
            className="text-xs text-accent hover:underline disabled:opacity-40 disabled:no-underline disabled:cursor-not-allowed"
          >
            Show all
          </button>
          <span className="text-xs text-muted-foreground/50">·</span>
          <button
            type="button"
            disabled={allHidden}
            onClick={() =>
              setDisplaySettings({
                showTabBar: false,
                showHomeTab: false,
                showFileEditorButton: false,
                showHistoryButton: false,
                showSettingsButton: false,
                showNotificationBell: false,
                showTerminalContext: false,
                showWorkingDirectory: false,
                showGitBranch: false,
                showStatusBar: false,
                showInputModeToggle: false,
                showStatusBadge: false,
                showAgentModeSelector: false,
                showContextUsage: false,
                showMcpBadge: false,
              })
            }
            className="text-xs text-accent hover:underline disabled:opacity-40 disabled:no-underline disabled:cursor-not-allowed"
          >
            Hide all
          </button>
          <span className="text-xs text-muted-foreground/50">·</span>
          <button
            type="button"
            onClick={() => setDisplaySettings({ ...defaultDisplaySettings })}
            className="text-xs text-accent hover:underline"
          >
            Reset to defaults
          </button>
        </div>
      </div>
    </div>
  );
}
