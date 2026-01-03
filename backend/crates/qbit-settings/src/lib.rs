//! Centralized TOML-based settings system for Qbit.
//!
//! This crate provides configuration management for the Qbit application, including:
//! - Loading settings from `~/.qbit/settings.toml`
//! - Environment variable interpolation (`$VAR` and `${VAR}` syntax)
//! - Atomic file writes with temp file + rename
//! - First-run template generation
//! - Type-safe settings schema with serde defaults
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate in the Qbit architecture:
//! - Depends on: external crates only (serde, toml, tokio, etc.)
//! - Used by: qbit (main application), qbit-tools, qbit-ai, etc.
//!
//! # Usage
//!
//! ```rust,ignore
//! use qbit_settings::{SettingsManager, QbitSettings, get_with_env_fallback};
//!
//! // Load settings
//! let manager = SettingsManager::new().await?;
//! let settings = manager.get().await;
//!
//! // Get a value with environment variable fallback
//! let api_key = get_with_env_fallback(
//!     &settings.api_keys.tavily,
//!     &["TAVILY_API_KEY"],
//!     None,
//! );
//! ```
//!
//! # Environment Variable Interpolation
//!
//! Settings values can reference environment variables:
//!
//! ```toml
//! [ai.vertex_ai]
//! credentials_path = "$GOOGLE_APPLICATION_CREDENTIALS"
//! project_id = "${VERTEX_AI_PROJECT_ID}"
//! ```
//!
//! Both `$VAR` and `${VAR}` syntax are supported.

pub mod loader;
pub mod schema;

// Re-export commonly used items
pub use loader::{get_with_env_fallback, settings_path, SettingsManager};
pub use schema::{LangSmithSettings, QbitSettings, TelemetrySettings, WindowSettings};
