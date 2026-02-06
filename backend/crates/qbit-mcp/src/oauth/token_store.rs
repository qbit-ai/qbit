//! Token persistence to disk.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;

use super::types::StoredTokenData;

/// Get the OAuth authentication directory: ~/.qbit/mcp-auth/
pub fn auth_dir() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    Ok(home.join(".qbit").join("mcp-auth"))
}

/// Get the token file path for a given server key.
pub fn token_path(server_key: &str) -> Result<PathBuf> {
    let dir = auth_dir()?;
    Ok(dir.join(format!("{}.json", server_key)))
}

/// Generate a filesystem-safe server key from a URL.
pub fn server_key_from_url(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();

    let hostname = parsed
        .as_ref()
        .and_then(|u| u.host_str())
        .unwrap_or("unknown");

    // Sanitize hostname for filesystem
    hostname
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Load stored token data from disk.
pub async fn load_token(server_key: &str) -> Result<StoredTokenData> {
    let path = token_path(server_key)?;

    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read token file: {}", path.display()))?;

    let data = serde_json::from_str(&content).context("Failed to parse stored token data")?;

    tracing::debug!("Loaded token for server key: {}", server_key);

    Ok(data)
}

/// Save token data to disk.
pub async fn save_token(server_key: &str, data: &StoredTokenData) -> Result<()> {
    let dir = auth_dir()?;
    fs::create_dir_all(&dir)
        .await
        .context("Failed to create auth directory")?;

    let path = token_path(server_key)?;
    let json = serde_json::to_string_pretty(data).context("Failed to serialize token data")?;

    fs::write(&path, json)
        .await
        .with_context(|| format!("Failed to write token file: {}", path.display()))?;

    tracing::info!("Saved token for server key: {}", server_key);

    Ok(())
}

/// Delete stored token.
pub async fn delete_token(server_key: &str) -> Result<()> {
    let path = token_path(server_key)?;

    if path.exists() {
        fs::remove_file(&path)
            .await
            .with_context(|| format!("Failed to delete token file: {}", path.display()))?;

        tracing::info!("Deleted token for server key: {}", server_key);
    }

    Ok(())
}

/// Check if a token is expired (with 60 second buffer).
pub fn is_token_expired(data: &StoredTokenData) -> bool {
    if let Some(expires_at) = data.expires_at {
        let now = chrono::Utc::now().timestamp();
        let buffer = 60; // 60 second buffer
        expires_at <= (now + buffer)
    } else {
        false // If no expiry, assume it's valid
    }
}
