/**
 * Theme system exports
 * Centralized exports for the theme module
 */

export { ThemeRegistry } from "./registry";
export { applyTheme, loadThemeFromFile, loadThemeFromUrl } from "./ThemeLoader";
export { ThemeManager } from "./ThemeManager";

// Schema and validation
export { 
  QbitThemeSchema, 
  validateTheme, 
  safeValidateTheme,
  isSchemaVersionCompatible,
  THEME_SCHEMA_VERSION,
} from "./theme-schema";

// Export/Import utilities
export {
  exportTheme,
  exportThemeToString,
  importThemeFromFile,
  importThemeFromFileHandle,
  importThemeFromString,
  copyThemeToClipboard,
  importThemeFromClipboard,
  generateThemeShareUrl,
  parseThemeFromShareUrl,
  smartImportTheme,
  type ThemeImportResult,
} from "./theme-export";

export type {
  AnsiColors,
  CursorEffect,
  CursorStyle,
  QbitTheme,
  QbitThemeMetadata,
  SyntaxColors,
  TerminalSettings,
  TerminalTypography,
  ThemeColors,
  ThemeEffects,
  ThemePlugin,
  ThemeRadii,
  ThemeRegistryEntry,
  ThemeTypography,
  UIColors,
  UITypography,
} from "./types";

