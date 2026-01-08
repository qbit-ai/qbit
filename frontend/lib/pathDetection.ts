/**
 * File path detection utilities
 * Detects file paths in text content using regex patterns
 */

export interface DetectedPath {
  /** Original matched text */
  raw: string;
  /** Extracted path (without line numbers) */
  path: string;
  /** Line number if present (e.g., file.ts:42) */
  line?: number;
  /** Column if present (e.g., file.ts:42:10) */
  column?: number;
  /** Start index in source text */
  start: number;
  /** End index in source text */
  end: number;
  /** Type of path detected */
  type: "absolute" | "relative" | "filename";
}

// Common file extensions to match
const FILE_EXTENSIONS =
  "(?:ts|tsx|js|jsx|mjs|cjs|json|md|mdx|py|rs|go|java|c|cpp|h|hpp|css|scss|html|xml|yaml|yml|toml|sh|bash|zsh|sql|rb|php|swift|kt|scala|vim|lua|zig|hs|ex|exs|erl|clj|vue|svelte|astro)";

/**
 * Main regex pattern for detecting file paths.
 * Captures:
 * - Absolute paths: /Users/foo/bar.ts
 * - Relative paths: ./src/file.ts, ../lib/util.ts, src/file.ts
 * - Filenames: main.rs
 * - With optional line:column suffix: file.ts:42, file.ts:42:10
 */
const FILE_PATH_REGEX = new RegExp(
  // Start boundary: beginning of string or common delimiters
  `(?:^|[\\s"'\`({\\[])` +
    // Capture group for the full match (path + optional line:col)
    `(` +
    // Path part: either absolute (/...), relative (./... or ../... or name/...), or just filename
    `(?:` +
    // Absolute path starting with /
    `/[\\w./-]+` +
    `|` +
    // Relative path: starts with ./ or ../ or has / in it
    `\\.{1,2}/[\\w./-]+` +
    `|` +
    // Path with directory (no leading dot): dir/file.ext
    `[a-zA-Z_][\\w-]*(?:/[\\w.-]+)+` +
    `|` +
    // Just a filename: name.ext (must start with letter)
    `[a-zA-Z_][\\w.-]*` +
    `)` +
    // Must end with known file extension
    `\\.${FILE_EXTENSIONS}` +
    // Optional line number suffix :42 or :42:10
    `(?::(\\d+)(?::(\\d+))?)?` +
    `)` +
    // End boundary: end of string or common delimiters
    `(?=$|[\\s"'\`)}\\]:,;])`,
  "gi"
);

// Patterns to exclude (URLs, emails, etc.)
const EXCLUDE_PATTERNS = [
  /^https?:\/\//i, // URLs
  /^ftp:\/\//i, // FTP
  /^file:\/\//i, // File URLs
  /@[a-z]+\.[a-z]/i, // Email-like patterns
  /^\d+\.\d+\.\d+/, // Version numbers like 1.2.3
];

/**
 * Detect file paths in text content
 * @param text - Text to scan for file paths
 * @returns Array of detected paths with their positions
 */
export function detectFilePaths(text: string): DetectedPath[] {
  const results: DetectedPath[] = [];

  // Reset regex state
  FILE_PATH_REGEX.lastIndex = 0;

  let match: RegExpExecArray | null = FILE_PATH_REGEX.exec(text);
  while (match !== null) {
    const fullMatch = match[1]; // The captured path (without boundary chars)
    const lineNum = match[2];
    const colNum = match[3];

    // Calculate actual start position (match.index includes boundary char)
    const boundaryOffset = match[0].indexOf(fullMatch);
    const start = match.index + boundaryOffset;
    const end = start + fullMatch.length;

    // Extract path without line:col suffix
    let path = fullMatch;
    if (lineNum) {
      path = fullMatch.replace(/:(\d+)(?::(\d+))?$/, "");
    }

    // Skip excluded patterns
    if (EXCLUDE_PATTERNS.some((pattern) => pattern.test(path))) {
      match = FILE_PATH_REGEX.exec(text);
      continue;
    }

    // Determine path type
    let type: DetectedPath["type"];
    if (path.startsWith("/")) {
      type = "absolute";
    } else if (path.includes("/")) {
      type = "relative";
    } else {
      type = "filename";
    }

    results.push({
      raw: fullMatch,
      path,
      line: lineNum ? Number.parseInt(lineNum, 10) : undefined,
      column: colNum ? Number.parseInt(colNum, 10) : undefined,
      start,
      end,
      type,
    });

    match = FILE_PATH_REGEX.exec(text);
  }

  return results;
}

/**
 * Check if a string looks like a file path
 * @param text - Text to check
 * @returns true if the text appears to be a file path
 */
export function isLikelyFilePath(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return false;

  // Quick check: must have a file extension
  const extensionMatch = new RegExp(`\\.${FILE_EXTENSIONS}(?::\\d+(?::\\d+)?)?$`, "i");
  if (!extensionMatch.test(trimmed)) return false;

  // Exclude URLs and emails
  if (EXCLUDE_PATTERNS.some((pattern) => pattern.test(trimmed))) return false;

  return true;
}
