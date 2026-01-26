//! PTY module - re-exports from qbit-pty crate.
//!
//! This module provides a thin wrapper around the qbit-pty infrastructure crate.
//!
//! # Architecture
//!
//! - **qbit-pty**: Infrastructure crate with PTY management system
//! - **qbit/pty/mod.rs**: Re-exports for compatibility

// Re-export everything from qbit-pty
pub use qbit_pty::*;
