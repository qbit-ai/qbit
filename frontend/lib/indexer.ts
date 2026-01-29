import { invoke } from "@tauri-apps/api/core";

// Types matching Rust structs

/** Result of indexing a file or directory */
export interface IndexResult {
  files_indexed: number;
  success: boolean;
  message: string;
}

// =============================================================================
// Deduplication for concurrent initialization calls
// =============================================================================

/** Tracks ongoing initialization promises to prevent thundering herd */
let initPromise: Promise<IndexResult> | null = null;
let initWorkspace: string | null = null;

/** Tracks ongoing indexDirectory promises per path */
const indexingPromises = new Map<string, Promise<IndexResult>>();

/** Search result from the indexer */
export interface IndexSearchResult {
  file_path: string;
  line_number: number;
  line_content: string;
  matches: string[];
}

/** Symbol information from tree-sitter analysis */
export interface SymbolResult {
  name: string;
  kind: string;
  line: number;
  column: number;
  scope: string | null;
  signature: string | null;
  documentation: string | null;
}

/** Code analysis result */
export interface AnalysisResult {
  symbols: SymbolResult[];
  metrics: MetricsResult | null;
  dependencies: DependencyResult[];
}

/** Code metrics */
export interface MetricsResult {
  lines_of_code: number;
  lines_of_comments: number;
  blank_lines: number;
  functions_count: number;
  classes_count: number;
  variables_count: number;
  imports_count: number;
  comment_ratio: number;
}

/** Dependency information */
export interface DependencyResult {
  name: string;
  kind: string;
  source: string | null;
}

// Indexer Commands

/**
 * Initialize the code indexer for a workspace.
 * Concurrent calls for the same workspace will share the same promise.
 */
export async function initIndexer(workspacePath: string): Promise<IndexResult> {
  // If there's an ongoing init for the same workspace, reuse it
  if (initPromise && initWorkspace === workspacePath) {
    return initPromise;
  }

  // If there's an ongoing init for a different workspace, wait for it first
  if (initPromise && initWorkspace !== workspacePath) {
    try {
      await initPromise;
    } catch {
      // Ignore errors from previous init
    }
  }

  // Start new initialization
  initWorkspace = workspacePath;
  initPromise = invoke<IndexResult>("init_indexer", { workspacePath }).finally(() => {
    // Clear promise when done (but only if this is still the active init)
    if (initWorkspace === workspacePath) {
      initPromise = null;
      initWorkspace = null;
    }
  });

  return initPromise;
}

/**
 * Check if the indexer is initialized
 */
export async function isIndexerInitialized(): Promise<boolean> {
  return invoke("is_indexer_initialized");
}

/**
 * Get the current workspace root
 */
export async function getIndexerWorkspace(): Promise<string | null> {
  return invoke("get_indexer_workspace");
}

/**
 * Get the count of indexed files
 */
export async function getIndexedFileCount(): Promise<number> {
  return invoke("get_indexed_file_count");
}

/**
 * Get all indexed file paths as absolute paths
 */
export async function getAllIndexedFiles(): Promise<string[]> {
  return invoke("get_all_indexed_files");
}

/**
 * Index a specific file
 */
export async function indexFile(filePath: string): Promise<IndexResult> {
  return invoke("index_file", { filePath });
}

/**
 * Index a directory recursively.
 * Concurrent calls for the same directory will share the same promise.
 */
export async function indexDirectory(dirPath: string): Promise<IndexResult> {
  // If there's an ongoing indexing for this path, reuse it
  const existing = indexingPromises.get(dirPath);
  if (existing) {
    return existing;
  }

  // Start new indexing
  const promise = invoke<IndexResult>("index_directory", { dirPath }).finally(() => {
    // Clean up when done
    indexingPromises.delete(dirPath);
  });

  indexingPromises.set(dirPath, promise);
  return promise;
}

/**
 * Search for content in indexed files
 * @param pattern - Search pattern (regex)
 * @param pathFilter - Optional file path filter
 */
export async function searchCode(
  pattern: string,
  pathFilter?: string
): Promise<IndexSearchResult[]> {
  return invoke("search_code", { pattern, pathFilter });
}

/**
 * Search for files by name pattern
 */
export async function searchFiles(pattern: string): Promise<string[]> {
  return invoke("search_files", { pattern });
}

/**
 * Analyze a file using tree-sitter
 * Returns symbols, metrics, and dependencies
 */
export async function analyzeFile(filePath: string): Promise<AnalysisResult> {
  return invoke("analyze_file", { filePath });
}

/** Alias for SymbolResult */
export type SymbolInfo = SymbolResult;

/**
 * Extract symbols from a file
 */
export async function extractSymbols(filePath: string): Promise<SymbolResult[]> {
  return invoke("extract_symbols", { filePath });
}

/**
 * Get code metrics for a file
 */
export async function getFileMetrics(filePath: string): Promise<MetricsResult> {
  return invoke("get_file_metrics", { filePath });
}

/**
 * Detect the language of a file
 */
export async function detectLanguage(filePath: string): Promise<string> {
  return invoke("detect_language", { filePath });
}

/**
 * Shutdown the indexer
 */
export async function shutdownIndexer(): Promise<void> {
  return invoke("shutdown_indexer");
}

// =============================================================================
// Codebase Management
// =============================================================================

/** Information about an indexed codebase */
export interface CodebaseInfo {
  /** The path to the codebase */
  path: string;
  /** Number of indexed files (0 if not yet indexed) */
  file_count: number;
  /** Current status: "synced", "indexing", "not_indexed", or "error" */
  status: "synced" | "indexing" | "not_indexed" | "error";
  /** Error message if status is "error" */
  error?: string;
  /** Memory file associated with this codebase: "AGENTS.md", "CLAUDE.md", or undefined */
  memory_file?: string;
}

/**
 * List all indexed codebases from settings
 */
export async function listIndexedCodebases(): Promise<CodebaseInfo[]> {
  return invoke("list_indexed_codebases");
}

/**
 * Add a new codebase to the indexed list and start indexing
 */
export async function addIndexedCodebase(path: string): Promise<CodebaseInfo> {
  return invoke("add_indexed_codebase", { path });
}

/**
 * Remove a codebase from the indexed list and delete its index files
 */
export async function removeIndexedCodebase(path: string): Promise<void> {
  return invoke("remove_indexed_codebase", { path });
}

/**
 * Re-index a codebase (clear and rebuild the index)
 */
export async function reindexCodebase(path: string): Promise<CodebaseInfo> {
  return invoke("reindex_codebase", { path });
}

/**
 * Update the memory file setting for a codebase
 * @param path - The codebase path
 * @param memoryFile - The memory file name ("AGENTS.md", "CLAUDE.md") or null for none
 */
export async function updateCodebaseMemoryFile(
  path: string,
  memoryFile: string | null
): Promise<void> {
  return invoke("update_codebase_memory_file", { path, memoryFile });
}

/**
 * Detect memory files at the root of a codebase
 * Returns the detected memory file based on priority: AGENTS.md > CLAUDE.md > null
 */
export async function detectMemoryFiles(path: string): Promise<string | null> {
  return invoke("detect_memory_files", { path });
}

/**
 * Migrate a codebase's index to the configured storage location
 * @param path - The codebase path
 * @returns The new index path if migrated, null if no migration was needed
 */
export async function migrateCodebaseIndex(path: string): Promise<string | null> {
  return invoke("migrate_codebase_index", { path });
}

// =============================================================================
// Home View
// =============================================================================

/** Git branch information for a project */
export interface BranchInfo {
  /** Branch name (e.g., "main", "feature/new-components") */
  name: string;
  /** Full path to the worktree/checkout */
  path: string;
  /** Number of files with changes */
  file_count: number;
  /** Lines added */
  insertions: number;
  /** Lines deleted */
  deletions: number;
  /** Last activity time (relative, e.g., "2h ago") */
  last_activity: string;
}

/** Project information for the home view */
export interface ProjectInfo {
  /** Path to the project root */
  path: string;
  /** Project name (directory name) */
  name: string;
  /** Git branches with their stats */
  branches: BranchInfo[];
  /** Number of warnings/errors */
  warnings: number;
  /** Last activity time (relative, e.g., "2h ago") */
  last_activity: string;
}

/** Recent directory information for the home view */
export interface RecentDirectory {
  /** Full path to the directory */
  path: string;
  /** Directory name */
  name: string;
  /** Current git branch (if in a git repo) */
  branch: string | null;
  /** Number of files with changes */
  file_count: number;
  /** Lines added */
  insertions: number;
  /** Lines deleted */
  deletions: number;
  /** Last accessed time (relative, e.g., "2h ago") */
  last_accessed: string;
}

/**
 * List projects for the home view
 * Returns configured codebases with git branch information
 */
export async function listProjectsForHome(): Promise<ProjectInfo[]> {
  return invoke("list_projects_for_home");
}

/**
 * List recent directories from AI session history
 * @param limit - Maximum number of directories to return (default: 20)
 */
export async function listRecentDirectories(limit?: number): Promise<RecentDirectory[]> {
  return invoke("list_recent_directories", { limit });
}
