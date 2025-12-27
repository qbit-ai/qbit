//! Tauri commands for code indexer operations

use crate::settings::schema::IndexLocation;
use crate::state::AppState;
use qbit_indexer::paths::{compute_index_dir, find_existing_index_dir, migrate_index};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use vtcode_core::tools::tree_sitter::{
    analysis::{CodeMetrics, DependencyInfo},
    languages::SymbolInfo,
};

/// Result of indexing a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub files_indexed: usize,
    pub success: bool,
    pub message: String,
}

/// Search result from the indexer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexSearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub matches: Vec<String>,
}

/// Symbol information for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolResult {
    pub name: String,
    pub kind: String,
    pub line: usize,
    pub column: usize,
    pub scope: Option<String>,
    pub signature: Option<String>,
    pub documentation: Option<String>,
}

impl From<SymbolInfo> for SymbolResult {
    fn from(info: SymbolInfo) -> Self {
        Self {
            name: info.name,
            kind: format!("{:?}", info.kind),
            line: info.position.row,
            column: info.position.column,
            scope: info.scope,
            signature: info.signature,
            documentation: info.documentation,
        }
    }
}

/// Code analysis result for frontend consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub symbols: Vec<SymbolResult>,
    pub metrics: Option<MetricsResult>,
    pub dependencies: Vec<DependencyResult>,
}

/// Code metrics result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResult {
    pub lines_of_code: usize,
    pub lines_of_comments: usize,
    pub blank_lines: usize,
    pub functions_count: usize,
    pub classes_count: usize,
    pub variables_count: usize,
    pub imports_count: usize,
    pub comment_ratio: f64,
}

impl From<CodeMetrics> for MetricsResult {
    fn from(metrics: CodeMetrics) -> Self {
        Self {
            lines_of_code: metrics.lines_of_code,
            lines_of_comments: metrics.lines_of_comments,
            blank_lines: metrics.blank_lines,
            functions_count: metrics.functions_count,
            classes_count: metrics.classes_count,
            variables_count: metrics.variables_count,
            imports_count: metrics.imports_count,
            comment_ratio: metrics.comment_ratio,
        }
    }
}

/// Dependency information result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyResult {
    pub name: String,
    pub kind: String,
    pub source: Option<String>,
}

impl From<DependencyInfo> for DependencyResult {
    fn from(dep: DependencyInfo) -> Self {
        Self {
            name: dep.name,
            kind: format!("{:?}", dep.kind),
            source: Some(dep.source),
        }
    }
}

/// Initialize the code indexer for a workspace
#[tauri::command]
pub async fn init_indexer(
    workspace_path: String,
    state: State<'_, AppState>,
) -> Result<IndexResult, String> {
    tracing::info!("init_indexer called with workspace: {}", workspace_path);

    let path = PathBuf::from(&workspace_path);

    if !path.exists() {
        tracing::error!("Workspace path does not exist: {}", workspace_path);
        return Err(format!("Workspace path does not exist: {}", workspace_path));
    }

    // Get index location from settings
    let settings = state.settings_manager.get().await;
    let index_location = settings.indexer.index_location;

    tracing::debug!(
        "Workspace path exists, initializing indexer state with location: {:?}",
        index_location
    );

    state
        .indexer_state
        .initialize_with_location(path, index_location)
        .map_err(|e| {
            tracing::error!("Failed to initialize indexer: {}", e);
            e.to_string()
        })?;

    tracing::info!(
        "init_indexer completed successfully for: {}",
        workspace_path
    );

    Ok(IndexResult {
        files_indexed: 0,
        success: true,
        message: format!("Indexer initialized for workspace: {}", workspace_path),
    })
}

/// Check if the indexer is initialized
#[tauri::command]
pub fn is_indexer_initialized(state: State<'_, AppState>) -> bool {
    state.indexer_state.is_initialized()
}

/// Get the current workspace root
#[tauri::command]
pub fn get_indexer_workspace(state: State<'_, AppState>) -> Option<String> {
    state
        .indexer_state
        .workspace_root()
        .map(|p| p.to_string_lossy().to_string())
}

/// Get the count of indexed files
#[tauri::command]
pub fn get_indexed_file_count(state: State<'_, AppState>) -> Result<usize, String> {
    state
        .indexer_state
        .with_indexer(|indexer| {
            // Use all_files() instead of find_files("*") - more efficient and doesn't require regex
            Ok(indexer.all_files().len())
        })
        .map_err(|e| e.to_string())
}

/// Index a specific file
#[tauri::command]
pub async fn index_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<IndexResult, String> {
    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    state
        .indexer_state
        .with_indexer_mut(|indexer| {
            indexer.index_file(&path)?;
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    Ok(IndexResult {
        files_indexed: 1,
        success: true,
        message: format!("Indexed file: {}", file_path),
    })
}

/// Index a directory recursively
#[tauri::command]
pub async fn index_directory(
    dir_path: String,
    state: State<'_, AppState>,
) -> Result<IndexResult, String> {
    tracing::info!("index_directory called with path: {}", dir_path);

    let path = PathBuf::from(&dir_path);

    if !path.exists() {
        tracing::error!("Directory does not exist: {}", dir_path);
        return Err(format!("Directory does not exist: {}", dir_path));
    }

    tracing::debug!("Directory exists, checking indexer state...");
    tracing::debug!(
        "Indexer initialized: {}",
        state.indexer_state.is_initialized()
    );

    state
        .indexer_state
        .with_indexer_mut(|indexer| {
            tracing::info!("Starting directory indexing for: {:?}", path);
            let start = std::time::Instant::now();

            indexer.index_directory(&path)?;

            tracing::info!("Directory indexing completed in {:?}", start.elapsed(),);
            Ok(())
        })
        .map_err(|e| {
            tracing::error!("Failed to index directory: {}", e);
            e.to_string()
        })?;

    // Get the actual file count after indexing
    let files_indexed = state
        .indexer_state
        .with_indexer(|indexer| {
            let files = indexer.all_files();
            tracing::info!("Total files in index after indexing: {}", files.len());
            Ok(files.len())
        })
        .unwrap_or(0);

    tracing::info!(
        "index_directory completed successfully, {} files now in index",
        files_indexed
    );

    Ok(IndexResult {
        files_indexed,
        success: true,
        message: format!(
            "Indexed directory: {} ({} files in index)",
            dir_path, files_indexed
        ),
    })
}

/// Search for content in indexed files
#[tauri::command]
pub async fn search_code(
    pattern: String,
    path_filter: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<IndexSearchResult>, String> {
    state
        .indexer_state
        .with_indexer(|indexer| {
            let results = indexer.search(&pattern, path_filter.as_deref())?;
            Ok(results
                .into_iter()
                .map(|r| IndexSearchResult {
                    file_path: r.file_path,
                    line_number: r.line_number,
                    line_content: r.line_content,
                    matches: r.matches,
                })
                .collect())
        })
        .map_err(|e| e.to_string())
}

/// Search for files by name pattern
#[tauri::command]
pub async fn search_files(
    pattern: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    state
        .indexer_state
        .with_indexer(|indexer| {
            let results = indexer.find_files(&pattern)?;
            Ok(results)
        })
        .map_err(|e| e.to_string())
}

/// Analyze a file using tree-sitter
#[tauri::command]
pub async fn analyze_file(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<AnalysisResult, String> {
    use vtcode_core::tools::tree_sitter::analysis::CodeAnalyzer;

    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    // Get a fresh analyzer (TreeSitterAnalyzer is not Clone, so we create one per request)
    let mut analyzer = state
        .indexer_state
        .get_analyzer()
        .map_err(|e| e.to_string())?;

    // Parse the file
    let tree = analyzer
        .parse_file(&path)
        .await
        .map_err(|e| format!("Failed to parse file: {}", e))?;

    // Create code analyzer for the detected language and analyze
    let code_analyzer = CodeAnalyzer::new(&tree.language);
    let analysis = code_analyzer.analyze(&tree, &file_path);

    Ok(AnalysisResult {
        symbols: analysis
            .symbols
            .into_iter()
            .map(SymbolResult::from)
            .collect(),
        metrics: Some(MetricsResult::from(analysis.metrics)),
        dependencies: analysis
            .dependencies
            .into_iter()
            .map(DependencyResult::from)
            .collect(),
    })
}

/// Extract symbols from a file
#[tauri::command]
pub async fn extract_symbols(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<Vec<SymbolResult>, String> {
    use vtcode_core::tools::tree_sitter::languages::LanguageAnalyzer;

    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    // Get a fresh analyzer
    let mut analyzer = state
        .indexer_state
        .get_analyzer()
        .map_err(|e| e.to_string())?;

    // Parse the file
    let tree = analyzer
        .parse_file(&path)
        .await
        .map_err(|e| format!("Failed to parse file: {}", e))?;

    // Extract symbols using language analyzer for the detected language
    let lang_analyzer = LanguageAnalyzer::new(&tree.language);
    let symbols = lang_analyzer.extract_symbols(&tree);

    Ok(symbols.into_iter().map(SymbolResult::from).collect())
}

/// Get code metrics for a file
#[tauri::command]
pub async fn get_file_metrics(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<MetricsResult, String> {
    use vtcode_core::tools::tree_sitter::analysis::CodeAnalyzer;

    let path = PathBuf::from(&file_path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    // Get a fresh analyzer
    let mut analyzer = state
        .indexer_state
        .get_analyzer()
        .map_err(|e| e.to_string())?;

    // Parse the file
    let tree = analyzer
        .parse_file(&path)
        .await
        .map_err(|e| format!("Failed to parse file: {}", e))?;

    // Analyze the file and extract metrics
    let code_analyzer = CodeAnalyzer::new(&tree.language);
    let analysis = code_analyzer.analyze(&tree, &file_path);

    Ok(MetricsResult::from(analysis.metrics))
}

/// Detect the language of a file
#[tauri::command]
pub fn detect_language(file_path: String) -> Result<String, String> {
    use vtcode_core::tools::tree_sitter::analyzer::TreeSitterAnalyzer;

    let path = PathBuf::from(&file_path);

    // Create an analyzer to detect language
    let analyzer =
        TreeSitterAnalyzer::new().map_err(|e| format!("Failed to create analyzer: {}", e))?;

    match analyzer.detect_language_from_path(&path) {
        Ok(lang) => Ok(format!("{:?}", lang)),
        Err(e) => Err(format!("Could not detect language: {}", e)),
    }
}

/// Shutdown the indexer
#[tauri::command]
pub fn shutdown_indexer(state: State<'_, AppState>) {
    state.indexer_state.shutdown();
}

// =============================================================================
// Codebase Management Commands
// =============================================================================

/// Information about an indexed codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseInfo {
    /// The path to the codebase
    pub path: String,
    /// Number of indexed files (0 if not yet indexed)
    pub file_count: usize,
    /// Current status: "synced", "indexing", "not_indexed", or "error"
    pub status: String,
    /// Error message if status is "error"
    pub error: Option<String>,
    /// Memory file associated with this codebase: "AGENTS.md", "CLAUDE.md", or None
    pub memory_file: Option<String>,
}

/// Helper to expand ~ to home directory
fn expand_home_dir(path: &str) -> PathBuf {
    if path.starts_with("~/") {
        dirs::home_dir()
            .map(|home| home.join(&path[2..]))
            .unwrap_or_else(|| PathBuf::from(path))
    } else {
        PathBuf::from(path)
    }
}

/// Helper to contract home directory to ~
fn contract_home_dir(path: &std::path::Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            return format!("~/{}", stripped.display());
        }
    }
    path.to_string_lossy().to_string()
}

/// Helper to get file count for a codebase's index directory.
/// Checks both global and local locations for backward compatibility.
fn get_codebase_file_count(path: &std::path::Path) -> usize {
    // Check global location first (new default), then local for backward compatibility
    let index_dir = find_existing_index_dir(path, IndexLocation::Global)
        .unwrap_or_else(|| compute_index_dir(path, IndexLocation::Global));

    if !index_dir.exists() {
        return 0;
    }

    // Count .md files in the index directory
    std::fs::read_dir(&index_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("md"))
                .count()
        })
        .unwrap_or(0)
}

/// List all indexed codebases from settings
#[tauri::command]
pub async fn list_indexed_codebases(
    state: State<'_, AppState>,
) -> Result<Vec<CodebaseInfo>, String> {
    let settings = state.settings_manager.get().await;

    // Use the new codebases field, falling back to indexed_codebases for migration
    let codebases: Vec<CodebaseInfo> = if !settings.codebases.is_empty() {
        // New format: use codebases with memory_file
        settings
            .codebases
            .iter()
            .map(|config| {
                let path = expand_home_dir(&config.path);
                let exists = path.exists();
                let file_count = if exists {
                    get_codebase_file_count(&path)
                } else {
                    0
                };

                let (status, error) = if !exists {
                    ("error".to_string(), Some("Path does not exist".to_string()))
                } else if file_count > 0 {
                    ("synced".to_string(), None)
                } else {
                    ("not_indexed".to_string(), None)
                };

                CodebaseInfo {
                    path: config.path.clone(),
                    file_count,
                    status,
                    error,
                    memory_file: config.memory_file.clone(),
                }
            })
            .collect()
    } else {
        // Legacy format: migrate from indexed_codebases
        settings
            .indexed_codebases
            .iter()
            .map(|path_str| {
                let path = expand_home_dir(path_str);
                let exists = path.exists();
                let file_count = if exists {
                    get_codebase_file_count(&path)
                } else {
                    0
                };

                let (status, error) = if !exists {
                    ("error".to_string(), Some("Path does not exist".to_string()))
                } else if file_count > 0 {
                    ("synced".to_string(), None)
                } else {
                    ("not_indexed".to_string(), None)
                };

                CodebaseInfo {
                    path: path_str.clone(),
                    file_count,
                    status,
                    error,
                    memory_file: None,
                }
            })
            .collect()
    };

    Ok(codebases)
}

/// Add a new codebase to the indexed list and start indexing
#[tauri::command]
pub async fn add_indexed_codebase(
    path: String,
    state: State<'_, AppState>,
) -> Result<CodebaseInfo, String> {
    use crate::settings::schema::CodebaseConfig;

    tracing::info!("add_indexed_codebase called with path: {}", path);

    // Expand and normalize the path
    let expanded_path = expand_home_dir(&path);
    let normalized_path = expanded_path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    if !normalized_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    if !normalized_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    // Convert to display path (with ~)
    let display_path = contract_home_dir(&normalized_path);

    // Check if already in the list (check both old and new format)
    let settings = state.settings_manager.get().await;

    // Check new format
    for existing in &settings.codebases {
        let existing_expanded = expand_home_dir(&existing.path);
        if let Ok(existing_canonical) = existing_expanded.canonicalize() {
            if existing_canonical == normalized_path {
                return Err(format!("Codebase already indexed: {}", display_path));
            }
        }
    }

    // Check legacy format
    for existing in &settings.indexed_codebases {
        let existing_expanded = expand_home_dir(existing);
        if let Ok(existing_canonical) = existing_expanded.canonicalize() {
            if existing_canonical == normalized_path {
                return Err(format!("Codebase already indexed: {}", display_path));
            }
        }
    }

    // Add to settings using new format
    let mut updated_settings = settings.clone();
    updated_settings.codebases.push(CodebaseConfig {
        path: display_path.clone(),
        memory_file: None,
    });

    // Get index location before moving settings
    let index_location = updated_settings.indexer.index_location;

    state
        .settings_manager
        .update(updated_settings)
        .await
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    tracing::info!("Added codebase to settings: {}", display_path);

    // Initialize indexer and index the directory
    state
        .indexer_state
        .initialize_with_location(normalized_path.clone(), index_location)
        .map_err(|e| format!("Failed to initialize indexer: {}", e))?;

    state
        .indexer_state
        .with_indexer_mut(|indexer| {
            indexer.index_directory(&normalized_path)?;
            Ok(())
        })
        .map_err(|e| format!("Failed to index directory: {}", e))?;

    let file_count = get_codebase_file_count(&normalized_path);

    tracing::info!(
        "Indexed codebase {} with {} files",
        display_path,
        file_count
    );

    Ok(CodebaseInfo {
        path: display_path,
        file_count,
        status: "synced".to_string(),
        error: None,
        memory_file: None,
    })
}

/// Remove a codebase from the indexed list and delete its index files
#[tauri::command]
pub async fn remove_indexed_codebase(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::settings::schema::CodebaseConfig;

    tracing::info!("remove_indexed_codebase called with path: {}", path);

    // Expand the path
    let expanded_path = expand_home_dir(&path);

    // Remove from settings (both new and legacy format)
    let settings = state.settings_manager.get().await;

    // Filter new format
    let new_codebases: Vec<CodebaseConfig> = settings
        .codebases
        .iter()
        .filter(|config| {
            let p_expanded = expand_home_dir(&config.path);
            // Compare canonical paths if possible, otherwise compare as-is
            match (p_expanded.canonicalize(), expanded_path.canonicalize()) {
                (Ok(a), Ok(b)) => a != b,
                _ => config.path != path,
            }
        })
        .cloned()
        .collect();

    // Filter legacy format
    let legacy_codebases: Vec<String> = settings
        .indexed_codebases
        .iter()
        .filter(|p| {
            let p_expanded = expand_home_dir(p);
            match (p_expanded.canonicalize(), expanded_path.canonicalize()) {
                (Ok(a), Ok(b)) => a != b,
                _ => *p != &path,
            }
        })
        .cloned()
        .collect();

    let mut updated_settings = settings.clone();
    updated_settings.codebases = new_codebases;
    updated_settings.indexed_codebases = legacy_codebases;
    state
        .settings_manager
        .update(updated_settings)
        .await
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    // Delete the index directory from both possible locations
    // Check global location
    let global_index_dir = compute_index_dir(&expanded_path, IndexLocation::Global);
    if global_index_dir.exists() {
        std::fs::remove_dir_all(&global_index_dir)
            .map_err(|e| format!("Failed to delete global index directory: {}", e))?;
        tracing::info!("Deleted global index directory: {:?}", global_index_dir);
    }

    // Check local location
    let local_index_dir = compute_index_dir(&expanded_path, IndexLocation::Local);
    if local_index_dir.exists() {
        std::fs::remove_dir_all(&local_index_dir)
            .map_err(|e| format!("Failed to delete local index directory: {}", e))?;
        tracing::info!("Deleted local index directory: {:?}", local_index_dir);
    }

    tracing::info!("Removed codebase: {}", path);
    Ok(())
}

/// Re-index a codebase (clear and rebuild the index)
#[tauri::command]
pub async fn reindex_codebase(
    path: String,
    state: State<'_, AppState>,
) -> Result<CodebaseInfo, String> {
    tracing::info!("reindex_codebase called with path: {}", path);

    let expanded_path = expand_home_dir(&path);
    let normalized_path = expanded_path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    if !normalized_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Get existing settings
    let settings = state.settings_manager.get().await;
    let index_location = settings.indexer.index_location;
    let memory_file = settings
        .codebases
        .iter()
        .find(|config| {
            let config_expanded = expand_home_dir(&config.path);
            config_expanded
                .canonicalize()
                .ok()
                .map(|p| p == normalized_path)
                .unwrap_or(false)
        })
        .and_then(|config| config.memory_file.clone());

    // Delete existing index from both possible locations
    let global_index_dir = compute_index_dir(&normalized_path, IndexLocation::Global);
    if global_index_dir.exists() {
        std::fs::remove_dir_all(&global_index_dir)
            .map_err(|e| format!("Failed to delete global index directory: {}", e))?;
        tracing::info!(
            "Deleted existing global index directory: {:?}",
            global_index_dir
        );
    }
    let local_index_dir = compute_index_dir(&normalized_path, IndexLocation::Local);
    if local_index_dir.exists() {
        std::fs::remove_dir_all(&local_index_dir)
            .map_err(|e| format!("Failed to delete local index directory: {}", e))?;
        tracing::info!(
            "Deleted existing local index directory: {:?}",
            local_index_dir
        );
    }

    // Re-initialize and index at the configured location
    state
        .indexer_state
        .initialize_with_location(normalized_path.clone(), index_location)
        .map_err(|e| format!("Failed to initialize indexer: {}", e))?;

    state
        .indexer_state
        .with_indexer_mut(|indexer| {
            indexer.index_directory(&normalized_path)?;
            Ok(())
        })
        .map_err(|e| format!("Failed to index directory: {}", e))?;

    let file_count = get_codebase_file_count(&normalized_path);
    let display_path = contract_home_dir(&normalized_path);

    tracing::info!(
        "Re-indexed codebase {} with {} files",
        display_path,
        file_count
    );

    Ok(CodebaseInfo {
        path: display_path,
        file_count,
        status: "synced".to_string(),
        error: None,
        memory_file,
    })
}

/// Update the memory file setting for a codebase
#[tauri::command]
pub async fn update_codebase_memory_file(
    path: String,
    memory_file: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    use crate::settings::schema::CodebaseConfig;

    tracing::info!(
        "update_codebase_memory_file called with path: {}, memory_file: {:?}",
        path,
        memory_file
    );

    let expanded_path = expand_home_dir(&path);
    let normalized_path = expanded_path.canonicalize().ok();

    let settings = state.settings_manager.get().await;
    let mut updated_settings = settings.clone();

    // Find and update the codebase in the new format
    let mut found = false;
    for config in &mut updated_settings.codebases {
        let config_expanded = expand_home_dir(&config.path);
        let matches = match (&config_expanded.canonicalize().ok(), &normalized_path) {
            (Some(a), Some(b)) => a == b,
            _ => config.path == path,
        };

        if matches {
            config.memory_file = memory_file.clone();
            found = true;
            break;
        }
    }

    // If not found in new format, check legacy format and migrate
    if !found {
        for legacy_path in &settings.indexed_codebases {
            let legacy_expanded = expand_home_dir(legacy_path);
            let matches = match (&legacy_expanded.canonicalize().ok(), &normalized_path) {
                (Some(a), Some(b)) => a == b,
                _ => legacy_path == &path,
            };

            if matches {
                // Migrate from legacy to new format
                updated_settings.codebases.push(CodebaseConfig {
                    path: legacy_path.clone(),
                    memory_file: memory_file.clone(),
                });
                // Remove from legacy list
                updated_settings
                    .indexed_codebases
                    .retain(|p| p != legacy_path);
                found = true;
                break;
            }
        }
    }

    if !found {
        return Err(format!("Codebase not found: {}", path));
    }

    state
        .settings_manager
        .update(updated_settings)
        .await
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    tracing::info!("Updated memory_file for {}: {:?}", path, memory_file);
    Ok(())
}

/// Detect memory files at the root of a codebase
/// Returns the detected memory file based on priority: AGENTS.md > CLAUDE.md > None
#[tauri::command]
pub async fn detect_memory_files(path: String) -> Result<Option<String>, String> {
    let expanded_path = expand_home_dir(&path);

    if !expanded_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    // Check for AGENTS.md first (higher priority)
    let agents_md = expanded_path.join("AGENTS.md");
    if agents_md.exists() && agents_md.is_file() {
        return Ok(Some("AGENTS.md".to_string()));
    }

    // Check for CLAUDE.md second
    let claude_md = expanded_path.join("CLAUDE.md");
    if claude_md.exists() && claude_md.is_file() {
        return Ok(Some("CLAUDE.md".to_string()));
    }

    // Neither exists
    Ok(None)
}

/// Migrate a codebase's index to the configured storage location
#[tauri::command]
pub async fn migrate_codebase_index(
    path: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    tracing::info!("migrate_codebase_index called with path: {}", path);

    let expanded_path = expand_home_dir(&path);
    let normalized_path = expanded_path
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;

    let settings = state.settings_manager.get().await;
    let target_location = settings.indexer.index_location;

    // Determine current location by checking which exists
    let from_location = if compute_index_dir(&normalized_path, IndexLocation::Local).exists() {
        IndexLocation::Local
    } else if compute_index_dir(&normalized_path, IndexLocation::Global).exists() {
        IndexLocation::Global
    } else {
        return Ok(None); // No existing index
    };

    migrate_index(&normalized_path, from_location, target_location)
        .map(|opt| opt.map(|p| p.to_string_lossy().to_string()))
        .map_err(|e| e.to_string())
}
