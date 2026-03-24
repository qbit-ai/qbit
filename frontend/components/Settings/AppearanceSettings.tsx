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
  /** Gray out when a parent toggle makes this irrelevant */
  dimmed?: boolean;
}

function ToggleRow({ id, label, description, checked, onCheckedChange, dimmed }: ToggleRowProps) {
  return (
    <div className={cn("flex items-center justify-between", dimmed && "opacity-40 pointer-events-none")}>
      <div className="space-y-1">
        <label htmlFor={id} className="text-sm font-medium text-foreground cursor-pointer">
          {label}
        </label>
        <p className="text-xs text-muted-foreground">{description}</p>
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

  const { hideAiSettingsInShellMode: _behavior, ...visibilitySettings } = displaySettings;
  const allShown = Object.values(visibilitySettings).every(Boolean);
  const allHidden = Object.values(visibilitySettings).every((v) => !v);

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
    <div className="space-y-8">
      {/* Section header */}
      <div className="space-y-1">
        <h2 className="text-base font-semibold text-foreground">UI Customization</h2>
        <p className="text-sm text-muted-foreground">Fine-grained customization of UI elements and components</p>
      </div>

      {/* General */}
      <div className="space-y-4">
        <h3 className="text-sm font-medium text-foreground">General</h3>
        <ToggleRow
          id="hide-ai-settings-in-shell-mode"
          label="Hide AI Settings in Shell Mode"
          description="Hide model badge, token usage, agent mode, and MCP badge when in shell mode"
          checked={displaySettings.hideAiSettingsInShellMode}
          onCheckedChange={(checked) => update({ hideAiSettingsInShellMode: checked })}
        />
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--border-medium)]" />

      {/* Tab Bar */}
      <div className="space-y-4">
        <ToggleRow
          id="show-tab-bar-buttons"
          label="Tab Bar"
          description="Show application icons in the top tab bar"
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
        <div className={cn("space-y-4 pl-4 border-l-2 border-[var(--border-subtle)]", !tabBarParentOn && "opacity-40 pointer-events-none")}>
          <ToggleRow
            id="show-home-tab"
            label="Home Tab"
            description="Show the home tab in the tab bar"
            checked={displaySettings.showHomeTab}
            onCheckedChange={(checked) => update({ showHomeTab: checked })}
          />
          <ToggleRow
            id="show-file-editor-button"
            label="File Editor"
            description="Show the file editor panel button"
            checked={displaySettings.showFileEditorButton}
            onCheckedChange={(checked) => update({ showFileEditorButton: checked })}
          />
          <ToggleRow
            id="show-history-button"
            label="Session History"
            description="Show the session history button"
            checked={displaySettings.showHistoryButton}
            onCheckedChange={(checked) => update({ showHistoryButton: checked })}
          />
          <ToggleRow
            id="show-settings-button"
            label="Settings"
            description="Show the settings button"
            checked={displaySettings.showSettingsButton}
            onCheckedChange={(checked) => update({ showSettingsButton: checked })}
          />
          <ToggleRow
            id="show-notification-bell"
            label="Notification Bell"
            description="Show the notification bell"
            checked={displaySettings.showNotificationBell}
            onCheckedChange={(checked) => update({ showNotificationBell: checked })}
          />
        </div>
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--border-medium)]" />

      {/* Terminal Context */}
      <div className="space-y-4">
        <ToggleRow
          id="show-terminal-context"
          label="Context Bar"
          description="Show context information above the terminal input"
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
        <div className={cn("space-y-4 pl-4 border-l-2 border-[var(--border-subtle)]", !contextBarParentOn && "opacity-40 pointer-events-none")}>
          <ToggleRow
            id="show-working-directory"
            label="Working Directory"
            description="Show the current working directory path badge"
            checked={displaySettings.showWorkingDirectory}
            onCheckedChange={(checked) => update({ showWorkingDirectory: checked })}
          />
          <ToggleRow
            id="show-git-branch"
            label="Git Branch"
            description="Show the git branch and diff stats badge"
            checked={displaySettings.showGitBranch}
            onCheckedChange={(checked) => update({ showGitBranch: checked })}
          />
        </div>
      </div>

      {/* Divider */}
      <div className="border-t border-[var(--border-medium)]" />

      {/* Status Bar */}
      <div className="space-y-4">
        <ToggleRow
          id="show-status-bar"
          label="Status Bar"
          description="Show the bottom status bar"
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
        <div className={cn("space-y-4 pl-4 border-l-2 border-[var(--border-subtle)]", !statusBarParentOn && "opacity-40 pointer-events-none")}>
          <ToggleRow
            id="show-input-mode-toggle"
            label="Input Mode Toggle"
            description="Show the full Terminal / AI segmented toggle instead of collapsing it"
            checked={displaySettings.showInputModeToggle}
            onCheckedChange={(checked) => update({ showInputModeToggle: checked })}
          />
          <ToggleRow
            id="show-status-badge"
            label="Status & Model Badge"
            description="Show the connection status and active model name badge"
            checked={displaySettings.showStatusBadge}
            onCheckedChange={(checked) => update({ showStatusBadge: checked })}
          />
          <ToggleRow
            id="show-agent-mode-selector"
            label="Agent Mode Selector"
            description="Show the agent mode (Auto / Plan / etc.) dropdown"
            checked={displaySettings.showAgentModeSelector}
            onCheckedChange={(checked) => update({ showAgentModeSelector: checked })}
          />
          <ToggleRow
            id="show-context-usage"
            label="Token Usage"
            description="Show the context window / token usage percentage badge"
            checked={displaySettings.showContextUsage}
            onCheckedChange={(checked) => update({ showContextUsage: checked })}
          />
          <ToggleRow
            id="show-mcp-badge"
            label="MCP Servers Badge"
            description="Show the MCP servers connected indicator"
            checked={displaySettings.showMcpBadge}
            onCheckedChange={(checked) => update({ showMcpBadge: checked })}
          />
        </div>
      </div>
      {/* Quick actions */}
      <div className="flex items-center gap-3">
        <p className="text-xs text-muted-foreground">
          Choose which UI elements are visible.
        </p>
        <span className="text-xs text-muted-foreground/50">·</span>
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
              hideAiSettingsInShellMode: displaySettings.hideAiSettingsInShellMode,
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
              hideAiSettingsInShellMode: displaySettings.hideAiSettingsInShellMode,
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
  );
}
