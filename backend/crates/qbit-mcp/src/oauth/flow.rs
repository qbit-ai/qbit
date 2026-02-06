//! Main OAuth 2.1 flow orchestration.

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use std::time::Duration;
use uuid::Uuid;

use super::callback::start_callback_server;
use super::discovery::{
    discover_authorization_server, discover_protected_resource, parse_www_authenticate,
};
use super::pkce::{generate_challenge, generate_verifier};
use super::registration::register_client;
use super::token_store::{is_token_expired, load_token, save_token, server_key_from_url};
use super::types::{OAuthServerConfig, StoredTokenData, TokenResponse};

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(120);

/// Ensure we have a valid access token for the given server.
///
/// This is the main entry point for OAuth authentication.
/// It handles token loading, refresh, and full OAuth flow as needed.
pub async fn ensure_access_token(
    server_name: &str,
    server_url: &str,
    oauth_config: Option<&OAuthServerConfig>,
) -> Result<String> {
    let server_key = server_key_from_url(server_url);

    // Try to load existing token
    if let Ok(stored) = load_token(&server_key).await {
        if !is_token_expired(&stored) {
            tracing::debug!("Using cached access token for {}", server_name);
            return Ok(stored.access_token);
        }

        // Try to refresh
        if stored.refresh_token.is_some() {
            tracing::info!(
                "Access token expired, attempting refresh for {}",
                server_name
            );
            match refresh_token(&server_key, &stored).await {
                Ok(token_response) => {
                    let updated = update_stored_token(stored, token_response);
                    save_token(&server_key, &updated).await?;
                    return Ok(updated.access_token);
                }
                Err(e) => {
                    tracing::warn!("Token refresh failed: {}, will perform full OAuth flow", e);
                }
            }
        }
    }

    // Perform full OAuth flow
    tracing::info!("Performing OAuth authentication for {}", server_name);
    perform_oauth_flow(server_name, server_url, oauth_config, &server_key).await
}

/// Perform the complete OAuth 2.1 authorization code flow with PKCE.
async fn perform_oauth_flow(
    server_name: &str,
    server_url: &str,
    oauth_config: Option<&OAuthServerConfig>,
    server_key: &str,
) -> Result<String> {
    let client = Client::new();

    // Step 1: Probe the server to get a 401 response with WWW-Authenticate header
    tracing::debug!("Probing {} for OAuth metadata", server_url);
    let probe_response = client
        .get(server_url)
        .send()
        .await
        .context("Failed to probe server")?;

    if probe_response.status() != reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!(
            "Expected 401 Unauthorized from server, got {}",
            probe_response.status()
        ));
    }

    // Step 2: Parse WWW-Authenticate header
    let www_auth = probe_response
        .headers()
        .get("www-authenticate")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow!("Missing WWW-Authenticate header"))?;

    let (resource_metadata_url, suggested_scopes) = parse_www_authenticate(www_auth);

    // Step 3: Discover authorization server metadata through multiple strategies
    let client_for_metadata = Client::new();
    let mut pr_metadata: Option<super::types::ProtectedResourceMetadata> = None;

    // Strategy A: Use resource_metadata URL from WWW-Authenticate header if present
    if let Some(ref metadata_url) = resource_metadata_url {
        tracing::debug!(
            "Attempting to fetch protected resource metadata from header URL: {}",
            metadata_url
        );
        match client_for_metadata.get(metadata_url).send().await {
            Ok(response) if response.status().is_success() => match response.json().await {
                Ok(metadata) => {
                    tracing::debug!(
                        "Successfully fetched protected resource metadata from header URL"
                    );
                    pr_metadata = Some(metadata);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse protected resource metadata from header URL: {}",
                        e
                    );
                }
            },
            Ok(response) => {
                tracing::warn!(
                    "Protected resource metadata request failed with status: {}",
                    response.status()
                );
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch protected resource metadata from header URL: {}",
                    e
                );
            }
        }
    }

    // Strategy B: Try /.well-known/oauth-protected-resource on server base URL
    if pr_metadata.is_none() {
        tracing::debug!("Attempting to discover protected resource metadata from server base URL");
        match discover_protected_resource(&client_for_metadata, server_url).await {
            Ok(metadata) => {
                tracing::debug!(
                    "Successfully discovered protected resource metadata from server base URL"
                );
                pr_metadata = Some(metadata);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to discover protected resource metadata from server base URL: {}",
                    e
                );
            }
        }
    }

    // Step 4: Determine authorization server URL and discover its metadata
    let as_metadata = if let Some(ref metadata) = pr_metadata {
        // Use authorization server URL from protected resource metadata
        let auth_server_url = metadata
            .authorization_servers
            .as_ref()
            .and_then(|servers| servers.first())
            .ok_or_else(|| {
                anyhow!("No authorization server specified in protected resource metadata")
            })?;

        tracing::debug!(
            "Using authorization server URL from protected resource metadata: {}",
            auth_server_url
        );
        discover_authorization_server(&client, auth_server_url).await?
    } else {
        // Strategy C: Try /.well-known/oauth-authorization-server directly on server base URL
        tracing::debug!("No protected resource metadata available, attempting direct discovery of authorization server");
        discover_authorization_server(&client, server_url).await
            .context("Failed to discover authorization server metadata. Server does not provide resource_metadata in WWW-Authenticate header, does not serve /.well-known/oauth-protected-resource, and does not serve /.well-known/oauth-authorization-server")?
    };

    // Step 6: Start callback server (before DCR so we know the real redirect URI)
    let (callback_port, callback_rx) = start_callback_server().await?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", callback_port);

    // Step 7: Determine client credentials
    let (client_id, client_secret) = match oauth_config {
        Some(cfg) if cfg.client_id.is_some() => {
            tracing::debug!("Using pre-configured client_id");
            (cfg.client_id.clone().unwrap(), cfg.client_secret.clone())
        }
        _ => {
            // Perform Dynamic Client Registration
            let registration_endpoint =
                as_metadata.registration_endpoint.as_ref().ok_or_else(|| {
                    anyhow!("No registration endpoint available and no client_id configured")
                })?;

            let registration =
                register_client(&client, registration_endpoint, &redirect_uri).await?;

            (registration.client_id, registration.client_secret)
        }
    };

    // Step 8: Generate PKCE parameters
    let code_verifier = generate_verifier();
    let code_challenge = generate_challenge(&code_verifier);
    let state = Uuid::new_v4().to_string();

    // Step 9: Determine scopes
    let scopes = oauth_config
        .and_then(|cfg| cfg.scopes.as_ref())
        .or(suggested_scopes.as_ref())
        .or_else(|| {
            pr_metadata
                .as_ref()
                .and_then(|m| m.scopes_supported.as_ref())
        })
        .cloned();

    // Step 10: Build authorization URL
    let mut auth_url = url::Url::parse(&as_metadata.authorization_endpoint)
        .context("Invalid authorization endpoint URL")?;

    {
        let mut query = auth_url.query_pairs_mut();
        query.append_pair("response_type", "code");
        query.append_pair("client_id", &client_id);
        query.append_pair("redirect_uri", &redirect_uri);
        query.append_pair("state", &state);
        query.append_pair("code_challenge", &code_challenge);
        query.append_pair("code_challenge_method", "S256");

        if let Some(ref scope_list) = scopes {
            let scope_str = scope_list.join(" ");
            query.append_pair("scope", &scope_str);
        }
    }

    let auth_url_str = auth_url.to_string();

    // Step 11: Open browser
    tracing::info!("Opening browser for OAuth authorization...");
    tracing::info!("If the browser does not open, visit: {}", auth_url_str);

    if let Err(e) = open::that(&auth_url_str) {
        tracing::warn!("Failed to open browser: {}", e);
        println!(
            "\nPlease open this URL in your browser:\n{}\n",
            auth_url_str
        );
    }

    // Step 12: Wait for callback
    tracing::info!(
        "Waiting for OAuth callback (timeout: {}s)...",
        CALLBACK_TIMEOUT.as_secs()
    );

    let callback_result = tokio::time::timeout(CALLBACK_TIMEOUT, callback_rx)
        .await
        .context("Timeout waiting for OAuth callback")?
        .context("Callback channel closed unexpectedly")??;

    // Step 13: Verify state
    if callback_result.state != state {
        return Err(anyhow!("State mismatch in OAuth callback"));
    }

    // Step 14: Exchange authorization code for tokens
    tracing::debug!("Exchanging authorization code for tokens");

    let mut token_params = vec![
        ("grant_type", "authorization_code"),
        ("code", &callback_result.code),
        ("redirect_uri", &redirect_uri),
        ("code_verifier", &code_verifier),
        ("client_id", &client_id),
    ];

    let client_secret_str;
    if let Some(ref secret) = client_secret {
        client_secret_str = secret.clone();
        token_params.push(("client_secret", &client_secret_str));
    }

    let token_response = client
        .post(&as_metadata.token_endpoint)
        .form(&token_params)
        .send()
        .await
        .context("Failed to exchange authorization code")?;

    if !token_response.status().is_success() {
        let status = token_response.status();
        let body = token_response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Token exchange failed with status {}: {}",
            status,
            body
        ));
    }

    let token_data: TokenResponse = token_response
        .json()
        .await
        .context("Failed to parse token response")?;

    // Step 15: Calculate expiry timestamp
    let expires_at = token_data
        .expires_in
        .map(|seconds| chrono::Utc::now().timestamp() + seconds as i64);

    // Step 16: Persist tokens
    let stored = StoredTokenData {
        access_token: token_data.access_token.clone(),
        refresh_token: token_data.refresh_token.clone(),
        expires_at,
        client_id,
        client_secret,
        token_endpoint: as_metadata.token_endpoint.clone(),
        scopes,
    };

    save_token(server_key, &stored).await?;

    tracing::info!("OAuth authentication successful for {}", server_name);

    Ok(token_data.access_token)
}

/// Refresh an access token using a refresh token.
pub async fn refresh_token(server_key: &str, stored: &StoredTokenData) -> Result<TokenResponse> {
    let refresh_token = stored
        .refresh_token
        .as_ref()
        .ok_or_else(|| anyhow!("No refresh token available"))?;

    tracing::debug!("Refreshing access token for {}", server_key);

    let client = Client::new();

    let mut params = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
        ("client_id", &stored.client_id),
    ];

    let client_secret_str;
    if let Some(ref secret) = stored.client_secret {
        client_secret_str = secret.clone();
        params.push(("client_secret", &client_secret_str));
    }

    let response = client
        .post(&stored.token_endpoint)
        .form(&params)
        .send()
        .await
        .context("Failed to send token refresh request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Token refresh failed with status {}: {}",
            status,
            body
        ));
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .context("Failed to parse token refresh response")?;

    tracing::info!("Successfully refreshed access token for {}", server_key);

    Ok(token_response)
}

fn update_stored_token(
    mut stored: StoredTokenData,
    token_response: TokenResponse,
) -> StoredTokenData {
    stored.access_token = token_response.access_token;

    if let Some(refresh) = token_response.refresh_token {
        stored.refresh_token = Some(refresh);
    }

    stored.expires_at = token_response
        .expires_in
        .map(|seconds| chrono::Utc::now().timestamp() + seconds as i64);

    stored
}
