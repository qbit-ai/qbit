/**
 * Focus mode slice for the Zustand store.
 *
 * Manages focus mode state — a UI toggle that hides non-essential chrome
 * (notification bell, MCP badge, agent mode selector, inactive mode icon)
 * for a clean, minimal deep-work experience.
 */

import type { SliceCreator } from "./types";

const FOCUS_DISPLAY_SETTINGS_KEY = "qbit-focus-display-settings-v2";

/**
 * Controls which UI elements remain visible when focus mode is active.
 * `true` = shown in focus mode, `false` = hidden in focus mode.
 */
export interface FocusModeDisplaySettings {
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
}

export const defaultFocusModeDisplaySettings: FocusModeDisplaySettings = {
  showHomeTab: false,
  showFileEditorButton: false,
  showHistoryButton: false,
  showSettingsButton: false,
  showNotificationBell: false,
  showStatusBar: true,
  showInputModeToggle: false,
  showStatusBadge: false,
  showAgentModeSelector: false,
  showContextUsage: false,
  showMcpBadge: false,
};

function loadFocusModeDisplaySettings(): FocusModeDisplaySettings {
  try {
    const stored = localStorage.getItem(FOCUS_DISPLAY_SETTINGS_KEY);
    if (stored) {
      return { ...defaultFocusModeDisplaySettings, ...JSON.parse(stored) };
    }
  } catch {
    // ignore parse errors
  }
  return defaultFocusModeDisplaySettings;
}

function saveFocusModeDisplaySettings(settings: FocusModeDisplaySettings): void {
  try {
    localStorage.setItem(FOCUS_DISPLAY_SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // ignore storage errors
  }
}

// State interface
export interface FocusState {
  focusModeEnabled: boolean;
  focusModeDisplaySettings: FocusModeDisplaySettings;
}

// Actions interface
export interface FocusActions {
  toggleFocusMode: () => void;
  setFocusMode: (enabled: boolean) => void;
  setFocusModeDisplaySettings: (settings: FocusModeDisplaySettings) => void;
}

// Combined slice interface
export interface FocusSlice extends FocusState, FocusActions {}

// Initial state
export const initialFocusState: FocusState = {
  focusModeEnabled: false,
  focusModeDisplaySettings: loadFocusModeDisplaySettings(),
};

/**
 * Creates the focus mode slice.
 * Focus mode is a global (not per-session) toggle.
 */
export const createFocusSlice: SliceCreator<FocusSlice> = (set) => ({
  ...initialFocusState,

  toggleFocusMode: () =>
    set((state) => {
      state.focusModeEnabled = !state.focusModeEnabled;
    }),

  setFocusMode: (enabled) =>
    set((state) => {
      state.focusModeEnabled = enabled;
    }),

  setFocusModeDisplaySettings: (settings) =>
    set((state) => {
      state.focusModeDisplaySettings = settings;
      saveFocusModeDisplaySettings(settings);
    }),
});

// Selectors
export const selectFocusModeEnabled = <T extends FocusState>(state: T): boolean =>
  state.focusModeEnabled;

export const selectFocusModeDisplaySettings = <T extends FocusState>(
  state: T
): FocusModeDisplaySettings => state.focusModeDisplaySettings;
