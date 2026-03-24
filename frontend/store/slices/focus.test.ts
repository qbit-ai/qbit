import { beforeEach, describe, expect, it } from "vitest";
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import {
  createAppearanceSlice,
  type AppearanceSlice,
  defaultDisplaySettings,
  selectDisplaySettings,
} from "./appearance";

describe("Appearance Slice", () => {
  const createTestStore = () =>
    create<AppearanceSlice>()(immer((set, get) => createAppearanceSlice(set, get)));

  let store: ReturnType<typeof createTestStore>;

  beforeEach(() => {
    store = createTestStore();
  });

  describe("initial state", () => {
    it("should have all display settings defaulting to true", () => {
      const settings = store.getState().displaySettings;
      for (const value of Object.values(settings)) {
        expect(value).toBe(true);
      }
    });
  });

  describe("setDisplaySettings", () => {
    it("should update display settings", () => {
      store.getState().setDisplaySettings({ ...defaultDisplaySettings, showTabBar: false });
      expect(store.getState().displaySettings.showTabBar).toBe(false);
    });

    it("should replace all settings", () => {
      const allHidden: typeof defaultDisplaySettings = {
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
      };
      store.getState().setDisplaySettings(allHidden);
      for (const value of Object.values(store.getState().displaySettings)) {
        expect(value).toBe(false);
      }
    });
  });

  describe("selectors", () => {
    it("selectDisplaySettings should return current display settings", () => {
      const settings = selectDisplaySettings(store.getState());
      expect(settings).toEqual(defaultDisplaySettings);
    });
  });
});
