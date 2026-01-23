import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Use vi.hoisted to ensure mocks are available when vi.mock runs
const {
  mockStartPreview,
  mockCommitPreview,
  mockCancelPreview,
  mockUnregister,
  mockGetAll,
  mockThemeA,
  mockThemeB,
} = vi.hoisted(() => {
  // Create mock theme helper
  const createMockTheme = (name: string) => ({
    schemaVersion: "1.0",
    name,
    version: "1.0.0",
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
  });

  return {
    mockStartPreview: vi.fn().mockResolvedValue(true),
    mockCommitPreview: vi.fn(),
    mockCancelPreview: vi.fn().mockResolvedValue(undefined),
    mockUnregister: vi.fn().mockResolvedValue(true),
    mockGetAll: vi.fn(() => [
      { id: "theme-a", theme: createMockTheme("Theme A"), builtin: true },
      { id: "theme-b", theme: createMockTheme("Theme B"), builtin: false },
    ]),
    mockThemeA: createMockTheme("Theme A"),
    mockThemeB: createMockTheme("Theme B"),
  };
});

vi.mock("@/lib/appVersion", () => ({
  getAppVersion: () => "0.2.9",
}));

vi.mock("../lib/theme/ThemeManager", () => ({
  ThemeManager: {
    getTheme: () => mockThemeA,
    getThemeId: () => "theme-a",
    onChange: () => () => {},
    applyThemeById: vi.fn().mockResolvedValue(true),
    startPreview: mockStartPreview,
    commitPreview: mockCommitPreview,
    cancelPreview: mockCancelPreview,
    loadThemeFromObject: vi.fn().mockResolvedValue(undefined),
    tryLoadPersistedTheme: vi.fn().mockResolvedValue(false),
  },
}));

vi.mock("../lib/theme/registry", () => ({
  ThemeRegistry: {
    initialize: vi.fn().mockResolvedValue(undefined),
    getAll: mockGetAll,
    onChange: () => () => {},
    unregister: mockUnregister,
  },
}));

import { ThemeProvider, useTheme } from "./useTheme";

describe("useTheme", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: ReactNode }) => (
    <ThemeProvider>{children}</ThemeProvider>
  );

  it("throws error when used outside ThemeProvider", () => {
    // Suppress console.error for this test
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    expect(() => {
      renderHook(() => useTheme());
    }).toThrow("useTheme must be used within a ThemeProvider");

    consoleSpy.mockRestore();
  });

  it("provides previewTheme that calls ThemeManager.startPreview", async () => {
    const { result } = renderHook(() => useTheme(), { wrapper });

    await act(async () => {
      const success = await result.current.previewTheme("theme-b");
      expect(success).toBe(true);
    });

    expect(mockStartPreview).toHaveBeenCalledWith("theme-b");
  });

  it("provides commitThemePreview that calls ThemeManager.commitPreview", async () => {
    const { result } = await act(async () => renderHook(() => useTheme(), { wrapper }));

    act(() => {
      result.current.commitThemePreview();
    });

    expect(mockCommitPreview).toHaveBeenCalled();
  });

  it("provides cancelThemePreview that calls ThemeManager.cancelPreview", async () => {
    const { result } = renderHook(() => useTheme(), { wrapper });

    await act(async () => {
      await result.current.cancelThemePreview();
    });

    expect(mockCancelPreview).toHaveBeenCalled();
  });

  it("provides availableThemes from ThemeRegistry", async () => {
    const { result } = await act(async () => renderHook(() => useTheme(), { wrapper }));

    expect(result.current.availableThemes).toEqual([
      { id: "theme-a", name: "Theme A", builtin: true, version: "1.0.0", compatible: true },
      { id: "theme-b", name: "Theme B", builtin: false, version: "1.0.0", compatible: true },
    ]);
  });

  it("marks incompatible themes with a message", async () => {
    mockGetAll.mockReturnValue([
      {
        id: "theme-incompatible-min",
        theme: { ...mockThemeA, name: "Theme Incompatible", minAppVersion: "1.0.0" },
        builtin: false,
      },
    ]);

    const { result } = await act(async () => renderHook(() => useTheme(), { wrapper }));

    expect(result.current.availableThemes).toEqual([
      {
        id: "theme-incompatible-min",
        name: "Theme Incompatible",
        builtin: false,
        version: "1.0.0",
        compatible: false,
        compatibilityMessage: "Requires Qbit >= 1.0.0 (you are on 0.2.9)",
      },
    ]);
  });

  it("provides deleteTheme that calls ThemeRegistry.unregister", async () => {
    const { result } = renderHook(() => useTheme(), { wrapper });

    await act(async () => {
      const success = await result.current.deleteTheme("theme-b");
      expect(success).toBe(true);
    });

    expect(mockUnregister).toHaveBeenCalledWith("theme-b");
  });
});
