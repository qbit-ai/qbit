/**
 * Theme type definitions for Qbit
 *
 * NOTE: This file provides TypeScript interfaces for the theme system.
 * For runtime validation, use the Zod schemas in theme-schema.ts.
 *
 * The types here are kept for backward compatibility during migration.
 * New code should import types from theme-schema.ts instead.
 */

// Re-export types from the Zod schema for new code
export type {
  QbitTheme as QbitThemeV2,
  ThemeColors as ThemeColorsV2,
  UIColors as UIColorsV2,
  AnsiColors as AnsiColorsV2,
  SyntaxColors as SyntaxColorsV2,
  ChartColors,
  ThemeTypography as ThemeTypographyV2,
  ThemeRadii as ThemeRadiiV2,
  TerminalSettings as TerminalSettingsV2,
  BackgroundSettings as BackgroundSettingsV2,
  ThemeEffects as ThemeEffectsV2,
} from "./theme-schema";

// =============================================================================
// Legacy Types (for backward compatibility during migration)
// =============================================================================

export interface QbitThemeMetadata {
  schemaVersion: string;
  name: string;
  version?: string;
  author?: string;
  license?: string;
  homepage?: string;
  tags?: string[];
  /** Light or dark mode */
  mode?: "light" | "dark";
  /** Theme description */
  description?: string;
}

export interface UIColors {
  background: string;
  foreground: string;
  card: string;
  cardForeground: string;
  popover: string;
  popoverForeground: string;
  primary: string;
  primaryForeground: string;
  secondary: string;
  secondaryForeground: string;
  muted: string;
  mutedForeground: string;
  accent: string;
  accentForeground: string;
  destructive: string;
  border: string;
  input: string;
  ring: string;
  chart?: {
    c1: string;
    c2: string;
    c3: string;
    c4: string;
    c5: string;
  };
  sidebar: string;
  sidebarForeground: string;
  sidebarPrimary: string;
  sidebarPrimaryForeground: string;
  sidebarAccent: string;
  sidebarAccentForeground: string;
  sidebarBorder: string;
  sidebarRing: string;
}

export interface AnsiColors {
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
  defaultFg: string;
  defaultBg: string;
}

/**
 * Syntax highlighting colors (theme-controlled)
 */
export interface SyntaxColors {
  /** Keywords: if, else, return, function, class, import, export, etc. */
  keyword: string;
  /** String literals */
  string: string;
  /** Comments */
  comment: string;
  /** Function names and calls */
  function: string;
  /** Variable names */
  variable: string;
  /** Constants: true, false, null, undefined */
  constant: string;
  /** Operators: +, -, *, /, =, ==, etc. */
  operator: string;
  /** Punctuation: (), [], {}, etc. */
  punctuation: string;
  /** Class names */
  className: string;
  /** Numeric literals */
  number: string;
  /** Object properties */
  property: string;
  /** HTML/JSX tags */
  tag: string;
  /** HTML/JSX attributes */
  attribute: string;
  /** Regular expressions */
  regexp: string;
  /** Type annotations */
  type: string;
}

export interface ThemeColors {
  ui: UIColors;
  ansi: AnsiColors;
  /** Syntax highlighting colors (optional for backward compatibility) */
  syntax?: SyntaxColors;
}

export interface TerminalTypography {
  fontFamily?: string;
  fontSize?: number;
  lineHeight?: number;
}

export interface UITypography {
  fontFamily?: string;
  headingFamily?: string;
}

export interface ThemeTypography {
  terminal?: TerminalTypography;
  ui?: UITypography;
}

export interface ThemeRadii {
  base?: string;
  sm?: string;
  md?: string;
  lg?: string;
  xl?: string;
}

export type CursorStyle = "block" | "underline" | "bar";

export interface BackgroundSettings {
  image?: string;
  size?: string;
  position?: string;
  opacity?: number;
}

export interface TerminalSettings {
  cursorStyle?: CursorStyle;
  cursorBlink?: boolean;
  selectionBackground?: string;
  webgl?: boolean;
}

export interface CursorEffect {
  style?: string;
  color?: string;
}

export interface ThemePlugin {
  id: string;
  name?: string;
  entry: string;
  config?: Record<string, unknown>;
}

export interface ThemeEffects {
  cursor?: CursorEffect;
  plugins?: ThemePlugin[];
}

export interface QbitTheme extends QbitThemeMetadata {
  colors: ThemeColors;
  typography?: ThemeTypography;
  radii?: ThemeRadii;
  background?: BackgroundSettings;
  terminal?: TerminalSettings;
  effects?: ThemeEffects;
}

/**
 * Theme registry entry
 */
export interface ThemeRegistryEntry {
  id: string;
  theme: QbitTheme;
  builtin?: boolean;
}
