//! Google Gemini models on Vertex AI provider for rig.
//!
//! This crate provides integration with Google's Gemini models deployed on
//! Google Cloud Vertex AI. It implements rig-core's `CompletionModel` trait.
//!
//! # Example
//!
//! ```rust,no_run
//! use rig_gemini_vertex::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create client using Application Default Credentials
//!     let client = Client::from_env(
//!         "your-project-id",
//!         "us-central1",
//!     ).await?;
//!
//!     // Get a Gemini model
//!     let model = client.completion_model("gemini-2.5-flash");
//!
//!     // Use with rig's agent or completion request builders
//!     Ok(())
//! }
//! ```

mod client;
mod completion;
mod error;
mod streaming;
mod types;

pub use client::Client;
pub use completion::CompletionModel;
pub use error::GeminiVertexError;
pub use types::*;

/// Available Gemini models on Vertex AI
pub mod models {
    // Gemini 3 series (Preview)
    /// Gemini 3 Pro Preview - Most powerful multimodal model
    pub const GEMINI_3_PRO_PREVIEW: &str = "gemini-3-pro-preview";
    /// Gemini 3 Flash Preview - Balanced speed and intelligence
    pub const GEMINI_3_FLASH_PREVIEW: &str = "gemini-3-flash-preview";

    // Gemini 2.5 series (Stable)
    /// Gemini 2.5 Pro - Advanced thinking model
    pub const GEMINI_2_5_PRO: &str = "gemini-2.5-pro";
    /// Gemini 2.5 Flash - Best price-performance
    pub const GEMINI_2_5_FLASH: &str = "gemini-2.5-flash";
    /// Gemini 2.5 Flash-Lite - Fastest and most cost-efficient
    pub const GEMINI_2_5_FLASH_LITE: &str = "gemini-2.5-flash-lite";

    // Gemini 2.0 series (Deprecated March 2026)
    /// Gemini 2.0 Flash - Previous generation workhorse
    pub const GEMINI_2_0_FLASH: &str = "gemini-2.0-flash";
    /// Gemini 2.0 Flash-Lite - Previous generation fast model
    pub const GEMINI_2_0_FLASH_LITE: &str = "gemini-2.0-flash-lite";
}
