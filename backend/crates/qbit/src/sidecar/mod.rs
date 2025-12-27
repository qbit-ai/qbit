//! Sidecar module - re-exports from qbit-sidecar crate.
//!
//! This module provides a thin wrapper around the qbit-sidecar infrastructure crate.
//!
//! # Architecture
//!
//! - **qbit-sidecar**: Infrastructure crate with session management and context capture
//! - **qbit/sidecar/mod.rs**: Re-exports + Tauri commands

// Tauri commands (stay in main crate due to AppState dependency)
#[cfg(feature = "tauri")]
pub mod commands;

// Re-export everything from qbit-sidecar
pub use qbit_sidecar::*;

// Re-export commands for Tauri
#[cfg(feature = "tauri")]
pub use commands::*;
