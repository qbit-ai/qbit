import { invoke } from "@tauri-apps/api/core";

// Types matching Rust structs

/** Result of indexing a file or directory */
export interface IndexResult {
  files_indexed: number;
  success: boolean;
  message: string;
}

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
 * Initialize the code indexer for a workspace
 */
export async function initIndexer(workspacePath: string): Promise<IndexResult> {
  return invoke("init_indexer", { workspacePath });
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
 * Index a specific file
 */
export async function indexFile(filePath: string): Promise<IndexResult> {
  return invoke("index_file", { filePath });
}

/**
 * Index a directory recursively
 */
export async function indexDirectory(dirPath: string): Promise<IndexResult> {
  return invoke("index_directory", { dirPath });
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
