/**
 * Theme export/import utilities
 */

import type { QbitTheme } from "./types";
import { safeValidateTheme } from "./theme-schema";

/**
 * Export a theme to a JSON file download
 */
export function exportTheme(theme: QbitTheme): void {
  const json = JSON.stringify(theme, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  
  const filename = `${theme.name.toLowerCase().replace(/\s+/g, "-")}-theme.json`;
  
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Export a theme to a JSON string
 */
export function exportThemeToString(theme: QbitTheme): string {
  return JSON.stringify(theme, null, 2);
}

/**
 * Import a theme from a file
 * Returns a promise that resolves with the validated theme or rejects with an error
 */
export function importThemeFromFile(): Promise<QbitTheme> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) {
        reject(new Error("No file selected"));
        return;
      }
      
      try {
        const theme = await importThemeFromFileHandle(file);
        resolve(theme);
      } catch (error) {
        reject(error);
      }
    };
    
    input.click();
  });
}

/**
 * Import a theme from a File object
 */
export async function importThemeFromFileHandle(file: File): Promise<QbitTheme> {
  const text = await file.text();
  return importThemeFromString(text);
}

/**
 * Import a theme from a JSON string
 */
export function importThemeFromString(json: string): QbitTheme {
  let parsed: unknown;
  
  try {
    parsed = JSON.parse(json);
  } catch (e) {
    throw new Error(`Invalid JSON: ${(e as Error).message}`);
  }
  
  const result = safeValidateTheme(parsed);
  
  if (!result.success) {
    const errorMsg = result.error || "Unknown validation error";
    throw new Error(`Invalid theme: ${errorMsg}`);
  }
  
  // Cast through unknown to handle schema differences
  return result.data as unknown as QbitTheme;
}

/**
 * Copy theme JSON to clipboard
 */
export async function copyThemeToClipboard(theme: QbitTheme): Promise<void> {
  const json = JSON.stringify(theme, null, 2);
  await navigator.clipboard.writeText(json);
}

/**
 * Import theme from clipboard
 */
export async function importThemeFromClipboard(): Promise<QbitTheme> {
  const text = await navigator.clipboard.readText();
  return importThemeFromString(text);
}

/**
 * Generate a shareable theme URL (base64 encoded)
 * Note: This creates long URLs - for production, use a URL shortener or server-side storage
 */
export function generateThemeShareUrl(theme: QbitTheme, baseUrl: string = window.location.origin): string {
  const json = JSON.stringify(theme);
  const encoded = btoa(encodeURIComponent(json));
  return `${baseUrl}/theme?data=${encoded}`;
}

/**
 * Parse a theme from a share URL
 */
export function parseThemeFromShareUrl(url: string): QbitTheme {
  const urlObj = new URL(url);
  const encoded = urlObj.searchParams.get("data");
  
  if (!encoded) {
    throw new Error("No theme data found in URL");
  }
  
  try {
    const json = decodeURIComponent(atob(encoded));
    return importThemeFromString(json);
  } catch (e) {
    throw new Error(`Failed to parse theme from URL: ${(e as Error).message}`);
  }
}

/**
 * Theme import result with metadata
 */
export interface ThemeImportResult {
  theme: QbitTheme;
  source: "file" | "clipboard" | "url" | "string";
  filename?: string;
}

/**
 * Smart import that tries multiple methods
 */
export async function smartImportTheme(input: string | File): Promise<ThemeImportResult> {
  // File input
  if (input instanceof File) {
    const theme = await importThemeFromFileHandle(input);
    return { theme, source: "file", filename: input.name };
  }
  
  // URL input
  if (input.startsWith("http://") || input.startsWith("https://")) {
    if (input.includes("/theme?data=")) {
      // Share URL
      const theme = parseThemeFromShareUrl(input);
      return { theme, source: "url" };
    }
    
    // Fetch from URL
    const response = await fetch(input);
    if (!response.ok) {
      throw new Error(`Failed to fetch theme: ${response.statusText}`);
    }
    const json = await response.text();
    const theme = importThemeFromString(json);
    return { theme, source: "url" };
  }
  
  // JSON string
  const theme = importThemeFromString(input);
  return { theme, source: "string" };
}
