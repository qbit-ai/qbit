import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  DEFAULT_CARET_SETTINGS,
  type TerminalSettings as TerminalSettingsType,
} from "@/lib/settings";

// Mock ThemePicker since it depends on Tauri commands
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
import { TerminalSettings } from "./TerminalSettings";

const baseSettings: TerminalSettingsType = {
  shell: null,
  font_family: "JetBrains Mono",
  font_size: 14,
  scrollback: 10000,
  fullterm_commands: [],
  caret: { ...DEFAULT_CARET_SETTINGS },
};

const blockCaretSettings: TerminalSettingsType = {
  ...baseSettings,
  caret: {
    style: "block",
    width: 1.0,
    color: null,
    blink_speed: 530,
    opacity: 1.0,
  },
};

describe("TerminalSettings", () => {
  describe("caret section", () => {
    it("should render the Input Caret heading", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByText("Input Caret")).toBeInTheDocument();
    });

    it("should render the Preview label", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByText("Preview")).toBeInTheDocument();
    });

    it("should render the Style label", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByText("Style")).toBeInTheDocument();
    });

    it("should not show block-specific controls when style is default", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.queryByText("Width")).not.toBeInTheDocument();
      expect(screen.queryByText("Blink Speed")).not.toBeInTheDocument();
      expect(screen.queryByText("Opacity")).not.toBeInTheDocument();
    });

    it("should show block-specific controls when style is block", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.getByText("Width")).toBeInTheDocument();
      expect(screen.getByText("Color")).toBeInTheDocument();
      expect(screen.getByText("Blink Speed")).toBeInTheDocument();
      expect(screen.getByText("Opacity")).toBeInTheDocument();
    });

    it("should display width value in ch units", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.getByText("1.0ch")).toBeInTheDocument();
    });

    it("should display blink speed in ms", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.getByText("530ms")).toBeInTheDocument();
    });

    it("should display 'No blink' when blink_speed is 0", () => {
      const onChange = vi.fn();
      const settings = {
        ...blockCaretSettings,
        caret: { ...blockCaretSettings.caret, blink_speed: 0 },
      };
      render(<TerminalSettings settings={settings} onChange={onChange} />);
      expect(screen.getByText("No blink")).toBeInTheDocument();
    });

    it("should display opacity as percentage", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.getByText("100%")).toBeInTheDocument();
    });

    it("should show 'Reset to theme default' button when color is set", () => {
      const onChange = vi.fn();
      const settings = {
        ...blockCaretSettings,
        caret: { ...blockCaretSettings.caret, color: "#ff0000" },
      };
      render(<TerminalSettings settings={settings} onChange={onChange} />);
      expect(screen.getByText("Reset to theme default")).toBeInTheDocument();
    });

    it("should not show 'Reset to theme default' button when color is null", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.queryByText("Reset to theme default")).not.toBeInTheDocument();
    });

    it("should call onChange to reset color when 'Reset to theme default' is clicked", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      const settings = {
        ...blockCaretSettings,
        caret: { ...blockCaretSettings.caret, color: "#ff0000" },
      };
      render(<TerminalSettings settings={settings} onChange={onChange} />);
      await user.click(screen.getByText("Reset to theme default"));
      expect(onChange).toHaveBeenCalledWith(
        expect.objectContaining({
          caret: expect.objectContaining({ color: null }),
        })
      );
    });

    it("should render color picker button with aria label", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);
      expect(screen.getByLabelText("Pick caret color")).toBeInTheDocument();
    });

    it("should toggle color picker visibility on button click", async () => {
      const user = userEvent.setup();
      const onChange = vi.fn();
      render(<TerminalSettings settings={blockCaretSettings} onChange={onChange} />);

      // Color picker should not be visible initially
      expect(screen.queryByTestId("hex-color-picker")).not.toBeInTheDocument();

      // Click to open
      await user.click(screen.getByLabelText("Pick caret color"));
      expect(screen.getByTestId("hex-color-picker")).toBeInTheDocument();

      // Click to close
      await user.click(screen.getByLabelText("Pick caret color"));
      expect(screen.queryByTestId("hex-color-picker")).not.toBeInTheDocument();
    });

    it("should handle missing caret settings gracefully (backward compat)", () => {
      const onChange = vi.fn();
      // Simulate old config without caret field
      const settings = { ...baseSettings } as TerminalSettingsType;
      // @ts-expect-error Testing backward compat with missing caret
      delete settings.caret;
      render(<TerminalSettings settings={settings} onChange={onChange} />);
      // Should still render without crashing
      expect(screen.getByText("Input Caret")).toBeInTheDocument();
    });
  });

  describe("existing settings", () => {
    it("should render shell input", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByLabelText("Shell")).toBeInTheDocument();
    });

    it("should render font family input", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByLabelText("Font Family")).toBeInTheDocument();
    });

    it("should render font size input", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByLabelText("Font Size")).toBeInTheDocument();
    });

    it("should render scrollback input", () => {
      const onChange = vi.fn();
      render(<TerminalSettings settings={baseSettings} onChange={onChange} />);
      expect(screen.getByLabelText("Scrollback Lines")).toBeInTheDocument();
    });
  });
});
