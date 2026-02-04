import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ITerminalOptions, Terminal as XTerm } from "@xterm/xterm";
import type { QbitTheme } from "./types";

// Mock dependencies
vi.mock("@/lib/logger", () => ({
  logger: {
    warn: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
}));

vi.mock("../themes", () => ({
  getThemeAssetPath: vi.fn().mockResolvedValue("/mock/path"),
}));

vi.mock("./builtin/obsidian-ember/assets/background.jpeg?url", () => ({
  default: "/mock/obsidian-ember-bg.jpeg",
}));

// Create a mock theme for testing
const createMockTheme = (name: string): QbitTheme => ({
  schemaVersion: "1.0",
  name,
  colors: {
    ui: {
      background: "#000000",
      foreground: "#ffffff",
      card: "#111111",
      cardForeground: "#ffffff",
      popover: "#111111",
      popoverForeground: "#ffffff",
      primary: "#0066ff",
      primaryForeground: "#ffffff",
      secondary: "#333333",
      secondaryForeground: "#ffffff",
      muted: "#222222",
      mutedForeground: "#888888",
      accent: "#0066ff",
      accentForeground: "#ffffff",
      destructive: "#ff0000",
      border: "#333333",
      input: "#222222",
      ring: "#0066ff",
      sidebar: "#111111",
      sidebarForeground: "#ffffff",
      sidebarPrimary: "#0066ff",
      sidebarPrimaryForeground: "#ffffff",
      sidebarAccent: "#222222",
      sidebarAccentForeground: "#ffffff",
      sidebarBorder: "#333333",
      sidebarRing: "#0066ff",
    },
    ansi: {
      black: "#000000",
      red: "#ff0000",
      green: "#00ff00",
      yellow: "#ffff00",
      blue: "#0000ff",
      magenta: "#ff00ff",
      cyan: "#00ffff",
      white: "#ffffff",
      brightBlack: "#666666",
      brightRed: "#ff6666",
      brightGreen: "#66ff66",
      brightYellow: "#ffff66",
      brightBlue: "#6666ff",
      brightMagenta: "#ff66ff",
      brightCyan: "#66ffff",
      brightWhite: "#ffffff",
      defaultFg: "#ffffff",
      defaultBg: "#000000",
    },
  },
  typography: {
    terminal: {
      fontFamily: "JetBrains Mono",
      fontSize: 14,
    },
  },
  terminal: {
    cursorBlink: true,
    cursorStyle: "block" as const,
  },
});

// Mock ThemeRegistry
const mockThemes = new Map<string, QbitTheme>();
vi.mock("./registry", () => ({
  ThemeRegistry: {
    get: vi.fn((id: string) => mockThemes.get(id)),
    has: vi.fn((id: string) => mockThemes.has(id)),
    getEntry: vi.fn((id: string) => {
      const theme = mockThemes.get(id);
      return theme ? { id, theme, builtin: false } : null;
    }),
    saveTheme: vi.fn(),
    initialize: vi.fn(),
    getAll: vi.fn(() => []),
    onChange: vi.fn(() => () => {}),
    unregister: vi.fn(),
  },
}));

// Import after mocks are set up
import { ThemeManager } from "./ThemeManager";

describe("ThemeManager Terminal Batching", () => {
  // Track property assignments to the terminal
  let propertySetCount: number;
  let mockTerminal: XTerm;
  let optionsProxy: ITerminalOptions;

  beforeEach(() => {
    propertySetCount = 0;
    mockThemes.clear();
    mockThemes.set("test-theme", createMockTheme("Test Theme"));

    // Create a proxy to track individual property sets
    const optionsTarget: Partial<ITerminalOptions> = {};
    optionsProxy = new Proxy(optionsTarget as ITerminalOptions, {
      set(_target, _prop, _value) {
        propertySetCount++;
        return true;
      },
      get(_target, prop) {
        return Reflect.get(_target, prop);
      },
    });

    mockTerminal = {
      options: optionsProxy,
    } as unknown as XTerm;
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("applyToTerminal batching", () => {
    it("should apply all theme options in a single batched Object.assign call", async () => {
      // Apply the theme first to load it
      await ThemeManager.applyThemeById("test-theme");

      // Reset the counter before testing applyToTerminal
      propertySetCount = 0;

      // Apply to terminal
      ThemeManager.applyToTerminal(mockTerminal);

      // With batching via Object.assign, we should see exactly 5 property assignments
      // (theme, fontFamily, fontSize, cursorBlink, cursorStyle)
      // but they're all applied in a single Object.assign call
      expect(propertySetCount).toBe(5);
    });

    it("should use Object.assign for batched updates", async () => {
      // Spy on Object.assign
      const assignSpy = vi.spyOn(Object, "assign");

      await ThemeManager.applyThemeById("test-theme");
      assignSpy.mockClear();

      ThemeManager.applyToTerminal(mockTerminal);

      // Verify Object.assign was called once with term.options as the target
      expect(assignSpy).toHaveBeenCalledTimes(1);
      expect(assignSpy).toHaveBeenCalledWith(
        mockTerminal.options,
        expect.objectContaining({
          theme: expect.any(Object),
        })
      );

      assignSpy.mockRestore();
    });

    it("should only apply terminal options once per theme change", async () => {
      await ThemeManager.applyThemeById("test-theme");

      propertySetCount = 0;
      ThemeManager.applyToTerminal(mockTerminal);

      const firstApplyCount = propertySetCount;

      // Apply again - should trigger the same number of updates
      propertySetCount = 0;
      ThemeManager.applyToTerminal(mockTerminal);

      // Each application should be consistent
      expect(propertySetCount).toBe(firstApplyCount);
    });

    it("should skip undefined theme properties", async () => {
      // Create a theme without optional properties
      const minimalTheme: QbitTheme = {
        schemaVersion: "1.0",
        name: "Minimal Theme",
        colors: {
          ui: {
            background: "#000000",
            foreground: "#ffffff",
            card: "#111111",
            cardForeground: "#ffffff",
            popover: "#111111",
            popoverForeground: "#ffffff",
            primary: "#0066ff",
            primaryForeground: "#ffffff",
            secondary: "#333333",
            secondaryForeground: "#ffffff",
            muted: "#222222",
            mutedForeground: "#888888",
            accent: "#0066ff",
            accentForeground: "#ffffff",
            destructive: "#ff0000",
            border: "#333333",
            input: "#222222",
            ring: "#0066ff",
            sidebar: "#111111",
            sidebarForeground: "#ffffff",
            sidebarPrimary: "#0066ff",
            sidebarPrimaryForeground: "#ffffff",
            sidebarAccent: "#222222",
            sidebarAccentForeground: "#ffffff",
            sidebarBorder: "#333333",
            sidebarRing: "#0066ff",
          },
          ansi: {
            black: "#000000",
            red: "#ff0000",
            green: "#00ff00",
            yellow: "#ffff00",
            blue: "#0000ff",
            magenta: "#ff00ff",
            cyan: "#00ffff",
            white: "#ffffff",
            brightBlack: "#666666",
            brightRed: "#ff6666",
            brightGreen: "#66ff66",
            brightYellow: "#ffff66",
            brightBlue: "#6666ff",
            brightMagenta: "#ff66ff",
            brightCyan: "#66ffff",
            brightWhite: "#ffffff",
            defaultFg: "#ffffff",
            defaultBg: "#000000",
          },
        },
        // No typography or terminal settings
      };

      mockThemes.set("minimal-theme", minimalTheme);
      await ThemeManager.applyThemeById("minimal-theme");

      propertySetCount = 0;
      ThemeManager.applyToTerminal(mockTerminal);

      // Should only set theme (1 property) since typography and terminal are undefined
      expect(propertySetCount).toBe(1);
    });
  });

  describe("theme change efficiency", () => {
    it("should not trigger redundant terminal updates when theme is unchanged", async () => {
      await ThemeManager.applyThemeById("test-theme");

      // First application
      propertySetCount = 0;
      ThemeManager.applyToTerminal(mockTerminal);
      const firstCount = propertySetCount;

      // Apply same theme again (no change)
      await ThemeManager.applyThemeById("test-theme");

      // Second application after "change" to same theme
      propertySetCount = 0;
      ThemeManager.applyToTerminal(mockTerminal);

      // Should be the same count (no extra updates)
      expect(propertySetCount).toBe(firstCount);
    });
  });
});
