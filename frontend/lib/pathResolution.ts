/**
 * File path resolution utilities
 * Resolves detected paths to absolute paths using workspace context
 */

import { searchFiles } from "./indexer";
import type { DetectedPath } from "./pathDetection";

export interface ResolvedPath {
  /** Absolute path to the file */
  absolutePath: string;
  /** Path relative to workspace (for display) */
  relativePath: string;
  /** Line number to navigate to */
  line?: number;
  /** Column number to navigate to */
  column?: number;
}

/**
 * Normalize path separators and resolve . and .. segments
 */
function normalizePath(path: string): string {
  // Replace backslashes with forward slashes
  let normalized = path.replace(/\\/g, "/");

  // Remove duplicate slashes
  normalized = normalized.replace(/\/+/g, "/");

  // Resolve . and .. segments
  const segments = normalized.split("/");
  const result: string[] = [];

  for (const segment of segments) {
    if (segment === "..") {
      result.pop();
    } else if (segment !== "." && segment !== "") {
      result.push(segment);
    }
  }

  // Preserve leading slash for absolute paths
  const prefix = normalized.startsWith("/") ? "/" : "";
  return prefix + result.join("/");
}

/**
 * Join path segments, handling absolute vs relative paths
 */
function joinPaths(base: string, relative: string): string {
  if (relative.startsWith("/")) {
    return normalizePath(relative);
  }

  // Remove trailing slash from base
  const cleanBase = base.endsWith("/") ? base.slice(0, -1) : base;

  // Remove leading ./ from relative
  const cleanRelative = relative.startsWith("./") ? relative.slice(2) : relative;

  return normalizePath(`${cleanBase}/${cleanRelative}`);
}

/**
 * Get relative path from workspace root
 */
function getRelativePath(absolutePath: string, workingDirectory: string): string {
  const normalizedAbs = normalizePath(absolutePath);
  const normalizedWd = normalizePath(workingDirectory);

  if (normalizedAbs.startsWith(`${normalizedWd}/`)) {
    return normalizedAbs.slice(normalizedWd.length + 1);
  }

  return normalizedAbs;
}

/**
 * Resolve a detected path to absolute path(s)
 *
 * @param detected - The detected path info
 * @param workingDirectory - Current working directory for resolution
 * @returns Array of resolved paths (may be multiple for bare filenames)
 */
export async function resolvePath(
  detected: DetectedPath,
  workingDirectory: string
): Promise<ResolvedPath[]> {
  const { path, line, column, type } = detected;

  if (type === "absolute") {
    // Absolute path - use directly
    return [
      {
        absolutePath: normalizePath(path),
        relativePath: getRelativePath(path, workingDirectory),
        line,
        column,
      },
    ];
  }

  if (type === "relative") {
    // Relative path - resolve against working directory
    const absolutePath = joinPaths(workingDirectory, path);
    return [
      {
        absolutePath,
        relativePath: getRelativePath(absolutePath, workingDirectory),
        line,
        column,
      },
    ];
  }

  // Filename only - search for matches
  const matches = await findFilesByName(path);

  if (matches.length === 0) {
    // No matches found - return as relative path anyway
    const absolutePath = joinPaths(workingDirectory, path);
    return [
      {
        absolutePath,
        relativePath: path,
        line,
        column,
      },
    ];
  }

  return matches.map((absPath) => ({
    absolutePath: absPath,
    relativePath: getRelativePath(absPath, workingDirectory),
    line,
    column,
  }));
}

/**
 * Search for files by name in the workspace
 * @param filename - Filename to search for (e.g., "main.rs")
 * @returns Array of absolute paths matching the filename
 */
export async function findFilesByName(filename: string): Promise<string[]> {
  try {
    // Use glob pattern to search for the filename anywhere in workspace
    const pattern = `**/${filename}`;
    const results = await searchFiles(pattern);
    return results;
  } catch (error) {
    console.error("Failed to search for files:", error);
    return [];
  }
}
