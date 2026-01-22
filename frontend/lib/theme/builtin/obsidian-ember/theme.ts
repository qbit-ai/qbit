import type { QbitTheme } from "../../types";

// Obsidian Ember Color Palette
const palette = {
  // Amber/Gold - primary brand color
  amber400: "#E0B97A", // Bright amber
  amber500: "#D2A968", // Primary golden amber
  amber600: "#A8733D", // Darker amber

  // Blue - informational
  blue500: "#516A80", // Bright blue
  blue600: "#415464", // Dark blue

  // Cyan
  cyan400: "#8FB7B6", // Bright cyan
  cyan500: "#7BA1A0", // Primary cyan

  // Neutrals - grays from darkest (950) to lightest (50)
  gray50: "#E8E2D3", // Lightest - primary text
  gray100: "#DDD7C7", // Light text
  gray400: "#8B8576", // Muted text
  gray500: "#3A383C", // Muted elements
  gray600: "#343236", // Borders and inputs
  gray650: "#323034", // Selection background
  gray700: "#2B2A30", // Accent surfaces
  gray800: "#1D1C20", // Secondary surfaces
  gray850: "#1A191D", // Sidebar background
  gray900: "#141417", // Very dark - cards/elevated surfaces
  gray950: "#0A0A0C", // Darkest - main background
  gray975: "#0E0E10", // Slightly lighter than black
  gray1000: "#0D0C0A", // Near black - used for text on bright backgrounds

  // Green - success states
  green400: "#7FB69D", // Bright green
  green500: "#6EA38A", // Primary green

  // Purple/Magenta
  purple400: "#A98AA0", // Bright purple
  purple500: "#9A7A8F", // Primary purple

  // Red - destructive/error states
  red400: "#C0463E", // Bright red
  red500: "#B33B32", // Primary red

  // Syntax highlighting colors (warm palette to match ember theme)
  syntaxKeyword: "#D2A968", // Amber - keywords
  syntaxString: "#6EA38A", // Green - strings
  syntaxComment: "#8B8576", // Gray - comments
  syntaxFunction: "#516A80", // Blue - functions
  syntaxVariable: "#E8E2D3", // Light gray - variables
  syntaxConstant: "#C0463E", // Red - constants
  syntaxOperator: "#8FB7B6", // Cyan - operators
  syntaxPunctuation: "#8B8576", // Gray - punctuation
  syntaxClassName: "#7BA1A0", // Cyan - class names
  syntaxNumber: "#C0463E", // Red - numbers
  syntaxProperty: "#8FB7B6", // Cyan - properties
  syntaxTag: "#B33B32", // Red - HTML tags
  syntaxAttribute: "#D2A968", // Amber - attributes
  syntaxRegexp: "#A98AA0", // Purple - regex
  syntaxType: "#7BA1A0", // Cyan - types
};

export const obsidianEmber: QbitTheme = {
  author: "ally",
  license: "MIT",
  name: "Obsidian Ember",
  schemaVersion: "1.0.0",
  version: "1.0.0",
  description: "Warm amber-accented dark theme with background image",
  tags: ["dark", "warm", "amber", "ember"],

  background: {
    image: "assets/background.jpeg",
    opacity: 0.1,
    position: "center",
    size: "cover",
  },

  colors: {
    ansi: {
      black: palette.gray975,
      blue: palette.blue600,
      brightBlack: palette.gray500,
      brightBlue: palette.blue500,
      brightCyan: palette.cyan400,
      brightGreen: palette.green400,
      brightMagenta: palette.purple400,
      brightRed: palette.red400,
      brightWhite: palette.gray50,
      brightYellow: palette.amber400,
      cyan: palette.cyan500,
      defaultBg: palette.gray950,
      defaultFg: palette.gray50,
      green: palette.green500,
      magenta: palette.purple500,
      red: palette.red500,
      white: palette.gray100,
      yellow: palette.amber500,
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
      accent: palette.gray700,
      accentForeground: palette.gray50,
      background: palette.gray950,
      border: palette.gray600,
      card: palette.gray900,
      cardForeground: palette.gray50,

      chart: {
        c1: palette.amber500,
        c2: palette.amber600,
        c3: palette.gray400,
        c4: palette.red500,
        c5: palette.green500,
      },

      destructive: palette.red500,
      foreground: palette.gray50,
      input: palette.gray600,
      muted: palette.gray800,
      mutedForeground: palette.gray400,
      popover: palette.gray900,
      popoverForeground: palette.gray50,
      primary: palette.amber500,
      primaryForeground: palette.gray1000,
      ring: palette.gray400,
      secondary: palette.gray800,
      secondaryForeground: palette.gray50,
      sidebar: palette.gray850,
      sidebarAccent: palette.gray700,
      sidebarAccentForeground: palette.gray50,
      sidebarBorder: palette.gray600,
      sidebarForeground: palette.gray50,
      sidebarPrimary: palette.amber500,
      sidebarPrimaryForeground: palette.gray1000,
      sidebarRing: palette.gray400,
    },
  },

  effects: {
    plugins: [],
  },

  radii: {
    base: "0.625rem",
  },

  terminal: {
    cursorBlink: true,
    cursorStyle: "block",
    selectionBackground: palette.gray650,
  },

  typography: {
    terminal: {
      fontFamily: "Rodin, RodinNTLG, Avenir Next, Helvetica Neue, sans-serif",
      fontSize: 14,
    },
    ui: {
      fontFamily: "Rodin, RodinNTLG, Avenir Next, Helvetica Neue, sans-serif",
    },
  },
};
