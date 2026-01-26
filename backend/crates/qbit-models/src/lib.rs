//! Model registry and definitions for Qbit LLM providers.
//!
//! This crate provides a centralized registry of model definitions and capabilities,
//! replacing string-matching heuristics with explicit model metadata.
//!
//! # Architecture
//!
//! This is a **Layer 2 (Infrastructure)** crate:
//! - Depends on: qbit-settings (for AiProvider enum)
//! - Used by: qbit-llm-providers, qbit-ai
//!
//! # Model Sources
//!
//! Models can come from two sources:
//! - **Static**: Pre-defined models in the `MODEL_REGISTRY`
//! - **Dynamic**: Runtime-discovered models (e.g., Ollama's `/api/tags`)
//!
//! # Example
//!
//! ```
//! use qbit_models::{get_model, get_models_for_provider, ModelCapabilities, AiProvider};
//!
//! // Look up a specific model
//! if let Some(model) = get_model("gpt-5.2") {
//!     println!("Model: {}", model.display_name);
//!     println!("Supports temperature: {}", model.capabilities.supports_temperature);
//! }
//!
//! // Get all models for a provider
//! let anthropic_models = get_models_for_provider(AiProvider::Anthropic);
//! for model in anthropic_models {
//!     println!("- {} ({})", model.display_name, model.id);
//! }
//! ```

mod capabilities;
mod providers;
mod registry;

pub use capabilities::*;
pub use providers::*;
pub use registry::*;

// Re-export AiProvider for convenience
pub use qbit_settings::schema::AiProvider;
