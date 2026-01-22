/**
 * Theme Schema with Zod Validation
 *
 * This schema defines the structure of Qbit themes with:
 * - Runtime validation via Zod
 * - Type inference for TypeScript
 * - Support for --color-* CSS variable prefix convention
 * - Syntax highlighting tokens (theme-controlled)
 * - ANSI terminal colors
 * - Semantic UI colors
 */

import { z } from "zod";

export const THEME_SCHEMA_VERSION = "2.0.0";

// =============================================================================
// Color Token Schema
// =============================================================================

/**
 * Validates color values in various formats:
 * - Hex: #RGB, #RRGGBB, #RRGGBBAA
 * - RGB/RGBA: rgb(...), rgba(...)
 * - HSL: hsl(...), hsla(...)
 * - OKLCH: oklch(...)
 */
const ColorTokenSchema = z.string().refine(
  (val) => {
    // Hex colors
    if (/^#[0-9A-Fa-f]{3}$/.test(val)) return true;
    if (/^#[0-9A-Fa-f]{6}$/.test(val)) return true;
    if (/^#[0-9A-Fa-f]{8}$/.test(val)) return true;
    // Functional colors
    if (/^rgba?\(/.test(val)) return true;
    if (/^hsla?\(/.test(val)) return true;
    if (/^oklch\(/.test(val)) return true;
    return false;
  },
  { message: "Invalid color format. Use hex (#RGB, #RRGGBB), rgb(), rgba(), hsl(), hsla(), or oklch()." }
);

// =============================================================================
// ANSI Terminal Colors
// =============================================================================

export const AnsiColorsSchema = z.object({
  black: ColorTokenSchema,
  red: ColorTokenSchema,
  green: ColorTokenSchema,
  yellow: ColorTokenSchema,
  blue: ColorTokenSchema,
  magenta: ColorTokenSchema,
  cyan: ColorTokenSchema,
  white: ColorTokenSchema,
  brightBlack: ColorTokenSchema,
  brightRed: ColorTokenSchema,
  brightGreen: ColorTokenSchema,
  brightYellow: ColorTokenSchema,
  brightBlue: ColorTokenSchema,
  brightMagenta: ColorTokenSchema,
  brightCyan: ColorTokenSchema,
  brightWhite: ColorTokenSchema,
  defaultFg: ColorTokenSchema,
  defaultBg: ColorTokenSchema,
});

// =============================================================================
// Syntax Highlighting Colors (theme-controlled)
// =============================================================================

export const SyntaxColorsSchema = z.object({
  /** Keywords: if, else, return, function, class, import, export, etc. */
  keyword: ColorTokenSchema,
  /** String literals: "hello", 'world', `template` */
  string: ColorTokenSchema,
  /** Comments: // single line, /* multi-line *\/ */
  comment: ColorTokenSchema,
  /** Function names and calls */
  function: ColorTokenSchema,
  /** Variable names */
  variable: ColorTokenSchema,
  /** Constants: true, false, null, undefined, SCREAMING_CASE */
  constant: ColorTokenSchema,
  /** Operators: +, -, *, /, =, ==, ===, etc. */
  operator: ColorTokenSchema,
  /** Punctuation: (), [], {}, ,, ;, etc. */
  punctuation: ColorTokenSchema,
  /** Class names and type constructors */
  className: ColorTokenSchema,
  /** Numeric literals: 42, 3.14, 0xFF */
  number: ColorTokenSchema,
  /** Object properties and keys */
  property: ColorTokenSchema,
  /** HTML/JSX tags */
  tag: ColorTokenSchema,
  /** HTML/JSX attributes */
  attribute: ColorTokenSchema,
  /** Regular expressions */
  regexp: ColorTokenSchema,
  /** Type annotations and interfaces */
  type: ColorTokenSchema,
});

// =============================================================================
// Chart Colors
// =============================================================================

export const ChartColorsSchema = z.object({
  c1: ColorTokenSchema,
  c2: ColorTokenSchema,
  c3: ColorTokenSchema,
  c4: ColorTokenSchema,
  c5: ColorTokenSchema,
});

// =============================================================================
// UI Colors (semantic color tokens)
// =============================================================================

export const UIColorsSchema = z.object({
  // -------------------------------------------------------------------------
  // Core backgrounds
  // -------------------------------------------------------------------------
  /** Primary background color (darkest in dark mode) */
  background: ColorTokenSchema,
  /** Secondary/elevated background */
  backgroundSecondary: ColorTokenSchema,
  /** Tertiary background for subtle differentiation */
  backgroundTertiary: ColorTokenSchema,
  /** Background color on hover states */
  backgroundHover: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Foreground/text
  // -------------------------------------------------------------------------
  /** Primary text color */
  foreground: ColorTokenSchema,
  /** Secondary/less prominent text */
  foregroundSecondary: ColorTokenSchema,
  /** Muted/subtle text */
  foregroundMuted: ColorTokenSchema,
  /** Disabled text */
  foregroundDisabled: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Surfaces (cards, popovers, modals)
  // -------------------------------------------------------------------------
  /** Card/surface background */
  surface: ColorTokenSchema,
  /** Text on surfaces */
  surfaceForeground: ColorTokenSchema,
  /** Surface hover state */
  surfaceHover: ColorTokenSchema,

  /** Popover/dropdown background */
  popover: ColorTokenSchema,
  /** Popover text color */
  popoverForeground: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Primary action color
  // -------------------------------------------------------------------------
  /** Primary brand/action color */
  primary: ColorTokenSchema,
  /** Text on primary color */
  primaryForeground: ColorTokenSchema,
  /** Primary color on hover */
  primaryHover: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Secondary/neutral action color
  // -------------------------------------------------------------------------
  /** Secondary action color */
  secondary: ColorTokenSchema,
  /** Text on secondary color */
  secondaryForeground: ColorTokenSchema,
  /** Secondary color on hover */
  secondaryHover: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Accent color (highlight, links)
  // -------------------------------------------------------------------------
  /** Accent/highlight color */
  accent: ColorTokenSchema,
  /** Text on accent color */
  accentForeground: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Muted backgrounds (for subtle emphasis)
  // -------------------------------------------------------------------------
  /** Muted/subtle background */
  muted: ColorTokenSchema,
  /** Text on muted background */
  mutedForeground: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Semantic colors
  // -------------------------------------------------------------------------
  /** Success/positive state */
  success: ColorTokenSchema,
  /** Text on success background */
  successForeground: ColorTokenSchema,
  /** Warning state */
  warning: ColorTokenSchema,
  /** Text on warning background */
  warningForeground: ColorTokenSchema,
  /** Error/destructive state */
  error: ColorTokenSchema,
  /** Text on error background */
  errorForeground: ColorTokenSchema,
  /** Info/neutral semantic */
  info: ColorTokenSchema,
  /** Text on info background */
  infoForeground: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Borders
  // -------------------------------------------------------------------------
  /** Default border color */
  border: ColorTokenSchema,
  /** Border color on hover */
  borderHover: ColorTokenSchema,
  /** Border color on focus */
  borderFocus: ColorTokenSchema,
  /** Input field border */
  input: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Focus ring
  // -------------------------------------------------------------------------
  /** Focus ring/outline color */
  ring: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Overlay (modal backdrops)
  // -------------------------------------------------------------------------
  /** Modal/overlay backdrop */
  overlay: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Sidebar (if different from main surfaces)
  // -------------------------------------------------------------------------
  /** Sidebar background */
  sidebar: ColorTokenSchema,
  /** Sidebar text */
  sidebarForeground: ColorTokenSchema,
  /** Sidebar primary action */
  sidebarPrimary: ColorTokenSchema,
  /** Text on sidebar primary */
  sidebarPrimaryForeground: ColorTokenSchema,
  /** Sidebar accent/hover */
  sidebarAccent: ColorTokenSchema,
  /** Text on sidebar accent */
  sidebarAccentForeground: ColorTokenSchema,
  /** Sidebar border */
  sidebarBorder: ColorTokenSchema,
  /** Sidebar focus ring */
  sidebarRing: ColorTokenSchema,

  // -------------------------------------------------------------------------
  // Chart colors
  // -------------------------------------------------------------------------
  /** Chart/visualization colors */
  chart: ChartColorsSchema,
});

// =============================================================================
// Theme Colors (all color categories combined)
// =============================================================================

export const ThemeColorsSchema = z.object({
  /** UI semantic colors */
  ui: UIColorsSchema,
  /** ANSI terminal colors */
  ansi: AnsiColorsSchema,
  /** Syntax highlighting colors */
  syntax: SyntaxColorsSchema,
});

// =============================================================================
// Typography
// =============================================================================

export const TerminalTypographySchema = z.object({
  /** Terminal font family */
  fontFamily: z.string().optional(),
  /** Terminal font size in pixels */
  fontSize: z.number().min(8).max(32).optional(),
  /** Terminal line height multiplier */
  lineHeight: z.number().min(1).max(3).optional(),
});

export const UITypographySchema = z.object({
  /** Primary UI font family */
  fontFamily: z.string().optional(),
  /** Heading font family */
  headingFamily: z.string().optional(),
});

export const ThemeTypographySchema = z.object({
  /** Terminal typography settings */
  terminal: TerminalTypographySchema.optional(),
  /** UI typography settings */
  ui: UITypographySchema.optional(),
});

// =============================================================================
// Border Radii
// =============================================================================

export const ThemeRadiiSchema = z.object({
  /** Base radius value */
  base: z.string().optional(),
  /** Small radius */
  sm: z.string().optional(),
  /** Medium radius */
  md: z.string().optional(),
  /** Large radius */
  lg: z.string().optional(),
  /** Extra large radius */
  xl: z.string().optional(),
});

// =============================================================================
// Terminal Settings
// =============================================================================

export const CursorStyleSchema = z.enum(["block", "underline", "bar"]);

export const TerminalSettingsSchema = z.object({
  /** Cursor style */
  cursorStyle: CursorStyleSchema.optional(),
  /** Whether cursor blinks */
  cursorBlink: z.boolean().optional(),
  /** Selection background color */
  selectionBackground: ColorTokenSchema.optional(),
  /** Enable WebGL rendering */
  webgl: z.boolean().optional(),
});

// =============================================================================
// Background Settings
// =============================================================================

export const BackgroundSettingsSchema = z.object({
  /** Background image URL or path */
  image: z.string().optional(),
  /** CSS background-size */
  size: z.string().optional(),
  /** CSS background-position */
  position: z.string().optional(),
  /** Background opacity (0-1) */
  opacity: z.number().min(0).max(1).optional(),
});

// =============================================================================
// Effects & Plugins
// =============================================================================

export const ThemePluginSchema = z.object({
  /** Unique plugin identifier */
  id: z.string(),
  /** Display name */
  name: z.string().optional(),
  /** Plugin entry point */
  entry: z.string(),
  /** Plugin configuration */
  config: z.record(z.unknown()).optional(),
});

export const CursorEffectSchema = z.object({
  /** Cursor effect style */
  style: z.string().optional(),
  /** Cursor effect color */
  color: ColorTokenSchema.optional(),
});

export const ThemeEffectsSchema = z.object({
  /** Custom cursor effects */
  cursor: CursorEffectSchema.optional(),
  /** Theme plugins */
  plugins: z.array(ThemePluginSchema).optional(),
});

// =============================================================================
// Theme Metadata
// =============================================================================

export const ThemeMetadataSchema = z.object({
  /** Schema version for compatibility checking */
  schemaVersion: z.string().regex(/^\d+\.\d+\.\d+$/, "schemaVersion must be semver format"),
  /** Theme display name */
  name: z.string().min(1),
  /** Theme version */
  version: z
    .string()
    .regex(/^\d+\.\d+\.\d+$/, "version must be semver format")
    .optional(),
  /** Theme author */
  author: z.string().optional(),
  /** Theme license */
  license: z.string().optional(),
  /** Theme homepage URL */
  homepage: z.string().url().optional(),
  /** Searchable tags */
  tags: z.array(z.string()).optional(),
  /** Light or dark mode */
  mode: z.enum(["light", "dark"]).optional(),
  /** Theme description */
  description: z.string().optional(),
});

// =============================================================================
// Complete Theme Schema
// =============================================================================

export const QbitThemeSchema = z
  .object({
    /** All color definitions */
    colors: ThemeColorsSchema,
    /** Typography settings */
    typography: ThemeTypographySchema.optional(),
    /** Border radius values */
    radii: ThemeRadiiSchema.optional(),
    /** Background settings (image, etc.) */
    background: BackgroundSettingsSchema.optional(),
    /** Terminal-specific settings */
    terminal: TerminalSettingsSchema.optional(),
    /** Effects and plugins */
    effects: ThemeEffectsSchema.optional(),
  })
  .merge(ThemeMetadataSchema);

// =============================================================================
// Type Exports (inferred from Zod schemas)
// =============================================================================

export type QbitTheme = z.infer<typeof QbitThemeSchema>;
export type ThemeColors = z.infer<typeof ThemeColorsSchema>;
export type UIColors = z.infer<typeof UIColorsSchema>;
export type AnsiColors = z.infer<typeof AnsiColorsSchema>;
export type SyntaxColors = z.infer<typeof SyntaxColorsSchema>;
export type ChartColors = z.infer<typeof ChartColorsSchema>;
export type ThemeMetadata = z.infer<typeof ThemeMetadataSchema>;
export type ThemeTypography = z.infer<typeof ThemeTypographySchema>;
export type TerminalTypography = z.infer<typeof TerminalTypographySchema>;
export type UITypography = z.infer<typeof UITypographySchema>;
export type ThemeRadii = z.infer<typeof ThemeRadiiSchema>;
export type TerminalSettings = z.infer<typeof TerminalSettingsSchema>;
export type CursorStyle = z.infer<typeof CursorStyleSchema>;
export type BackgroundSettings = z.infer<typeof BackgroundSettingsSchema>;
export type ThemeEffects = z.infer<typeof ThemeEffectsSchema>;
export type ThemePlugin = z.infer<typeof ThemePluginSchema>;
export type CursorEffect = z.infer<typeof CursorEffectSchema>;

// =============================================================================
// Validation Helpers
// =============================================================================

/**
 * Validate theme data and throw on error.
 * @throws {z.ZodError} If validation fails
 */
export function validateTheme(themeData: unknown): QbitTheme {
  return QbitThemeSchema.parse(themeData);
}

/**
 * Safely validate theme data without throwing.
 * @returns Object with success status and either data or error
 */
export function safeValidateTheme(themeData: unknown): {
  success: boolean;
  data?: QbitTheme;
  error?: z.ZodError;
} {
  const result = QbitThemeSchema.safeParse(themeData);
  if (result.success) {
    return { success: true, data: result.data };
  }
  return { success: false, error: result.error };
}

/**
 * Check if a theme schema version is compatible with the current version.
 * Supports semver major.minor compatibility (patch differences are OK).
 */
export function isSchemaVersionCompatible(themeSchemaVersion: string): boolean {
  const [currentMajor, currentMinor] = THEME_SCHEMA_VERSION.split(".").map(Number);
  const [themeMajor, themeMinor] = themeSchemaVersion.split(".").map(Number);

  // Major version must match
  if (themeMajor !== currentMajor) {
    return false;
  }

  // Theme minor version can be <= current minor version
  return themeMinor <= currentMinor;
}
