import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_CARET_SETTINGS,
  type CaretSettings,
  type TerminalSettings as TerminalSettingsType,
} from "@/lib/settings";
import { useStore } from "@/store";
import { defaultDisplaySettings } from "@/store/slices";

// Mock ThemePicker since it depends on Tauri/theme hooks
vi.mock("./ThemePicker", () => ({
  ThemePicker: () => <div data-testid="theme-picker">ThemePicker</div>,
}));

// Mock react-colorful
vi.mock("react-colorful", () => ({
  HexColorPicker: ({ color, onChange }: { color: string; onChange: (c: string) => void }) => (
    <input data-testid="hex-color-picker" data-color={color} onChange={() => onChange("#00ff00")} />
  ),
}));

// Import after mocks
import { AppearanceSettings } from "./AppearanceSettings";

const baseTerminalSettings: TerminalSettingsType = {
  shell: null,
  font_family: "JetBrains Mono",
  font_size: 14,
  scrollback: 10000,
  fullterm_commands: [],
  caret: { ...DEFAULT_CARET_SETTINGS },
};

const blockCaretTerminalSettings: TerminalSettingsType = {
  ...baseTerminalSettings,
  caret: {
    style: "block",
    width: 1.0,
    color: null,
    blink_speed: 530,
    opacity: 1.0,
  },
};

describe("AppearanceSettings", () => {
  beforeEach(() => {
    // Reset display settings to defaults before each test
    useStore.getState().setDisplaySettings({ ...defaultDisplaySettings });
  });

  describe("theme section", () => {
    it("should render the Theme heading", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Theme")).toBeInTheDocument();
    });

    it("should render the ThemePicker component", () => {
      render(<AppearanceSettings />);
      expect(screen.getByTestId("theme-picker")).toBeInTheDocument();
    });
  });

  describe("caret section", () => {
    it("should render the Input Caret heading", () => {
      render(
        <AppearanceSettings
          terminalSettings={baseTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("Input Caret")).toBeInTheDocument();
    });

    it("should render the Preview via CaretPreview", () => {
      render(
        <AppearanceSettings
          terminalSettings={baseTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    it("should render the Style selector", () => {
      render(
        <AppearanceSettings
          terminalSettings={baseTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("Style")).toBeInTheDocument();
    });

    it("should not show block-specific controls when style is default", () => {
      render(
        <AppearanceSettings
          terminalSettings={baseTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.queryByText("Width")).not.toBeInTheDocument();
      expect(screen.queryByText("Blink Speed")).not.toBeInTheDocument();
      expect(screen.queryByText("Opacity")).not.toBeInTheDocument();
    });

    it("should show block-specific controls when style is block", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("Width")).toBeInTheDocument();
      expect(screen.getByText("Color")).toBeInTheDocument();
      expect(screen.getByText("Blink Speed")).toBeInTheDocument();
      expect(screen.getByText("Opacity")).toBeInTheDocument();
    });

    it("should display width value in ch units", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("1.0ch")).toBeInTheDocument();
    });

    it("should display blink speed in ms", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("530ms")).toBeInTheDocument();
    });

    it("should display 'No blink' when blink_speed is 0", () => {
      const settings = {
        ...blockCaretTerminalSettings,
        caret: { ...blockCaretTerminalSettings.caret, blink_speed: 0 },
      };
      render(
        <AppearanceSettings terminalSettings={settings} onTerminalChange={vi.fn()} />
      );
      expect(screen.getByText("No blink")).toBeInTheDocument();
    });

    it("should display opacity as percentage", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByText("100%")).toBeInTheDocument();
    });

    it("should show 'Reset to theme default' button when color is set", () => {
      const settings: TerminalSettingsType = {
        ...blockCaretTerminalSettings,
        caret: { ...blockCaretTerminalSettings.caret, color: "#ff0000" },
      };
      render(
        <AppearanceSettings terminalSettings={settings} onTerminalChange={vi.fn()} />
      );
      expect(screen.getByText("Reset to theme default")).toBeInTheDocument();
    });

    it("should not show 'Reset to theme default' button when color is null", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.queryByText("Reset to theme default")).not.toBeInTheDocument();
    });

    it("should call onTerminalChange to reset color when 'Reset to theme default' is clicked", async () => {
      const user = userEvent.setup();
      const onTerminalChange = vi.fn();
      const settings: TerminalSettingsType = {
        ...blockCaretTerminalSettings,
        caret: { ...blockCaretTerminalSettings.caret, color: "#ff0000" },
      };
      render(
        <AppearanceSettings terminalSettings={settings} onTerminalChange={onTerminalChange} />
      );
      await user.click(screen.getByText("Reset to theme default"));
      expect(onTerminalChange).toHaveBeenCalledWith(
        expect.objectContaining({
          caret: expect.objectContaining({ color: null }),
        })
      );
    });

    it("should render color picker button with aria label", () => {
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );
      expect(screen.getByLabelText("Pick caret color")).toBeInTheDocument();
    });

    it("should toggle color picker visibility on button click", async () => {
      const user = userEvent.setup();
      render(
        <AppearanceSettings
          terminalSettings={blockCaretTerminalSettings}
          onTerminalChange={vi.fn()}
        />
      );

      // Color picker should not be visible initially
      expect(screen.queryByTestId("hex-color-picker")).not.toBeInTheDocument();

      // Click to open
      await user.click(screen.getByLabelText("Pick caret color"));
      expect(screen.getByTestId("hex-color-picker")).toBeInTheDocument();

      // Click to close
      await user.click(screen.getByLabelText("Pick caret color"));
      expect(screen.queryByTestId("hex-color-picker")).not.toBeInTheDocument();
    });

    it("should fall back to DEFAULT_CARET_SETTINGS when terminalSettings is not provided", () => {
      render(<AppearanceSettings />);
      // Should still render without crashing, using defaults
      expect(screen.getByText("Input Caret")).toBeInTheDocument();
      expect(screen.getByText("Style")).toBeInTheDocument();
    });

    it("should fall back to DEFAULT_CARET_SETTINGS when caret field is missing (backward compat)", () => {
      const settings = { ...baseTerminalSettings } as TerminalSettingsType;
      // @ts-expect-error Testing backward compat with missing caret
      delete settings.caret;
      render(
        <AppearanceSettings terminalSettings={settings} onTerminalChange={vi.fn()} />
      );
      // Should still render without crashing
      expect(screen.getByText("Input Caret")).toBeInTheDocument();
    });
  });

  describe("UI customization section", () => {
    it("should render the UI Customization heading", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("UI Customization")).toBeInTheDocument();
    });

    it("should render the General heading with Hide AI Settings toggle", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Hide AI Settings in Shell Mode")).toBeInTheDocument();
    });

    it("should render Tab Bar parent toggle", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Tab Bar")).toBeInTheDocument();
    });

    it("should render Tab Bar child toggles", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Home Tab")).toBeInTheDocument();
      expect(screen.getByText("File Editor")).toBeInTheDocument();
      expect(screen.getByText("Session History")).toBeInTheDocument();
      expect(screen.getByRole("switch", { name: "Settings" })).toBeInTheDocument();
      expect(screen.getByText("Notification Bell")).toBeInTheDocument();
    });

    it("should render Context Bar parent toggle and children", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Context Bar")).toBeInTheDocument();
      expect(screen.getByText("Working Directory")).toBeInTheDocument();
      expect(screen.getByText("Git Branch")).toBeInTheDocument();
    });

    it("should render Status Bar parent toggle and children", () => {
      render(<AppearanceSettings />);
      expect(screen.getByText("Status Bar")).toBeInTheDocument();
      expect(screen.getByText("Input Mode Toggle")).toBeInTheDocument();
      expect(screen.getByText("Status & Model Badge")).toBeInTheDocument();
      expect(screen.getByText("Agent Mode Selector")).toBeInTheDocument();
      expect(screen.getByText("Token Usage")).toBeInTheDocument();
      expect(screen.getByText("MCP Servers Badge")).toBeInTheDocument();
    });

    it("should toggle Hide AI Settings in Shell Mode", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      const toggle = screen.getByRole("switch", { name: "Hide AI Settings in Shell Mode" });
      expect(toggle).toHaveAttribute("aria-checked", "false");

      await user.click(toggle);
      expect(useStore.getState().displaySettings.hideAiSettingsInShellMode).toBe(true);
    });

    it("should turn off Tab Bar and all children when parent is toggled off", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      // Tab Bar should be on by default
      const tabBarToggle = screen.getByRole("switch", { name: "Tab Bar" });
      expect(tabBarToggle).toHaveAttribute("aria-checked", "true");

      // Toggle it off
      await user.click(tabBarToggle);

      const state = useStore.getState().displaySettings;
      expect(state.showTabBar).toBe(false);
      expect(state.showHomeTab).toBe(false);
      expect(state.showFileEditorButton).toBe(false);
      expect(state.showHistoryButton).toBe(false);
      expect(state.showSettingsButton).toBe(false);
      expect(state.showNotificationBell).toBe(false);
    });

    it("should turn off Context Bar and all children when parent is toggled off", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      const contextBarToggle = screen.getByRole("switch", { name: "Context Bar" });
      expect(contextBarToggle).toHaveAttribute("aria-checked", "true");

      await user.click(contextBarToggle);

      const state = useStore.getState().displaySettings;
      expect(state.showTerminalContext).toBe(false);
      expect(state.showWorkingDirectory).toBe(false);
      expect(state.showGitBranch).toBe(false);
    });

    it("should turn off Status Bar and all children when parent is toggled off", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      const statusBarToggle = screen.getByRole("switch", { name: "Status Bar" });
      expect(statusBarToggle).toHaveAttribute("aria-checked", "true");

      await user.click(statusBarToggle);

      const state = useStore.getState().displaySettings;
      expect(state.showStatusBar).toBe(false);
      expect(state.showInputModeToggle).toBe(false);
      expect(state.showStatusBadge).toBe(false);
      expect(state.showAgentModeSelector).toBe(false);
      expect(state.showContextUsage).toBe(false);
      expect(state.showMcpBadge).toBe(false);
    });

    it("should toggle individual child settings independently", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      const homeTabToggle = screen.getByRole("switch", { name: "Home Tab" });
      expect(homeTabToggle).toHaveAttribute("aria-checked", "true");

      await user.click(homeTabToggle);
      expect(useStore.getState().displaySettings.showHomeTab).toBe(false);
      // Other settings remain true
      expect(useStore.getState().displaySettings.showFileEditorButton).toBe(true);
    });

    it("should show 'Show all' button that enables all visibility settings", async () => {
      const user = userEvent.setup();

      // First hide everything
      useStore.getState().setDisplaySettings({
        ...defaultDisplaySettings,
        showTabBar: false,
        showHomeTab: false,
        showStatusBar: false,
      });

      render(<AppearanceSettings />);

      const showAllBtn = screen.getByText("Show all");
      await user.click(showAllBtn);

      const state = useStore.getState().displaySettings;
      expect(state.showTabBar).toBe(true);
      expect(state.showHomeTab).toBe(true);
      expect(state.showStatusBar).toBe(true);
      expect(state.showTerminalContext).toBe(true);
    });

    it("should show 'Hide all' button that disables all visibility settings", async () => {
      const user = userEvent.setup();
      render(<AppearanceSettings />);

      const hideAllBtn = screen.getByText("Hide all");
      await user.click(hideAllBtn);

      const state = useStore.getState().displaySettings;
      expect(state.showTabBar).toBe(false);
      expect(state.showHomeTab).toBe(false);
      expect(state.showFileEditorButton).toBe(false);
      expect(state.showTerminalContext).toBe(false);
      expect(state.showStatusBar).toBe(false);
      expect(state.showMcpBadge).toBe(false);
    });

    it("should show 'Reset to defaults' button that restores default settings", async () => {
      const user = userEvent.setup();

      // Change some settings
      useStore.getState().setDisplaySettings({
        ...defaultDisplaySettings,
        showTabBar: false,
        showStatusBar: false,
        hideAiSettingsInShellMode: true,
      });

      render(<AppearanceSettings />);

      const resetBtn = screen.getByText("Reset to defaults");
      await user.click(resetBtn);

      const state = useStore.getState().displaySettings;
      expect(state).toEqual(defaultDisplaySettings);
    });

    it("should disable 'Show all' when all settings are already shown", () => {
      // All are shown by default
      render(<AppearanceSettings />);
      const showAllBtn = screen.getByText("Show all");
      expect(showAllBtn).toBeDisabled();
    });

    it("should disable 'Hide all' when all visibility settings are already hidden", () => {
      useStore.getState().setDisplaySettings({
        ...defaultDisplaySettings,
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
      });

      render(<AppearanceSettings />);
      const hideAllBtn = screen.getByText("Hide all");
      expect(hideAllBtn).toBeDisabled();
    });

    it("should preserve hideAiSettingsInShellMode when using 'Show all'", async () => {
      const user = userEvent.setup();

      useStore.getState().setDisplaySettings({
        ...defaultDisplaySettings,
        showTabBar: false,
        hideAiSettingsInShellMode: true,
      });

      render(<AppearanceSettings />);
      await user.click(screen.getByText("Show all"));

      // hideAiSettingsInShellMode should be preserved (not overwritten)
      expect(useStore.getState().displaySettings.hideAiSettingsInShellMode).toBe(true);
    });

    it("should preserve hideAiSettingsInShellMode when using 'Hide all'", async () => {
      const user = userEvent.setup();

      useStore.getState().setDisplaySettings({
        ...defaultDisplaySettings,
        hideAiSettingsInShellMode: true,
      });

      render(<AppearanceSettings />);
      await user.click(screen.getByText("Hide all"));

      // hideAiSettingsInShellMode should be preserved
      expect(useStore.getState().displaySettings.hideAiSettingsInShellMode).toBe(true);
    });
  });
});
