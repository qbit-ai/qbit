/**
 * FileIndex data structure for O(1) file path lookups
 * Built from the indexer's file list for validating detected paths
 */

export interface FileIndex {
  /** All absolute paths for O(1) existence check */
  absolutePaths: Set<string>;
  /** Map from filename → absolute paths (handles duplicates) */
  byFilename: Map<string, string[]>;
  /** Map from relative path → absolute path */
  byRelativePath: Map<string, string>;
  /** Workspace root for relative path calculations */
  workspaceRoot: string;
}

/**
 * Build a FileIndex from a list of absolute file paths
 * @param absolutePaths - Array of absolute file paths from the indexer
 * @param workspaceRoot - The workspace root directory
 * @returns A FileIndex for fast lookups
 */
export function buildFileIndex(absolutePaths: string[], workspaceRoot: string): FileIndex {
  const index: FileIndex = {
    absolutePaths: new Set(absolutePaths),
    byFilename: new Map(),
    byRelativePath: new Map(),
    workspaceRoot,
  };

  // Normalize workspace root - ensure no trailing slash
  const normalizedRoot = workspaceRoot.endsWith("/") ? workspaceRoot.slice(0, -1) : workspaceRoot;

  for (const absPath of absolutePaths) {
    // Extract filename (last segment)
    const lastSlash = absPath.lastIndexOf("/");
    const filename = lastSlash >= 0 ? absPath.slice(lastSlash + 1) : absPath;

    // Add to filename map (may have multiple files with same name)
    const existing = index.byFilename.get(filename);
    if (existing) {
      existing.push(absPath);
    } else {
      index.byFilename.set(filename, [absPath]);
    }

    // Calculate relative path
    const relativePath = absPath.startsWith(`${normalizedRoot}/`)
      ? absPath.slice(normalizedRoot.length + 1)
      : absPath; // File outside workspace - use absolute path as key

    index.byRelativePath.set(relativePath, absPath);
  }

  return index;
}
