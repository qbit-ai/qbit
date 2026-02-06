//! Core OAuth types and data structures.

use serde::{Deserialize, Serialize};

/// OAuth configuration that can be specified in mcp.json per-server.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OAuthServerConfig {
    /// Pre-registered client ID (skip DCR if provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,

    /// Pre-registered client secret
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    /// Requested OAuth scopes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

/// Protected Resource Metadata (RFC 9728).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_servers: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
}

/// Authorization Server Metadata (RFC 8414).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_endpoint: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_types_supported: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_challenge_methods_supported: Option<Vec<String>>,
}

/// Token response from the token endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Persisted token data (written to disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<i64>, // unix timestamp
    pub client_id: String,
    pub client_secret: Option<String>,
    pub token_endpoint: String,
    pub scopes: Option<Vec<String>>,
}

/// DCR registration response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id_issued_at: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret_expires_at: Option<u64>,
}
