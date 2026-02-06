//! Dynamic Client Registration (RFC 7591).

use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::json;

use super::types::ClientRegistrationResponse;

/// Register a new OAuth client via Dynamic Client Registration.
pub async fn register_client(
    client: &Client,
    registration_endpoint: &str,
    redirect_uri: &str,
) -> Result<ClientRegistrationResponse> {
    tracing::debug!("Registering OAuth client at {}", registration_endpoint);

    let registration_request = json!({
        "redirect_uris": [redirect_uri],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "application_type": "native",
        "client_name": "Qbit"
    });

    let response = client
        .post(registration_endpoint)
        .json(&registration_request)
        .send()
        .await
        .context("Failed to send client registration request")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Client registration failed with status {}: {}",
            status,
            body
        ));
    }

    let registration = response
        .json::<ClientRegistrationResponse>()
        .await
        .context("Failed to parse client registration response")?;

    tracing::info!(
        "Successfully registered OAuth client: {}",
        registration.client_id
    );

    Ok(registration)
}
