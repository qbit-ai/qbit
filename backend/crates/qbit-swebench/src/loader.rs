//! Dataset loading and caching for SWE-bench.
//!
//! Downloads the SWE-bench Lite dataset from HuggingFace and caches it locally.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::{debug, info};

use crate::types::SWEBenchInstance;

/// HuggingFace datasets API endpoint for SWE-bench Lite.
const HUGGINGFACE_DATASETS_API: &str = "https://datasets-server.huggingface.co/rows";

/// Dataset identifier on HuggingFace.
const DATASET_ID: &str = "princeton-nlp/SWE-bench_Lite";

/// Local cache directory relative to ~/.qbit
const CACHE_DIR: &str = "benchmarks/swebench/datasets";

/// Cached dataset filename
const CACHE_FILE: &str = "lite.json";

/// Rows per page from HuggingFace API.
const ROWS_PER_PAGE: usize = 100;

/// Response from HuggingFace datasets API.
#[derive(Debug, Deserialize)]
struct HuggingFaceResponse {
    rows: Vec<HuggingFaceRow>,
    num_rows_total: usize,
    #[serde(default)]
    #[allow(dead_code)]
    num_rows_per_page: usize,
}

/// A single row from the HuggingFace response.
#[derive(Debug, Deserialize)]
struct HuggingFaceRow {
    #[allow(dead_code)]
    row_idx: usize,
    row: SWEBenchInstance,
}

/// Loader for SWE-bench datasets.
pub struct DatasetLoader {
    /// Path to the cache directory
    cache_dir: PathBuf,
    /// HTTP client for downloads
    client: reqwest::Client,
}

impl DatasetLoader {
    /// Create a new dataset loader with default cache location.
    pub fn new() -> Result<Self> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let cache_dir = home.join(".qbit").join(CACHE_DIR);
        Self::with_cache_dir(cache_dir)
    }

    /// Create a new dataset loader with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self { cache_dir, client })
    }

    /// Get the path to the cached dataset file.
    pub fn cache_path(&self) -> PathBuf {
        self.cache_dir.join(CACHE_FILE)
    }

    /// Check if the dataset is cached locally.
    pub fn is_cached(&self) -> bool {
        self.cache_path().exists()
    }

    /// Load the SWE-bench Lite dataset.
    ///
    /// Downloads from HuggingFace if not cached locally.
    pub async fn load_lite(&self) -> Result<Vec<SWEBenchInstance>> {
        let cache_path = self.cache_path();

        if cache_path.exists() {
            debug!("Loading cached dataset from {}", cache_path.display());
            return self.load_from_file(&cache_path);
        }

        info!("Downloading SWE-bench Lite dataset...");
        self.download_and_cache().await
    }

    /// Download the dataset and cache it locally.
    async fn download_and_cache(&self) -> Result<Vec<SWEBenchInstance>> {
        let cache_path = self.cache_path();
        let mut all_instances = Vec::new();
        let mut offset = 0;

        loop {
            let url = format!(
                "{}?dataset={}&config=default&split=test&offset={}&length={}",
                HUGGINGFACE_DATASETS_API, DATASET_ID, offset, ROWS_PER_PAGE
            );

            debug!(
                "Fetching {} instances from offset {}",
                ROWS_PER_PAGE, offset
            );

            let response = self
                .client
                .get(&url)
                .send()
                .await
                .context("Failed to download SWE-bench Lite dataset")?;

            if !response.status().is_success() {
                anyhow::bail!("Failed to download dataset: HTTP {}", response.status());
            }

            let hf_response: HuggingFaceResponse = response
                .json()
                .await
                .context("Failed to parse HuggingFace API response")?;

            let num_rows = hf_response.rows.len();
            let total_rows = hf_response.num_rows_total;

            // Extract instances from the response
            for row in hf_response.rows {
                all_instances.push(row.row);
            }

            info!(
                "Downloaded {} instances (total: {}/{})",
                num_rows,
                all_instances.len(),
                total_rows
            );

            // Check if we've fetched all rows
            if all_instances.len() >= total_rows || num_rows == 0 {
                break;
            }

            offset += num_rows;
        }

        // Cache the dataset as JSON array
        let content = serde_json::to_string_pretty(&all_instances)
            .context("Failed to serialize instances to JSON")?;

        std::fs::write(&cache_path, &content)
            .with_context(|| format!("Failed to cache dataset to {}", cache_path.display()))?;

        info!(
            "Downloaded and cached {} instances to {}",
            all_instances.len(),
            cache_path.display()
        );

        Ok(all_instances)
    }

    /// Load instances from a local file.
    fn load_from_file(&self, path: &Path) -> Result<Vec<SWEBenchInstance>> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;

        // Try JSON array first
        if let Ok(instances) = serde_json::from_str::<Vec<SWEBenchInstance>>(&content) {
            return Ok(instances);
        }

        // Try JSONL format
        let instances: Vec<SWEBenchInstance> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
            .filter_map(|(i, line)| {
                serde_json::from_str(line)
                    .map_err(|e| {
                        tracing::warn!("Failed to parse line {}: {}", i + 1, e);
                        e
                    })
                    .ok()
            })
            .collect();

        if instances.is_empty() {
            anyhow::bail!("No valid instances found in {}", path.display());
        }

        Ok(instances)
    }

    /// Load a specific instance by ID.
    pub async fn load_instance(&self, instance_id: &str) -> Result<SWEBenchInstance> {
        let instances = self.load_lite().await?;
        instances
            .into_iter()
            .find(|i| i.instance_id == instance_id)
            .with_context(|| format!("Instance not found: {}", instance_id))
    }

    /// Load instances matching a filter.
    pub async fn load_filtered<F>(&self, filter: F) -> Result<Vec<SWEBenchInstance>>
    where
        F: Fn(&SWEBenchInstance) -> bool,
    {
        let instances = self.load_lite().await?;
        Ok(instances.into_iter().filter(filter).collect())
    }

    /// Load instances by repository.
    pub async fn load_by_repo(&self, repo: &str) -> Result<Vec<SWEBenchInstance>> {
        self.load_filtered(|i| i.repo == repo).await
    }

    /// List all unique repositories in the dataset.
    pub async fn list_repos(&self) -> Result<Vec<String>> {
        let instances = self.load_lite().await?;
        let mut repos: Vec<String> = instances.iter().map(|i| i.repo.clone()).collect();
        repos.sort();
        repos.dedup();
        Ok(repos)
    }

    /// Get dataset statistics.
    pub async fn stats(&self) -> Result<DatasetStats> {
        let instances = self.load_lite().await?;

        let mut repos = std::collections::HashMap::new();
        for instance in &instances {
            *repos.entry(instance.repo.clone()).or_insert(0) += 1;
        }

        Ok(DatasetStats {
            total_instances: instances.len(),
            repos: repos.into_iter().collect(),
        })
    }
}

impl Default for DatasetLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create default DatasetLoader")
    }
}

/// Statistics about the loaded dataset.
#[derive(Debug, Clone)]
pub struct DatasetStats {
    /// Total number of instances
    pub total_instances: usize,
    /// Instances per repository
    pub repos: Vec<(String, usize)>,
}

impl DatasetStats {
    /// Print a summary of the dataset.
    pub fn print_summary(&self) {
        println!("SWE-bench Lite Dataset");
        println!("======================");
        println!("Total instances: {}", self.total_instances);
        println!();
        println!("Instances by repository:");

        let mut repos = self.repos.clone();
        repos.sort_by(|a, b| b.1.cmp(&a.1));

        for (repo, count) in repos {
            println!("  {}: {}", repo, count);
        }
    }
}

/// Parse a problem range string into a set of indices or instance IDs.
///
/// Supports:
/// - Single indices: "5" -> {5}
/// - Ranges: "0-10" -> {0, 1, 2, ..., 10}
/// - Comma-separated: "0,5,10" -> {0, 5, 10}
/// - Mixed: "0-5,10,15-20" -> {0, 1, 2, 3, 4, 5, 10, 15, 16, 17, 18, 19, 20}
/// - Instance IDs: "django__django-11133" -> specific instance
pub fn parse_instance_filter(filter: &str) -> InstanceFilter {
    // Check if it looks like an instance ID (contains __)
    if filter.contains("__") {
        return InstanceFilter::ById(filter.to_string());
    }

    // Check if it's a repo filter (contains /)
    if filter.contains('/') {
        return InstanceFilter::ByRepo(filter.to_string());
    }

    // Parse as numeric range
    let mut indices = std::collections::HashSet::new();
    for part in filter.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let parts: Vec<&str> = part.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    for i in start..=end {
                        indices.insert(i);
                    }
                }
            }
        } else if let Ok(i) = part.parse::<usize>() {
            indices.insert(i);
        }
    }

    InstanceFilter::ByIndex(indices.into_iter().collect())
}

/// Filter for selecting instances.
#[derive(Debug, Clone)]
pub enum InstanceFilter {
    /// Filter by numeric index
    ByIndex(Vec<usize>),
    /// Filter by instance ID
    ById(String),
    /// Filter by repository
    ByRepo(String),
}

impl InstanceFilter {
    /// Apply the filter to a list of instances.
    pub fn apply(&self, instances: Vec<SWEBenchInstance>) -> Vec<SWEBenchInstance> {
        match self {
            InstanceFilter::ByIndex(indices) => instances
                .into_iter()
                .enumerate()
                .filter(|(i, _)| indices.contains(i))
                .map(|(_, instance)| instance)
                .collect(),
            InstanceFilter::ById(id) => instances
                .into_iter()
                .filter(|i| i.instance_id == *id)
                .collect(),
            InstanceFilter::ByRepo(repo) => {
                instances.into_iter().filter(|i| i.repo == *repo).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_index() {
        match parse_instance_filter("5") {
            InstanceFilter::ByIndex(indices) => {
                assert_eq!(indices, vec![5]);
            }
            _ => panic!("Expected ByIndex"),
        }
    }

    #[test]
    fn test_parse_range() {
        match parse_instance_filter("0-3") {
            InstanceFilter::ByIndex(mut indices) => {
                indices.sort();
                assert_eq!(indices, vec![0, 1, 2, 3]);
            }
            _ => panic!("Expected ByIndex"),
        }
    }

    #[test]
    fn test_parse_instance_id() {
        match parse_instance_filter("django__django-11133") {
            InstanceFilter::ById(id) => {
                assert_eq!(id, "django__django-11133");
            }
            _ => panic!("Expected ById"),
        }
    }

    #[test]
    fn test_parse_repo() {
        match parse_instance_filter("django/django") {
            InstanceFilter::ByRepo(repo) => {
                assert_eq!(repo, "django/django");
            }
            _ => panic!("Expected ByRepo"),
        }
    }
}
