/**
 * Appearance slice for the Zustand store.
 *
 * Manages display settings — per-element visibility toggles that let users
 * customise which UI chrome is shown (tab bar buttons, context bar, status bar, etc.).
 */

import type { SliceCreator } from "./types";

const DISPLAY_SETTINGS_KEY = "qbit-display-settings-v3";

/**
 * Controls which UI elements are visible.
 * `true` = shown, `false` = hidden.
 */
export interface DisplaySettings {
  /** Show the right-side tab bar buttons (parent gate). */
  showTabBar: boolean;
  /** Show the home tab in the tab bar. */
  showHomeTab: boolean;
  /** Show the file editor button in the tab bar. */
  showFileEditorButton: boolean;
  /** Show the session history button in the tab bar. */
  showHistoryButton: boolean;
  /** Show the settings button in the tab bar. */
  showSettingsButton: boolean;
  /** Show the notification bell in the tab bar. */
  showNotificationBell: boolean;
  /** Show the terminal context bar (path + git branch + env). */
  showTerminalContext: boolean;
  /** Show the working directory path badge in the context bar. */
  showWorkingDirectory: boolean;
  /** Show the git branch badge in the context bar. */
  showGitBranch: boolean;
  /** Show the entire bottom status bar row. */
  showStatusBar: boolean;
  /** Show the full input mode toggle (Terminal / AI) instead of collapsing to a single icon. */
  showInputModeToggle: boolean;
  /** Show the connection status and model name badge. */
  showStatusBadge: boolean;
  /** Show the agent mode selector dropdown. */
  showAgentModeSelector: boolean;
  /** Show the context / token usage percentage badge. */
  showContextUsage: boolean;
  /** Show the MCP servers indicator badge. */
  showMcpBadge: boolean;
  /** Hide AI-specific status bar items (model badge, token usage, agent mode, MCP) when in shell mode. */
  hideAiSettingsInShellMode: boolean;
}

export const defaultDisplaySettings: DisplaySettings = {
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
  hideAiSettingsInShellMode: false,
};

function loadDisplaySettings(): DisplaySettings {
  try {
    const stored = localStorage.getItem(DISPLAY_SETTINGS_KEY);
    if (stored) {
      return { ...defaultDisplaySettings, ...JSON.parse(stored) };
    }
  } catch {
    // ignore parse errors
  }
  return defaultDisplaySettings;
}

function saveDisplaySettings(settings: DisplaySettings): void {
  try {
    localStorage.setItem(DISPLAY_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
}

// State interface
export interface AppearanceState {
  displaySettings: DisplaySettings;
}

// Actions interface
export interface AppearanceActions {
  setDisplaySettings: (settings: DisplaySettings) => void;
}

// Combined slice interface
export interface AppearanceSlice extends AppearanceState, AppearanceActions {}

// Initial state
export const initialAppearanceState: AppearanceState = {
  displaySettings: loadDisplaySettings(),
};

/**
 * Creates the appearance slice.
 * Display settings are global (not per-session).
 */
export const createAppearanceSlice: SliceCreator<AppearanceSlice> = (set) => ({
  ...initialAppearanceState,

  setDisplaySettings: (settings) =>
    set((state) => {
      state.displaySettings = settings;
      saveDisplaySettings(settings);
    }),
});

// Selectors
export const selectDisplaySettings = <T extends AppearanceState>(
  state: T,
): DisplaySettings => state.displaySettings;
