import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { QbitTheme } from "./types";

// Mock dependencies before importing ThemeManager
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

// Create a minimal mock theme for testing
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

describe("ThemeManager", () => {
  // Mock localStorage
  const localStorageMock = (() => {
    let store: Record<string, string> = {};
    return {
      getItem: vi.fn((key: string) => store[key] ?? null),
      setItem: vi.fn((key: string, value: string) => {
        store[key] = value;
      }),
      removeItem: vi.fn((key: string) => {
        delete store[key];
      }),
      clear: vi.fn(() => {
        store = {};
      }),
    };
  })();

  beforeEach(() => {
    // Setup localStorage mock
    Object.defineProperty(window, "localStorage", { value: localStorageMock });
    localStorageMock.clear();
    vi.clearAllMocks();

    // Reset mock themes
    mockThemes.clear();

    // Add test themes to the mock registry
    mockThemes.set("theme-a", createMockTheme("Theme A"));
    mockThemes.set("theme-b", createMockTheme("Theme B"));
    mockThemes.set("theme-c", createMockTheme("Theme C"));
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe("Preview Mode", () => {
    it("startPreview applies theme without persisting to localStorage", async () => {
      // First apply a theme normally
      await ThemeManager.applyThemeById("theme-a");
      expect(localStorageMock.setItem).toHaveBeenCalledWith("qbit.currentThemeId", "theme-a");

      localStorageMock.setItem.mockClear();

      // Start preview with a different theme
      const result = await ThemeManager.startPreview("theme-b");

      expect(result).toBe(true);
      expect(ThemeManager.getThemeId()).toBe("theme-b");
      expect(ThemeManager.isInPreviewMode()).toBe(true);
      // Should NOT persist to localStorage during preview
      expect(localStorageMock.setItem).not.toHaveBeenCalled();
    });

    it("startPreview saves original theme ID for later revert", async () => {
      await ThemeManager.applyThemeById("theme-a");
      await ThemeManager.startPreview("theme-b");

      expect(ThemeManager.getThemeId()).toBe("theme-b");
      expect(ThemeManager.isInPreviewMode()).toBe(true);
    });

    it("multiple previews do not overwrite original theme", async () => {
      // Apply original theme
      await ThemeManager.applyThemeById("theme-a");

      // Start preview
      await ThemeManager.startPreview("theme-b");
      expect(ThemeManager.getThemeId()).toBe("theme-b");

      // Preview another theme
      await ThemeManager.startPreview("theme-c");
      expect(ThemeManager.getThemeId()).toBe("theme-c");

      // Cancel should revert to original (theme-a), not the intermediate (theme-b)
      await ThemeManager.cancelPreview();
      expect(ThemeManager.getThemeId()).toBe("theme-a");
    });

    it("commitPreview persists the previewed theme", async () => {
      await ThemeManager.applyThemeById("theme-a");
      localStorageMock.setItem.mockClear();

      await ThemeManager.startPreview("theme-b");
      expect(localStorageMock.setItem).not.toHaveBeenCalled();

      ThemeManager.commitPreview();

      expect(localStorageMock.setItem).toHaveBeenCalledWith("qbit.currentThemeId", "theme-b");
      expect(ThemeManager.isInPreviewMode()).toBe(false);
      expect(ThemeManager.getThemeId()).toBe("theme-b");
    });

    it("cancelPreview reverts to original theme", async () => {
      await ThemeManager.applyThemeById("theme-a");
      await ThemeManager.startPreview("theme-b");

      expect(ThemeManager.getThemeId()).toBe("theme-b");

      await ThemeManager.cancelPreview();

      expect(ThemeManager.getThemeId()).toBe("theme-a");
      expect(ThemeManager.isInPreviewMode()).toBe(false);
    });

    it("cancelPreview does nothing if not in preview mode", async () => {
      await ThemeManager.applyThemeById("theme-a");

      await ThemeManager.cancelPreview();

      expect(ThemeManager.getThemeId()).toBe("theme-a");
      expect(ThemeManager.isInPreviewMode()).toBe(false);
    });

    it("commitPreview does nothing if not in preview mode", async () => {
      await ThemeManager.applyThemeById("theme-a");
      localStorageMock.setItem.mockClear();

      ThemeManager.commitPreview();

      // Should not call setItem again since we're not in preview mode
      expect(localStorageMock.setItem).not.toHaveBeenCalled();
    });

    it("isInPreviewMode returns correct state", async () => {
      expect(ThemeManager.isInPreviewMode()).toBe(false);

      await ThemeManager.startPreview("theme-a");
      expect(ThemeManager.isInPreviewMode()).toBe(true);

      ThemeManager.commitPreview();
      expect(ThemeManager.isInPreviewMode()).toBe(false);

      await ThemeManager.startPreview("theme-b");
      expect(ThemeManager.isInPreviewMode()).toBe(true);

      await ThemeManager.cancelPreview();
      expect(ThemeManager.isInPreviewMode()).toBe(false);
    });

    it("startPreview returns false if theme not found", async () => {
      const result = await ThemeManager.startPreview("non-existent-theme");

      expect(result).toBe(false);
    });

    it("applyThemeById with persist=false does not save to localStorage", async () => {
      await ThemeManager.applyThemeById("theme-a", false);

      expect(ThemeManager.getThemeId()).toBe("theme-a");
      expect(localStorageMock.setItem).not.toHaveBeenCalled();
    });

    it("applyThemeById with persist=true (default) saves to localStorage", async () => {
      // Ensure we're not in preview mode from previous tests
      await ThemeManager.cancelPreview();
      localStorageMock.setItem.mockClear();

      await ThemeManager.applyThemeById("theme-a");

      expect(localStorageMock.setItem).toHaveBeenCalledWith("qbit.currentThemeId", "theme-a");
    });
  });

  describe("Theme Listeners", () => {
    it("notifies listeners when theme changes via startPreview", async () => {
      const listener = vi.fn();
      const unsubscribe = ThemeManager.onChange(listener);

      await ThemeManager.startPreview("theme-a");

      expect(listener).toHaveBeenCalledWith(mockThemes.get("theme-a"));

      unsubscribe();
    });

    it("notifies listeners when preview is cancelled", async () => {
      await ThemeManager.applyThemeById("theme-a");

      const listener = vi.fn();
      const unsubscribe = ThemeManager.onChange(listener);

      await ThemeManager.startPreview("theme-b");
      listener.mockClear();

      await ThemeManager.cancelPreview();

      expect(listener).toHaveBeenCalledWith(mockThemes.get("theme-a"));

      unsubscribe();
    });

    it("unsubscribe removes listener", async () => {
      const listener = vi.fn();
      const unsubscribe = ThemeManager.onChange(listener);

      unsubscribe();

      await ThemeManager.applyThemeById("theme-a");

      expect(listener).not.toHaveBeenCalled();
    });
  });
});
