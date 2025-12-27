//! Settings module - re-exports from qbit-settings crate.
//!
//! This module provides a thin wrapper around the qbit-settings infrastructure crate,
//! adding Tauri-specific commands for the GUI application.
//!
//! # Architecture
//!
//! - **qbit-settings**: Infrastructure crate with core settings logic
//! - **qbit/settings/commands.rs**: Tauri commands (stays in main crate to avoid AppState circular dependency)
//! - **qbit/settings/mod.rs**: Re-exports and command registration

#[cfg(feature = "tauri")]
pub mod commands;

// Re-export everything from qbit-settings
pub use qbit_settings::*;

// Re-export commands for Tauri
#[cfg(feature = "tauri")]
pub use commands::*;
