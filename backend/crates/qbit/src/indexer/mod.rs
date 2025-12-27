//! Code indexer module - re-exports from qbit-indexer crate.
//!
//! This module provides a thin wrapper around the qbit-indexer infrastructure crate.
//!
//! # Architecture
//!
//! - **qbit-indexer**: Infrastructure crate with indexer state management
//! - **qbit/indexer/mod.rs**: Re-exports + Tauri commands

// Tauri commands (stay in main crate due to AppState dependency)
#[cfg(feature = "tauri")]
pub mod commands;

// Re-export everything from qbit-indexer
pub use qbit_indexer::*;

// Re-export commands for Tauri
#[cfg(feature = "tauri")]
pub use commands::*;
