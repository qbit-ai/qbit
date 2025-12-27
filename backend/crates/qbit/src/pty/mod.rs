//! PTY module - re-exports from qbit-pty crate.
//!
//! This module provides a thin wrapper around the qbit-pty infrastructure crate.
//!
//! # Architecture
//!
//! - **qbit-pty**: Infrastructure crate with PTY management system
//! - **qbit/pty/mod.rs**: Re-exports for compatibility

// Re-export everything from qbit-pty (feature-gated for tauri or cli)
#[cfg(any(feature = "tauri", feature = "cli"))]
pub use qbit_pty::*;
