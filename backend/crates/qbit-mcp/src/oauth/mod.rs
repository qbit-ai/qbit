//! OAuth 2.1 implementation for MCP server authentication.
//!
//! Supports the full OAuth flow including PKCE, Dynamic Client Registration,
//! metadata discovery, and token persistence.

pub mod callback;
pub mod discovery;
pub mod flow;
pub mod pkce;
pub mod registration;
pub mod token_store;
pub mod types;
