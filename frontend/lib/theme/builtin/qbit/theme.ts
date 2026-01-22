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
  textDisabled: "#3c4043", // Disabled text

  // Accent colors
  accent: "#5eead4", // Teal - primary accent
  accentDark: "#0d0f12", // Dark text on accent
  accentHover: "#2dd4bf", // Accent hover state

  // Semantic colors
  success: "#34d399", // Success/completion
  successDark: "#0d0f12", // Text on success
  warning: "#fbbf24", // Warning state
  warningDark: "#0d0f12", // Text on warning
  info: "#7aa2f7", // Info state
  infoDark: "#0d0f12", // Text on info
  destructive: "#f7768e", // Error/destructive
  destructiveDark: "#0d0f12", // Text on error

  // Border colors
  borderSubtle: "rgba(255, 255, 255, 0.06)",
  borderMedium: "rgba(255, 255, 255, 0.1)",
  borderFocus: "#5eead4",

  // Overlay
  overlay: "rgba(0, 0, 0, 0.6)",

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

  // Syntax highlighting colors (Tokyo Night inspired)
  syntaxKeyword: "#bb9af7", // Purple - keywords
  syntaxString: "#9ece6a", // Green - strings
  syntaxComment: "#565f89", // Gray - comments
  syntaxFunction: "#7aa2f7", // Blue - functions
  syntaxVariable: "#c0caf5", // Light blue - variables
  syntaxConstant: "#ff9e64", // Orange - constants
  syntaxOperator: "#89ddff", // Cyan - operators
  syntaxPunctuation: "#9aa0a6", // Gray - punctuation
  syntaxClassName: "#2ac3de", // Cyan - class names
  syntaxNumber: "#ff9e64", // Orange - numbers
  syntaxProperty: "#7dcfff", // Cyan - properties
  syntaxTag: "#f7768e", // Red - HTML tags
  syntaxAttribute: "#bb9af7", // Purple - attributes
  syntaxRegexp: "#b4f9f8", // Light cyan - regex
  syntaxType: "#2ac3de", // Cyan - types
};

export const qbitTheme: QbitTheme = {
  author: "Qbit Team",
  license: "MIT",
  name: "Qbit",
  schemaVersion: "1.0.0",
  version: "1.0.0",
  description: "Default dark theme for Qbit with teal accent",
  tags: ["dark", "default", "teal"],

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

    syntax: {
      keyword: palette.syntaxKeyword,
      string: palette.syntaxString,
      comment: palette.syntaxComment,
      function: palette.syntaxFunction,
      variable: palette.syntaxVariable,
      constant: palette.syntaxConstant,
      operator: palette.syntaxOperator,
      punctuation: palette.syntaxPunctuation,
      className: palette.syntaxClassName,
      number: palette.syntaxNumber,
      property: palette.syntaxProperty,
      tag: palette.syntaxTag,
      attribute: palette.syntaxAttribute,
      regexp: palette.syntaxRegexp,
      type: palette.syntaxType,
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
