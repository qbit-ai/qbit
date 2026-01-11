/**
 * File path detection utilities
 * Detects file paths in text content using regex patterns
 */

import type { FileIndex } from "./fileIndex";

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
  type: "absolute" | "relative" | "filename" | "directory";
}

// Common file extensions to match
const FILE_EXTENSIONS =
  "(?:ts|tsx|js|jsx|mjs|cjs|json|md|mdx|py|rs|go|java|c|cpp|h|hpp|css|scss|html|xml|yaml|yml|toml|sh|bash|zsh|sql|rb|php|swift|kt|scala|vim|lua|zig|hs|ex|exs|erl|clj|vue|svelte|astro|png|jpg|jpeg|gif|svg|ico|webp|lock|gitignore|env|txt)";

/**
 * Main regex pattern for detecting file paths WITH extensions.
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
    // Just a filename: name.ext (must start with letter or dot for hidden files)
    `\\.?[a-zA-Z_][\\w.-]*` +
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

/**
 * Regex pattern for detecting directory paths (without extensions).
 * Matches:
 * - Paths with slashes: src/components, ./lib, ../utils
 * - Directory names at end of ls -l lines (after timestamp pattern)
 */
const DIRECTORY_PATH_REGEX = new RegExp(
  // Start boundary
  `(?:^|[\\s"'\`({\\[])` +
    // Capture group
    `(` +
    `(?:` +
    // Relative path with ./
    `\\.{1,2}/[\\w.-]+(?:/[\\w.-]+)*` +
    `|` +
    // Path with multiple segments (like src/components)
    `[a-zA-Z_][\\w-]*(?:/[\\w.-]+)+` +
    `)` +
    `)` +
    // End boundary - must NOT be followed by a file extension
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

// Pattern to detect ls -l output lines and extract the filename/dirname at the end
// Matches: drwxr-xr-x@ 10 xlyk staff 320 Jan 8 11:27 backend
const LS_LINE_REGEX = /^[d-][rwx-]{9}[@+]?\s+\d+\s+\w+\s+\w+\s+[\d,]+\s+\w+\s+\d+\s+[\d:]+\s+(.+)$/;

/**
 * Detect file paths in text content
 * @param text - Text to scan for file paths
 * @returns Array of detected paths with their positions
 */
export function detectFilePaths(text: string): DetectedPath[] {
  const results: DetectedPath[] = [];
  const seenRanges = new Set<string>();

  // Helper to add result if not already covered
  const addResult = (result: DetectedPath) => {
    const key = `${result.start}-${result.end}`;
    if (!seenRanges.has(key)) {
      seenRanges.add(key);
      results.push(result);
    }
  };

  // 1. Detect files with extensions using main regex
  FILE_PATH_REGEX.lastIndex = 0;
  let match: RegExpExecArray | null = FILE_PATH_REGEX.exec(text);
  while (match !== null) {
    const fullMatch = match[1];
    const lineNum = match[2];
    const colNum = match[3];

    const boundaryOffset = match[0].indexOf(fullMatch);
    const start = match.index + boundaryOffset;
    const end = start + fullMatch.length;

    let path = fullMatch;
    if (lineNum) {
      path = fullMatch.replace(/:(\d+)(?::(\d+))?$/, "");
    }

    if (!EXCLUDE_PATTERNS.some((pattern) => pattern.test(path))) {
      let type: DetectedPath["type"];
      if (path.startsWith("/")) {
        type = "absolute";
      } else if (path.includes("/")) {
        type = "relative";
      } else {
        type = "filename";
      }

      addResult({
        raw: fullMatch,
        path,
        line: lineNum ? Number.parseInt(lineNum, 10) : undefined,
        column: colNum ? Number.parseInt(colNum, 10) : undefined,
        start,
        end,
        type,
      });
    }

    match = FILE_PATH_REGEX.exec(text);
  }

  // 2. Detect directory paths (with slashes but no extension)
  DIRECTORY_PATH_REGEX.lastIndex = 0;
  match = DIRECTORY_PATH_REGEX.exec(text);
  while (match !== null) {
    const fullMatch = match[1];
    const boundaryOffset = match[0].indexOf(fullMatch);
    const start = match.index + boundaryOffset;
    const end = start + fullMatch.length;

    // Skip if this looks like a file with extension (already caught above)
    if (!EXCLUDE_PATTERNS.some((pattern) => pattern.test(fullMatch))) {
      addResult({
        raw: fullMatch,
        path: fullMatch,
        start,
        end,
        type: "directory",
      });
    }

    match = DIRECTORY_PATH_REGEX.exec(text);
  }

  // 3. Detect filenames/dirnames from ls -l output lines
  const lines = text.split("\n");
  let lineStart = 0;
  for (const line of lines) {
    const lsMatch = LS_LINE_REGEX.exec(line);
    if (lsMatch) {
      const name = lsMatch[1];
      // Find the position of the name in the line
      const nameStart = lineStart + line.lastIndexOf(name);
      const nameEnd = nameStart + name.length;

      // Check if already detected (as a file with extension)
      const key = `${nameStart}-${nameEnd}`;
      if (!seenRanges.has(key)) {
        // Determine if it's a directory (line starts with 'd')
        const isDir = line.startsWith("d");
        addResult({
          raw: name,
          path: name,
          start: nameStart,
          end: nameEnd,
          type: isDir ? "directory" : "filename",
        });
      }
    }
    lineStart += line.length + 1; // +1 for newline
  }

  // Sort by start position
  results.sort((a, b) => a.start - b.start);

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

// =============================================================================
// Index-Aware Path Detection
// =============================================================================

export interface DetectedPathWithResolution extends DetectedPath {
  /** Resolved absolute path (if found in index) */
  absolutePath?: string;
  /** Whether this path was validated against the file index */
  validated: boolean;
}

/**
 * Normalize a relative path for lookup in the file index.
 * - Removes leading ./
 * - Handles ../ by stripping the prefix (simplified approach)
 */
function normalizeRelativePath(path: string): string {
  // Remove leading ./
  if (path.startsWith("./")) {
    return path.slice(2);
  }
  // For ../ paths, strip the prefix (will need context for proper resolution)
  // This is a simplified approach - full resolution would need working directory
  if (path.startsWith("../")) {
    // Strip all leading ../ segments for basic lookup
    return path.replace(/^(\.\.\/)+/, "");
  }
  return path;
}

/**
 * Detect file paths using file index for validation.
 * Only returns paths that exist in the index.
 *
 * @param text - Text to scan for file paths
 * @param fileIndex - Pre-built file index for validation
 * @returns Array of detected paths that exist in the index
 */
export function detectFilePathsWithIndex(
  text: string,
  fileIndex: FileIndex
): DetectedPathWithResolution[] {
  if (!text) return [];

  // 1. Run existing regex detection to get candidates
  const candidates = detectFilePaths(text);
  const results: DetectedPathWithResolution[] = [];
  const seenRanges = new Set<string>();

  for (const candidate of candidates) {
    const key = `${candidate.start}-${candidate.end}`;

    // Skip if exact range already seen
    if (seenRanges.has(key)) continue;

    // Skip if this range overlaps with any already-validated path
    // This prevents matching "src/main.ts" separately when "src/main.ts:42" was already detected
    if (overlapsWithSeen(candidate.start, candidate.end, seenRanges)) continue;

    if (candidate.type === "absolute") {
      if (fileIndex.absolutePaths.has(candidate.path)) {
        seenRanges.add(key);
        results.push({
          ...candidate,
          absolutePath: candidate.path,
          validated: true,
        });
      }
    } else if (candidate.type === "relative" || candidate.type === "directory") {
      // Normalize the path before lookup (handles ./ and ../ prefixes)
      const normalizedPath = normalizeRelativePath(candidate.path);
      const absolutePath = fileIndex.byRelativePath.get(normalizedPath);
      if (absolutePath) {
        seenRanges.add(key);
        results.push({
          ...candidate,
          absolutePath,
          validated: true,
        });
      }
    } else if (candidate.type === "filename") {
      const matches = fileIndex.byFilename.get(candidate.path);
      if (matches && matches.length > 0) {
        seenRanges.add(key);
        results.push({
          ...candidate,
          absolutePath: matches.length === 1 ? matches[0] : undefined,
          validated: true,
        });
      }
    }
  }

  // 2. Scan for bare words that match filenames in index (regex may have missed)
  const bareMatches = findBareFilenameMatches(text, fileIndex, seenRanges);
  results.push(...bareMatches);

  // Sort by position
  results.sort((a, b) => a.start - b.start);

  return results;
}

/**
 * Check if a range overlaps with any already-seen range
 */
function overlapsWithSeen(start: number, end: number, seenRanges: Set<string>): boolean {
  for (const key of seenRanges) {
    const [seenStart, seenEnd] = key.split("-").map(Number);
    // Overlap if ranges intersect
    if (start < seenEnd && end > seenStart) {
      return true;
    }
  }
  return false;
}

/**
 * Find bare filenames (words) in text that match files in the index.
 * This catches extensionless files like Makefile, Dockerfile, .gitignore
 * that the regex-based detection might miss.
 */
function findBareFilenameMatches(
  text: string,
  fileIndex: FileIndex,
  seenRanges: Set<string>
): DetectedPathWithResolution[] {
  const results: DetectedPathWithResolution[] = [];

  // Match word-like tokens that could be filenames
  // Includes dotfiles (.gitignore) and alphanumeric names with dots/dashes
  const wordPattern = /(?:^|[\s"'`({[])(\.?[\w][\w.-]*)(?=$|[\s"'`)\]:,;])/g;
  // biome-ignore lint/suspicious/noAssignInExpressions: standard regex exec pattern
  for (let match: RegExpExecArray | null; (match = wordPattern.exec(text)) !== null; ) {
    const word = match[1];
    const boundaryOffset = match[0].indexOf(word);
    const start = match.index + boundaryOffset;
    const end = start + word.length;
    const key = `${start}-${end}`;

    // Skip if exact range already seen
    if (seenRanges.has(key)) continue;

    // Skip if this range overlaps with any already-detected path
    // This prevents matching "main.ts" separately when "src/main.ts:42" was already detected
    if (overlapsWithSeen(start, end, seenRanges)) continue;

    const matches = fileIndex.byFilename.get(word);
    if (matches && matches.length > 0) {
      seenRanges.add(key);
      results.push({
        raw: word,
        path: word,
        start,
        end,
        type: "filename",
        absolutePath: matches.length === 1 ? matches[0] : undefined,
        validated: true,
      });
    }
  }

  return results;
}
