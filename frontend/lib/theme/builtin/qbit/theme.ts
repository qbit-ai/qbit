import type { QbitTheme } from "../../types";

// Qbit Color Palette - Redesigned with single teal accent
const palette = {
  // Core backgrounds (darkest to lightest)
  bgPrimary: "#0d0f12", // Main background
  bgSecondary: "#13161b", // Cards/elevated surfaces
  bgTertiary: "#1a1e24", // Secondary surfaces
  bgHover: "#1f242b", // Hover states

  // Text colors
  textPrimary: "#e8eaed", // Primary text
  textSecondary: "#9aa0a6", // Secondary text
  textMuted: "#5f6368", // Muted/disabled text

  // Accent colors
  accent: "#5eead4", // Teal - primary accent
  accentDark: "#0d0f12", // Dark text on accent

  // Semantic colors
  success: "#34d399", // Success/completion
  destructive: "#f7768e", // Error/destructive

  // Border colors
  borderSubtle: "rgba(255, 255, 255, 0.06)",
  borderMedium: "rgba(255, 255, 255, 0.1)",

  // Ring/focus
  ring: "#5f6368",

  // Chart colors (keep existing)
  chartPurple: "oklch(0.488 0.243 264.376)",
  chartGreen: "oklch(0.696 0.17 162.48)",
  chartYellow: "oklch(0.769 0.188 70.08)",
  chartMagenta: "oklch(0.627 0.265 303.9)",
  chartOrange: "oklch(0.645 0.246 16.439)",

  // ANSI terminal colors (keep for terminal output)
  ansiBlack: "#414868",
  ansiBlue: "#7aa2f7",
  ansiBrightBlack: "#565f89",
  ansiBrightBlue: "#99b4ff",
  ansiBrightCyan: "#a6e4ff",
  ansiBrightGreen: "#b9f27c",
  ansiBrightMagenta: "#d4b8ff",
  ansiBrightRed: "#ff9e9e",
  ansiBrightWhite: "#e9ecf5",
  ansiBrightYellow: "#ffd07b",
  ansiCyan: "#7dcfff",
  ansiDefaultBg: "#1a1b26",
  ansiDefaultFg: "#c0caf5",
  ansiGreen: "#9ece6a",
  ansiMagenta: "#bb9af7",
  ansiRed: "#f7768e",
  ansiWhite: "#c0caf5",
  ansiYellow: "#e0af68",
};

export const qbitTheme: QbitTheme = {
  author: "Qbit Team",
  license: "MIT",
  name: "Qbit",
  schemaVersion: "1.0.0",
  version: "1.0.0",

  colors: {
    ansi: {
      black: palette.ansiBlack,
      blue: palette.ansiBlue,
      brightBlack: palette.ansiBrightBlack,
      brightBlue: palette.ansiBrightBlue,
      brightCyan: palette.ansiBrightCyan,
      brightGreen: palette.ansiBrightGreen,
      brightMagenta: palette.ansiBrightMagenta,
      brightRed: palette.ansiBrightRed,
      brightWhite: palette.ansiBrightWhite,
      brightYellow: palette.ansiBrightYellow,
      cyan: palette.ansiCyan,
      defaultBg: palette.ansiDefaultBg,
      defaultFg: palette.ansiDefaultFg,
      green: palette.ansiGreen,
      magenta: palette.ansiMagenta,
      red: palette.ansiRed,
      white: palette.ansiWhite,
      yellow: palette.ansiYellow,
    },

    ui: {
      accent: palette.accent,
      accentForeground: palette.accentDark,
      background: palette.bgPrimary,
      border: palette.borderSubtle,
      card: palette.bgSecondary,
      cardForeground: palette.textPrimary,

      chart: {
        c1: palette.chartPurple,
        c2: palette.chartGreen,
        c3: palette.chartYellow,
        c4: palette.chartMagenta,
        c5: palette.chartOrange,
      },

      destructive: palette.destructive,
      foreground: palette.textPrimary,
      input: palette.borderMedium,
      muted: palette.bgTertiary,
      mutedForeground: palette.textSecondary,
      popover: palette.bgSecondary,
      popoverForeground: palette.textPrimary,
      primary: palette.accent,
      primaryForeground: palette.accentDark,
      ring: palette.ring,
      secondary: palette.bgTertiary,
      secondaryForeground: palette.textPrimary,
      sidebar: palette.bgSecondary,
      sidebarAccent: palette.bgTertiary,
      sidebarAccentForeground: palette.textPrimary,
      sidebarBorder: palette.borderSubtle,
      sidebarForeground: palette.textPrimary,
      sidebarPrimary: palette.accent,
      sidebarPrimaryForeground: palette.accentDark,
      sidebarRing: palette.ring,
    },
  },

  effects: {
    plugins: [],
  },

  radii: {
    base: "0.5rem",
  },

  terminal: {
    cursorBlink: true,
    cursorStyle: "block",
    selectionBackground: palette.bgTertiary,
  },

  typography: {
    terminal: {
      fontFamily: "'JetBrains Mono', monospace",
      fontSize: 14,
    },
    ui: {
      fontFamily:
        "'Source Sans 3', Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif",
    },
  },
};
