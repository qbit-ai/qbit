//! OAuth metadata discovery (RFC 8414, RFC 9728).

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use url::Url;

use super::types::{AuthorizationServerMetadata, ProtectedResourceMetadata};

/// Discover Protected Resource Metadata from the server.
pub async fn discover_protected_resource(
    client: &Client,
    server_url: &str,
) -> Result<ProtectedResourceMetadata> {
    let base_url = Url::parse(server_url).context("Failed to parse server URL")?;

    let metadata_url = base_url
        .join("/.well-known/oauth-protected-resource")
        .context("Failed to construct metadata URL")?;

    tracing::debug!(
        "Discovering protected resource metadata at {}",
        metadata_url
    );

    let response = client
        .get(metadata_url.as_str())
        .send()
        .await
        .context("Failed to fetch protected resource metadata")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Protected resource metadata request failed: {}",
            response.status()
        ));
    }

    let metadata = response
        .json::<ProtectedResourceMetadata>()
        .await
        .context("Failed to parse protected resource metadata")?;

    Ok(metadata)
}

/// Discover Authorization Server Metadata.
pub async fn discover_authorization_server(
    client: &Client,
    auth_server_url: &str,
) -> Result<AuthorizationServerMetadata> {
    let base_url =
        Url::parse(auth_server_url).context("Failed to parse authorization server URL")?;

    let metadata_url = base_url
        .join("/.well-known/oauth-authorization-server")
        .context("Failed to construct authorization server metadata URL")?;

    tracing::debug!(
        "Discovering authorization server metadata at {}",
        metadata_url
    );

    let response = client
        .get(metadata_url.as_str())
        .send()
        .await
        .context("Failed to fetch authorization server metadata")?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Authorization server metadata request failed: {}",
            response.status()
        ));
    }

    let metadata = response
        .json::<AuthorizationServerMetadata>()
        .await
        .context("Failed to parse authorization server metadata")?;

    Ok(metadata)
}

/// Parse WWW-Authenticate header to extract resource_metadata URL and optional scopes.
///
/// Expected format: `Bearer resource_metadata="https://...", scope="mcp:read mcp:write"`
pub fn parse_www_authenticate(header_value: &str) -> (Option<String>, Option<Vec<String>>) {
    if !header_value.starts_with("Bearer ") {
        return (None, None);
    }

    let params = &header_value[7..]; // Skip "Bearer "

    let mut resource_metadata = None;
    let mut scopes = None;

    for part in params.split(',') {
        let part = part.trim();

        if let Some(value) = part.strip_prefix("resource_metadata=\"") {
            if let Some(end_idx) = value.find('"') {
                resource_metadata = Some(value[..end_idx].to_string());
            }
        } else if let Some(value) = part.strip_prefix("scope=\"") {
            if let Some(end_idx) = value.find('"') {
                let scope_str = &value[..end_idx];
                scopes = Some(
                    scope_str
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect(),
                );
            }
        }
    }

    (resource_metadata, scopes)
}
